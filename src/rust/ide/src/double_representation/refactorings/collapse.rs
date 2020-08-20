//! Module with logic for node collapsing.
//!
//! See the [`collapse`] function for details.

use crate::prelude::*;

use crate::double_representation::connection::Connection;
use crate::double_representation::connection::Endpoint;
use crate::double_representation::definition::DefinitionInfo;
use crate::double_representation::definition::DefinitionName;
use crate::double_representation::definition;
use crate::double_representation::identifier::Identifier;
use crate::double_representation::node;
use crate::double_representation::node::NodeInfo;
use crate::double_representation::graph::GraphInfo;

use parser::Parser;
use std::collections::BTreeSet;
use ast::crumbs::Located;
use ast::BlockLine;



// TODO the nice doc
pub fn collapse
(graph:&GraphInfo
 , selected_nodes:impl IntoIterator<Item=node::Id>
 , name:DefinitionName, parser:&Parser
) -> FallibleResult<Collapsed> {
    Collapser::new(graph.clone(),selected_nodes,parser.clone_ref())?.collapse(name)
}



/// Result of running node collapse algorithm. Describes update to the refactored definition.
#[derive(Clone,Debug)]
pub struct Collapsed {
    /// New contents of the refactored definition.
    pub updated_definition : DefinitionInfo,
    /// Contents of the new definition that should be placed next to the refactored one.
    pub new_method         : definition::ToAdd,
}

#[derive(Clone,Debug,Default)]
struct ClassifiedConnections {
    inputs  : Vec<Connection>,
    outputs : Option<Connection>,
}

impl ClassifiedConnections {
    fn new(selected_nodes:&HashSet<node::Id>, all_connections:Vec<Connection>) -> ClassifiedConnections {
        let mut inputs  = Vec::new();
        let mut outputs = Vec::new();
        for connection in all_connections {
            let starts_inside = selected_nodes.contains(&connection.source.node);
            let ends_inside   = selected_nodes.contains(&connection.destination.node);
            if !starts_inside && ends_inside {
                inputs.push(connection)
            } else if starts_inside && !ends_inside {
                outputs.push(connection)
            }
        }

        let outputs = if outputs.len() > 1 {
            panic!("Too many outputs"); // TODO TODO TODO
        } else {
            outputs.into_iter().next()
        };
        //let outputs = outputs.into_iter().next();

        Self {inputs,outputs}
    }
}

// TODO
#[derive(Clone,Debug,Display,Fail)]
struct Error;

// TODO
#[derive(Clone,Debug,Display,Fail)]
pub struct NoNodesSelected;

// TODO
#[derive(Clone,Debug,Display,Fail)]
pub struct CannotResolveConnectionEndpoint;

// TODO
#[derive(Clone,Debug,Display,Fail)]
pub struct EndpointIdentifierCannotBeResolved;

fn lookup_node(nodes:&[NodeInfo], id:node::Id) -> Result<&NodeInfo,CannotResolveConnectionEndpoint> {
    nodes.iter().find(|node| node.id() == id).ok_or(CannotResolveConnectionEndpoint)
}

/// Collapser rewrites the refactoring definition line-by-line. This enum describes action to be
/// taken for a given line.
enum LineDisposition {Keep,Remove,Replace(Ast)}

/// Helper type that stores some common data used for collapsing algorithm and implements its logic.
struct Collapser {
    graph              : GraphInfo,
    selected_nodes     : Vec<NodeInfo>,
    selected_nodes_set : HashSet<node::Id>,
    nodes              : Vec<NodeInfo>,
    last_selected      : node::Id,
    connections        : ClassifiedConnections,
    parser             : Parser,
}

impl Collapser {
    /// Does some early pre-processing and gathers common data used in various parts of the
    /// refactoring algorithm.
    pub fn new
    (graph:GraphInfo, selected_nodes:impl IntoIterator<Item=node::Id>, parser:Parser)
    -> FallibleResult<Self> {
        let nodes                 = graph.nodes();
        let lookup_id             = |id| lookup_node(&nodes,id).cloned();
        let selected_nodes:Vec<_> = Result::from_iter(selected_nodes.into_iter().map(lookup_id))?;
        let selected_nodes_set    = selected_nodes.iter().map(|node| node.id()).collect();
        let last_selected         = selected_nodes.iter().last().ok_or(NoNodesSelected)?.id();
        let connections           = graph.connections();
        let connections           = ClassifiedConnections::new(&selected_nodes_set,connections);
        Ok(Collapser {
            graph,
            selected_nodes,
            selected_nodes_set,
            nodes,
            last_selected,
            connections,
            parser,
        })
    }

    pub fn endpoint_to_node(&self, endpoint:&Endpoint) -> FallibleResult<&NodeInfo> {
        let id  = endpoint.node;
        self.nodes.iter().find(|node| node.id() == id).ok_or(CannotResolveConnectionEndpoint.into())
    }

    pub fn endpoint_to_identifier(&self, endpoint:&Endpoint) -> FallibleResult<Identifier> {
        let node = self.endpoint_to_node(endpoint)?;
        let err = EndpointIdentifierCannotBeResolved;
        Identifier::new(node.ast().get_traversing(&endpoint.crumbs)?.clone_ref()).ok_or(err.into())
    }

    /// Check if the given node belongs to the selection (i.e. is extracted into a new method).
    pub fn is_selected(&self, id:node::Id) -> bool {
        // TODO is it worth to keep a separate field with set just for this?
        self.selected_nodes_set.contains(&id)
        //self.selected_nodes.iter().find(|node| node.id() == id).is_some()
    }

    /// Get the extracted function parameter names.
    ///
    /// All identifiers are in the variable form.
    pub fn arguments(&self) -> FallibleResult<BTreeSet<Identifier>> {
        let input_connections = self.connections.inputs.iter();
        Result::from_iter(input_connections.map(|connection| {
            // Here we take always the source of the connection. This is because it must be in a
            // pattern position, just like the function parameter we want to generate.
            self.endpoint_to_identifier(&connection.source)
        }))
    }

    /// Which node from the refactored graph should be replaced with a call to a extracted method.
    ///
    /// This only exists because we care about this node line's position, not its state.
    pub fn node_to_replace(&self) -> node::Id {
        // When possible, try using the node with output value assignment. That should minimalize
        // the side-effects from the refactoring.
        if let Some(output_connection) = &self.connections.outputs {
            output_connection.source.node
        } else {
            self.last_selected
        }
    }

    /// Get Ast of a line that needs to be appended to the extracted nodes' Asts. None if there is
    /// no such need.
    pub fn return_line(&self) -> Option<Ast> {
        let output_connection = self.connections.outputs.as_ref()?;
        self.endpoint_to_identifier(&output_connection.source).ok().map(Into::into)
    }

    /// Generate the description for the new method's definition with the extracted nodes.
    pub fn extracted_definition(&self,name:DefinitionName) -> FallibleResult<definition::ToAdd> {
        let inputs        = self.arguments()?;
        let return_line   = self.return_line();
        let mut selected_nodes_iter  = self.selected_nodes.iter().map(|node| node.ast().clone());
        let body_head                = selected_nodes_iter.next().unwrap();
        let body_tail                = selected_nodes_iter.chain(return_line).map(Some).collect();
        let explicit_parameter_names = inputs.iter().map(|input| input.name().into()).collect();
        Ok(definition::ToAdd {name,explicit_parameter_names,body_head,body_tail})
    }

    pub fn call_to_extracted(&self, extracted:&definition::ToAdd) -> FallibleResult<Ast> {
        let mut target = extracted.name.clone();
        target.extended_target.insert(0,Located::new_root("here".to_string()));
        let base = target.ast(&self.parser)?;
        let args  = extracted.explicit_parameter_names.iter().map(Ast::var);
        let chain = ast::prefix::Chain::new(base,args);
        Ok(chain.into_ast())
    }

    /// Assign to a line from refactored definition one of 3 dispositions:
    /// 1) Lines that are kept intact -- not belonging to selected nodes;
    /// 2) Lines that are extracted and removed -- all selected nodes, except:
    /// 3) Line that introduces output of the extracted function (if present at all) -> its
    ///    expression shall be replaced with a call to the extracted function.
    ///    If there is no usage of the extracted function output, its invocation should be placed
    ///    in place of the last extracted line.
    pub fn rewrite_line
    (&self, line:&BlockLine<Option<Ast>>, extracted_definition:&definition::ToAdd)
    -> FallibleResult<LineDisposition> {
        let replaced_node = self.node_to_replace();
        let has_output    = self.connections.outputs.is_some();
        let mut node_info = match line.elem.as_ref().and_then(NodeInfo::from_line_ast) {
            Some(node_info) => node_info,
            // We leave lines without nodes (blank lines) intact.
            _               => return Ok(LineDisposition::Keep),
        };
        let id = node_info.id();
        if !self.is_selected(id) {
            Ok(LineDisposition::Keep)
        } else if id == replaced_node {
            if !has_output {
                node_info.clear_pattern()
            }
            node_info.set_expression(self.call_to_extracted(&extracted_definition)?);
            Ok(LineDisposition::Replace(node_info.ast().clone_ref()))
        } else {
            Ok(LineDisposition::Remove)
        }
    }

    pub fn collapse(&self,name:DefinitionName) -> FallibleResult<Collapsed> {
        let new_method      = self.extracted_definition(name)?;
        let mut updated_definition = self.graph.source.clone();
        let mut new_lines   = Vec::new();
        for line in updated_definition.block_lines()? {
            match self.rewrite_line(&line,&new_method)? {
                LineDisposition::Keep         => new_lines.push(line),
                LineDisposition::Remove       => {},
                LineDisposition::Replace(ast) => new_lines.push(BlockLine::new(Some(ast)))
            }
        };
        updated_definition.set_block_lines(new_lines)?;
        Ok(Collapsed {new_method,updated_definition})
    }
}



// ============
// === Test ===
// ============

#[cfg(test)]
mod tests {
    use super::*;

    use crate::double_representation::graph;
    use crate::double_representation::module;
    use crate::double_representation::node::NodeInfo;

    struct Case {
        refactored_name     : DefinitionName,
        introduced_name     : DefinitionName,
        initial_method_code : String,
        extracted_lines     : Range<usize>,
        expected_generated  : String,
        expected_refactored : String
    }

    impl Case {
        // fn from_lines(initial_method_lines:&[&str], extracted_lines:Range<usize>, expected_generated_lines:&[&str], expected_refactored_lines:&[&str]) -> Case {
        //     use crate::test::mock::def_from_lines;;
        //     let refactored_name = DefinitionName::new_plain("main");
        //     let introduced_name = DefinitionName::new_plain("func1");
        //     let initial_method_code = def_from_lines(&refactored_name,initial_method_lines);
        //     let expected_generated  = def_from_lines(&introduced_name,expected_generated_lines);
        //     let expected_refactored = def_from_lines(&refactored_name,expected_refactored_lines);
        //     Case {refactored_name,introduced_name,initial_method_code,extracted_lines,
        //         expected_generated,expected_refactored}
        // }

        fn run(&self, parser:&Parser) {
            let ast   = parser.parse_module(&self.initial_method_code, default()).unwrap();
            let main  = module::locate_child(&ast,&self.refactored_name).unwrap();
            let graph = graph::GraphInfo::from_definition(main.item.clone());
            let nodes = graph.nodes();

            let selected_nodes = nodes[self.extracted_lines.clone()].iter().map(NodeInfo::id);
            let new_name       = self.introduced_name.clone();
            let collapsed      = collapse(&graph,selected_nodes,new_name,parser).unwrap();

            let new_method = collapsed.new_method.ast(0,parser).unwrap();
            let placement  = module::Placement::Before(self.refactored_name.clone());
            let new_main   = &collapsed.updated_definition.ast;
            println!("Generated method:\n{}",new_method);
            println!("Updated method:\n{}",new_main);
            let mut module = module::Info{ast};
            module.ast     = module.ast.set(&main.crumb().into(),new_main.ast().clone()).unwrap();
            module.add_method(collapsed.new_method,placement,parser).unwrap();
            println!("Module after refactoring:\n{}",&module.ast);

            assert_eq!(new_method.repr(),self.expected_generated);
            assert_eq!(new_main.repr(),self.expected_refactored);
        }
    }

    #[test] // TODO make wasm_bindgen_test
    fn test_collapse() {
        let parser          = Parser::new_or_panic();
        let introduced_name = DefinitionName::new_plain("custom_new");
        let refactored_name = DefinitionName::new_plain("custom_old");
        let initial_method_code = r"custom_old =
    a = 1
    b = 2
    c = A + B
    d = a + b
    c + 7".to_owned();
        let extracted_lines    = 1..4;
        let expected_generated = r"custom_new a =
    b = 2
    c = A + B
    d = a + b
    c".to_owned();
        let expected_refactored = r"custom_old =
    a = 1
    c = custom_new a
    c + 7".to_owned();

        let mut case = Case {refactored_name,introduced_name,initial_method_code,
            extracted_lines,expected_generated,expected_refactored};
        case.run(&parser);


        // ========================================================================================
        // Check that refactoring a single assignment line:
        // 1) Maintains the assignment and the introduced name for the value in the extracted
        //    method;
        // 2) That invocation appears in the extracted node's place but has no assignment.

        case.extracted_lines = 3..4;
        case.expected_generated = r"custom_new a b =
    d = a + b".to_owned();
        case.expected_refactored = r"custom_old =
    a = 1
    b = 2
    c = A + B
    custom_new a b
    c + 7".to_owned();
        case.run(&parser);

        // ========================================================================================
        // Check that refactoring a single non-assignment line:
        // 1) Maintains the assignment and the introduced name for the value in the extracted
        //    method;
        // 2) That invocation appears in the extracted node's place but has no assignment.

        case.initial_method_code = r"custom_old =
    a = 1
    b = 2
    c = A + B
    a + b
    c + 7".to_owned();
        case.extracted_lines = 3..4;
        case.expected_generated = r"custom_new a b = a + b".to_owned();
        case.expected_refactored = r"custom_old =
    a = 1
    b = 2
    c = A + B
    custom_new a b
    c + 7".to_owned();
        case.run(&parser);
    }
}
