#![allow(clippy::ptr_arg)] // workaround for https://github.com/asomers/mockall/issues/58

//! Project controller.
//!
//! Responsible for owning any remote connection clients, and providing controllers for specific
//! files and modules. Expected to live as long as the project remains open in the IDE.
pub mod synchronized;

use crate::prelude::*;

use enso_protocol::binary;
use enso_protocol::language_server;
use mockall::automock;
use parser::Parser;
use uuid::Uuid;



// =============
// === Model ===
// =============

/// The API of the Project Model.
#[automock]
pub trait API:Debug {
    /// Project's name
    fn name(&self) -> ImString;

    /// Get Language Server JSON-RPC Connection for this project.
    fn json_rpc(&self) -> Rc<language_server::Connection>;

    /// Get Language Server binary Connection for this project.
    fn binary_rpc(&self) -> Rc<binary::Connection>;

    /// Get the instance of parser that is set up for this project.
    fn parser(&self) -> Parser;

    /// Get the visualization controller.
    fn visualization(&self) -> &controller::Visualization;

    /// Get the suggestions database.
    fn suggestion_db(&self) -> Rc<model::SuggestionDatabase>;

    /// Returns a model of module opened from file.
    fn module<'a>
    (&'a self, path:crate::model::module::Path) -> BoxFuture<'a,FallibleResult<model::Module>>;

    /// Creates a new execution context with given definition as a root; and registers the context
    /// for receiving update.
    fn create_execution_context<'a>
    (&'a self, root_definition:language_server::MethodPointer)
    -> BoxFuture<'a,FallibleResult<model::ExecutionContext>>;

    /// Set a new project name..
    fn rename_project<'a>(&'a self, name:String) -> BoxFuture<'a,FallibleResult<()>>;

    /// Returns the primary content root id for this project.
    fn content_root_id(&self) -> Uuid {
        self.json_rpc().content_root()
    }

    /// Generates full module's qualified name that includes the leading project name segment.
    fn qualified_module_name(&self, path:&model::module::Path) -> crate::model::module::QualifiedName {
        path.qualified_module_name(self.name().deref())
    }
}

impl Debug for MockAPI {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"Mocked Project Model")
    }
}

/// The general, shared Project Model handle.
pub type Project      = Rc<dyn API>;
/// Project Model which synchronizes all changes with Language Server.
pub type Synchronized = synchronized::Project;

#[cfg(test)]
pub mod test {
    use super::*;

    /// Sets up parser expectation on the mock project.
    pub fn expect_parser(project:&mut MockAPI, parser:&Parser) {
        let parser = parser.clone_ref();
        project.expect_parser().returning_st(move || parser.clone_ref());
    }

    /// Sets up module expectation on the mock project, returning a give module.
    pub fn expect_module(project:&mut MockAPI, module:model::Module) {
        let module_path = module.path().clone_ref();
        project.expect_module()
            .withf_st    (move |path| path == &module_path)
            .returning_st(move |_path| futures::future::ready(Ok(module.clone_ref())).boxed_local());
    }
}
