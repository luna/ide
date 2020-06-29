//! Code for module-level double representation processing.

use crate::prelude::*;

use crate::double_representation::definition;
use crate::double_representation::definition::DefinitionProvider;

use ast::crumbs::ChildAst;
use ast::known;
use enso_protocol::language_server;



// ==============
// === Errors ===
// ==============

#[allow(missing_docs)]
#[derive(Fail,Clone,Debug)]
#[fail(display="Cannot find method by pointer {:?}.",_0)]
pub struct CannotFindMethod(language_server::MethodPointer);

#[allow(missing_docs)]
#[derive(Copy,Fail,Clone,Debug)]
#[fail(display="Encountered an empty definition ID. They must contain at least one crumb.")]
pub struct EmptyDefinitionId;



// ========================
// === Module Utilities ===
// ========================

/// Looks up graph in the module.
pub fn traverse_for_definition
(ast:&known::Module, id:&definition::Id) -> FallibleResult<definition::DefinitionInfo> {
    Ok(locate(ast, id)?.item)
}

/// Traverses the module's definition tree following the given Id crumbs, looking up the definition.
pub fn locate(ast:&known::Module, id:&definition::Id) -> FallibleResult<definition::ChildDefinition> {
    let mut crumbs_iter = id.crumbs.iter();
    // Not exactly regular - first crumb is a little special, because module is not a definition
    // nor a children.
    let first_crumb = crumbs_iter.next().ok_or(EmptyDefinitionId)?;
    let mut child = ast.def_iter().find_by_name(&first_crumb)?;
    for crumb in crumbs_iter {
        child = definition::resolve_single_name(child,crumb)?;
    }
    Ok(child)
}

/// TODO TODO
/// The module is assumed to be in the file identified by the `method.file` (for the purpose of
/// desugaring implicit extensions methods for modules).
pub fn lookup_method(ast:&known::Module, method:&language_server::MethodPointer) -> FallibleResult<definition::Id> {
    let module_path = model::module::Path::from_file_path(method.file.clone())?;
    let module_method = method.defined_on_type == module_path.module_name();

    for child in ast.def_iter() {
        let child_name : &definition::DefinitionName = &child.name.item;
        let name_matches = child_name.name.item == method.name;
        let type_matches = match child_name.extended_target.as_slice() {
            []         => module_method,
            [typename] => typename.item == method.defined_on_type,
            _          => child_name.explicitly_extends_type(&method.defined_on_type),
        };
        if name_matches && type_matches {
            let id = definition::Id::new_single_crumb(child_name.clone());
            return Ok(id)
        }
    }

    Err(CannotFindMethod(method.clone()).into())
}

impl DefinitionProvider for known::Module {
    fn indent(&self) -> usize { 0 }

    fn scope_kind(&self) -> definition::ScopeKind { definition::ScopeKind::Root }

    fn enumerate_asts<'a>(&'a self) -> Box<dyn Iterator<Item = ChildAst<'a>>+'a> {
        self.ast().direct_children()
    }
}



// ============
// === Test ===
// ============

#[cfg(test)]
mod tests {
    use super::*;
}