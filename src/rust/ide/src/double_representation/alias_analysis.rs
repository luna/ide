//! Module with alias analysis — allows telling what identifiers are used and introduced by each
//! node in the graph.

use crate::prelude::*;

use crate::double_representation::node::NodeInfo;

use std::borrow::Borrow;
use ast::crumbs::{InfixCrumb, Located};
use ast::crumbs::Crumb;
use crate::double_representation::definition::DefinitionInfo;

#[cfg(test)]
pub mod test_utils;

/// Identifier with its ast crumb location (relative to the node's ast).
pub type LocatedIdentifier = ast::crumbs::Located<NormalizedName>;



// ======================
// === NormalizedName ===
// ======================

/// The identifier name normalized to a lower-case (as the comparisons are case-insensitive).
/// Implements case-insensitive compare with AST.
#[derive(Clone,Debug,Display,Hash,PartialEq,Eq)]
pub struct NormalizedName(String);

impl NormalizedName {
    /// Wraps given string into the normalized name.
    pub fn new(name:impl Str) -> NormalizedName {
        let name = name.as_ref().to_lowercase();
        NormalizedName(name)
    }

    /// If the given AST is an identifier, returns its normalized name.
    pub fn try_from_ast(ast:&Ast) -> Option<NormalizedName> {
        ast::identifier::name(ast).map(NormalizedName::new)
    }
}

/// Tests if Ast is identifier that might reference the same name (case insensitive match).
impl PartialEq<Ast> for NormalizedName {
    fn eq(&self, other:&Ast) -> bool {
        NormalizedName::try_from_ast(other).contains_if(|other_name| {
            other_name == self
        })
    }
}



// =======================
// === IdentifierUsage ===
// =======================

/// Description of how some node is interacting with the graph's scope.
#[derive(Clone,Debug,Default)]
pub struct IdentifierUsage {
    /// Identifiers from the graph's scope that node is using.
    pub introduced : Vec<LocatedIdentifier>,
    /// Identifiers that node introduces into the parent scope.
    pub used       : Vec<LocatedIdentifier>,
}



// ================
// === Analysis ===
// ================

/// Says whether the identifier occurrence introduces it into scope or uses it from scope.
#[allow(missing_docs)]
#[derive(Clone,Copy,Debug,Display,PartialEq)]
pub enum OccurrenceKind { Used, Introduced }

/// If the current context in the AST processor is a pattern context.
#[allow(missing_docs)]
#[derive(Clone,Copy,Debug,Display,PartialEq)]
pub enum Context { NonPattern, Pattern }

/// Represents scope and information about identifiers usage within it.
#[derive(Clone,Debug,Default)]
pub struct Scope {
    #[allow(missing_docs)]
    pub symbols : IdentifierUsage,
}

impl Scope {
    /// Iterates over identifiers that are used in this scope but are not introduced in this scope
    /// i.e. the identifiers that parent scope must provide.
    pub fn used_from_parent(self) -> impl Iterator<Item=LocatedIdentifier> {
        let available = self.symbols.introduced.into_iter().map(|located_name| located_name.item);
        let available : HashSet<NormalizedName> = HashSet::from_iter(available);
        let all_used  = self.symbols.used.into_iter();
        all_used.filter(move |name| !available.contains(&name.item))
    }

    /// Drops the information about nested child scope by:
    /// 1) disregarding any usage of identifiers introduced in the child scope;
    /// 2) propagating all non-shadowed identifier usage from this scope into this scope usage list.
    fn coalesce_child(&mut self, child:Scope) {
        let symbols_to_use = child.used_from_parent();
        self.symbols.used.extend(symbols_to_use);
    }
}

/// Replaces macro matches on `->` operator with its resolved Ast.
fn pretend_that_lambda_is_not_a_macro_at_all(ast:&Ast) -> Option<Ast> {
    let match_ast = ast::known::Match::try_from(ast.clone()).ok()?;
    let lhs       = match_ast.pfx.as_ref()?;
    let segment   = &match_ast.segs.head;
    if ast::opr::is_arrow_opr(&segment.head) {
        println!("Got lambda, replacing with {}!", match_ast.resolved.repr());
        Some(match_ast.resolved.clone())
    } else {
        None
    }
}

/// Traverser AST and analyzes identifier usage.
#[derive(Clone,Debug,Default)]
pub struct AliasAnalyzer {
    /// Root scope for this analyzer.
    root_scope: Scope,
    /// Stack of scopes, shadowing the root one.
    scopes    : Vec<Scope>,
    /// Stack of context. Lack of any context information is considered non-pattern context.
    context   : Vec<Context>,
    /// Current location, relative to the input AST root.
    location  : Vec<ast::crumbs::Crumb>,
}

impl AliasAnalyzer {
    /// Creates a new analyzer.
    pub fn new() -> AliasAnalyzer {
        AliasAnalyzer::default()
    }

    /// Adds items to the target vector, calls the callback `f` then removes the items.
    fn with_items_added<T,Cs,R,F>
    ( &mut self
    , vec   : impl Fn(&mut Self) -> &mut Vec<T>
    , items : Cs
    , f     : F) -> R
    where
      Cs : IntoIterator<Item:Into<T>>,
      F  : FnOnce(&mut Self) -> R {
        let original_count = vec(self).len();
        vec(self).extend(items.into_iter().map(|item| item.into()));
        let ret = f(self);
        vec(self).truncate(original_count);
        ret
    }

    fn in_new_scope(&mut self, f:impl FnOnce(&mut AliasAnalyzer)) {
        let scope = Scope::default();
        self.scopes.push(scope);
        f(self);
        let scope = self.scopes.pop().unwrap();
        self.current_scope_mut().coalesce_child(scope);
    }

    fn in_context(&mut self, context:Context, f:impl FnOnce(&mut AliasAnalyzer)) {
        self.with_items_added(|this| &mut this.context, std::iter::once(context), f);
    }

    fn in_new_location<Cs,F,R>(&mut self, crumbs:Cs, f:F) -> R
    where Cs : IntoIterator<Item:Into<Crumb>>,
           F : FnOnce(&mut AliasAnalyzer) -> R {
        self.with_items_added(|this| &mut this.location, crumbs, f)
    }

    fn in_location<F,R>(&mut self, crumb:impl Into<Crumb>, f:F) -> R
        where F:FnOnce(&mut Self) -> R {
        self.in_new_location(std::iter::once(crumb),f)
    }

    fn in_location_of<T,F,R>(&mut self, located_item:&Located<T>, f:F) -> R
        where F:FnOnce(&mut Self) -> R {
        self.in_new_location(located_item.crumbs.iter().copied(), f)
    }

    fn current_scope_mut(&mut self) -> &mut Scope {
        self.scopes.last_mut().unwrap_or(&mut self.root_scope)
    }

    fn add_identifier(&mut self, kind: OccurrenceKind, identifier:NormalizedName) {
        let identifier  = LocatedIdentifier::new(self.location.clone(), identifier);
        let scope_index = self.scopes.len();
        let symbols     = &mut self.current_scope_mut().symbols;
        let target      = match kind {
            OccurrenceKind::Used       => &mut symbols.used,
            OccurrenceKind::Introduced => &mut symbols.introduced,
        };
        println!("Name {} is {} in scope @{}",identifier.item.0,kind,scope_index);
        target.push(identifier)
    }


    fn is_in_context(&self, context:Context) -> bool {
        self.context.last().unwrap_or(&Context::NonPattern) == &context
    }

    fn is_in_pattern(&self) -> bool {
        self.is_in_context(Context::Pattern)
    }

    fn store_name_occurrence(&mut self, kind: OccurrenceKind, ast:&Ast) -> bool {
        if let Some(name) = NormalizedName::try_from_ast(ast) {
            self.add_identifier(kind,name);
            true
        } else {
            false
        }
    }
    fn store_if_name<T>(&mut self, kind:OccurrenceKind, located:&Located<T>) -> bool
    where for<'a> &'a T : Into<&'a Ast> {
        let ast = (&located.item).into();
        self.in_location_of(located, |this| this.store_name_occurrence(kind, ast))
    }

    fn process_subtree(&mut self, crumb:impl Into<Crumb>, ast:&Ast) {
        self.in_location(crumb.into(), |this| this.process_ast(ast))
    }

    fn process_located_ast(&mut self, located_ast:&Located<impl Borrow<Ast>>) {
        self.in_location_of(&located_ast, |this| this.process_ast(located_ast.item.borrow()))
    }

    fn process_subtrees(&mut self, ast:&Ast) {
        for (crumb,ast) in ast.enumerate() {
            self.process_subtree(crumb,ast)
        }
    }

    fn process_ast(&mut self, ast:&Ast) {
        let ast = pretend_that_lambda_is_not_a_macro_at_all(ast).unwrap_or(ast.clone());
        println!("Processing `{}` in 0context {:?}",ast.repr(),self.context.last());
        if let Some(assignment) = ast::opr::to_assignment(&ast) {
            self.process_assignment(&assignment);
        } else if let Some(lambda) = ast::opr::to_arrow(&ast) {
            self.process_lambda(&lambda);
        } else if self.is_in_pattern() {
            // We are in the pattern (be it a lambda's or assignment's left side). Three options:
            // 1) This is a destructuring pattern match with prefix syntax, like `Point x y`.
            // 3) As above but with operator and infix syntax, like `head,tail`.
            // 2) This is a nullary symbol binding, like `foo`.
            // (the possibility of definition has been already excluded)
            if let Some(prefix_chain) = ast::prefix::Chain::try_new(&ast) {
                println!("Pattern of infix chain of {}",ast.repr());
                // Arguments introduce names, we ignore function name.
                // Arguments will just introduce names in pattern context.
                for argument in prefix_chain.enumerate_args() {
                    self.process_located_ast(&argument)
                }
            } else if let Some(infix_chain) = ast::opr::Chain::try_new(&ast) {
                for operand in infix_chain.enumerate_operands() {
                    self.process_located_ast(operand)
                }
                for operator in infix_chain.enumerate_operators() {
                    // Operators in infix positions are treated as constructors, i.e. they are used.
                    self.store_if_name(OccurrenceKind::Used, operator);
                }
            } else {
                self.store_name_occurrence(OccurrenceKind::Introduced, &ast);
            }
        } else if self.is_in_context(Context::NonPattern) {
            if let Ok(_) = ast::known::Block::try_from(&ast) {
                self.in_new_scope(|this| this.process_subtrees(&ast))
            } else if self.store_name_occurrence(OccurrenceKind::Used, &ast) {
                // Plain identifier: we just added as the condition side-effect.
                // No need to do anything more.
            } else {
                self.process_subtrees(&ast);
            }
        }
    }

    fn process_assignment(&mut self, assignment:&ast::known::Infix) {
        self.in_context(Context::Pattern, |this|
            this.process_subtree(InfixCrumb::LeftOperand, &assignment.larg)
        );
        self.process_subtree(InfixCrumb::RightOperand, &assignment.rarg);
    }

    fn process_lambda(&mut self, lambda:&ast::known::Infix) {
        self.in_new_scope(|this| {
            this.in_context(Context::Pattern, |this|
                this.process_subtree(InfixCrumb::LeftOperand, &lambda.larg)
            );
            this.process_subtree(InfixCrumb::RightOperand, &lambda.rarg);
        })
    }

    fn process_node(&mut self, node:&NodeInfo) {
        self.process_ast(node.ast())
    }
}

/// Describes identifiers that nodes introduces into the graph and identifiers from graph's scope
/// that node uses. This logic serves as a base for connection discovery.
pub fn analyse_identifier_usage(node:&NodeInfo) -> IdentifierUsage {
    println!("\n===============================================================================\n");
    println!("Case: {}",node.ast().repr());
    let mut analyzer = AliasAnalyzer::new();
    analyzer.process_node(node);
    analyzer.root_scope.symbols
}



// =============
// === Tests ===
// =============

#[cfg(test)]
mod tests {
    use super::*;
    use super::test_utils::*;

//    use wasm_bindgen_test::wasm_bindgen_test;
//    use wasm_bindgen_test::wasm_bindgen_test_configure;
//
//    wasm_bindgen_test_configure!(run_in_browser);

    /// Checks if actual observed sequence of located identifiers matches the expected one.
    /// Expected identifiers are described as code spans in the node's text representation.
    fn validate_identifiers
    (name:impl Str, node:&NodeInfo, expected:Vec<Range<usize>>, actual:&Vec<LocatedIdentifier>) {
        let mut checker = IdentifierValidator::new(name,node,expected);
        checker.validate_identifiers(actual);
    }

    /// Runs the test for the given test case description.
    fn run_case(parser:&parser::Parser, case:Case) {
        let ast    = parser.parse_line(&case.code).unwrap();
        let node   = NodeInfo::from_line_ast(&ast).unwrap();
        let result = analyse_identifier_usage(&node);
        println!("Analysis results: {:?}", result);
        validate_identifiers("introduced",&node, case.expected_introduced, &result.introduced);
        validate_identifiers("used",      &node, case.expected_used,       &result.used);
    }

    /// Runs the test for the test case expressed using markdown notation. See `Case` for details.
    fn run_markdown_case(parser:&parser::Parser, marked_code:impl Str) {
        println!("Running test case for {}", marked_code.as_ref());
        let case = Case::from_markdown(marked_code);
        run_case(parser,case)
    }


    #[test]
    fn test_alias_analysis() {
        let parser = parser::Parser::new_or_panic();

        // Removed cases
//            "«foo» a b = a »+« b",  // this we don't care, because this is not a node
//            "«log_name» object = »print« object.»name«",
//            "«^» a n = a * a ^ (n - 1)",

        let test_cases = vec![
//            "a -> »b«",
            "»foo«",
            "«five» = 5",
            "«foo» = »bar«",
            "«sum» = »a« »+« »b«",
            "Point «x» «u» = »point«",
            "«x» »,« «y» = »pair«",

            r"«inc» =
                »foo« »+« 1",

            r"«inc» =
                foo = 2
                foo »+« 1",

//            "a.«hello» = »print« 'Hello'",
//            "«log_name» = object -> »print« object.»name«",
//            "«log_name» = object -> »print« $ »name« object",
        ];
        for case in test_cases {
            run_markdown_case(&parser,case)
        }


//        let code   = "«sum» = »a« + »b«";
//        run_markdown_case(&parser, code);
    }
}
