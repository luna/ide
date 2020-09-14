//! The main view of single project opened in IDE.

use crate::prelude::*;

use crate::documentation;
use crate::graph_editor::GraphEditor;
use crate::graph_editor::NodeId;
use crate::graph_editor::component::node::Expression;
use crate::graph_editor::component::visualization;
use crate::searcher;

use enso_frp as frp;
use ensogl::application;
use ensogl::application::Application;
use ensogl::application::shortcut;
use ensogl::display;
use ensogl::display::shape::*;
use ensogl_gui_list_view as list_view;
use enso_frp::stream::EventEmitter;


#[derive(Clone,CloneRef,Debug)]
struct Model {
    logger         : Logger,
    display_object : display::object::Instance,
    graph_editor   : GraphEditor,
    searcher       : searcher::View,
    documentation  : documentation::View,
    //TODO[ao] This view should contain also Text Editor; it should be moved here during refactoring
    // planned in task https://github.com/enso-org/ide/issues/597
}

impl Model {
    fn new(app:&Application) -> Self {
        let scene          = app.display.scene();
        let logger         = Logger::new("project::View");
        let display_object = display::object::Instance::new(&logger);
        let graph_editor   = app.new_view::<GraphEditor>();
        let searcher       = app.new_view::<searcher::View>();
        let documentation  = documentation::View::new(&scene);
        display_object.add_child(&graph_editor);
        display_object.add_child(&documentation);
        display_object.remove_child(&documentation);
        display_object.add_child(&searcher);
        display_object.remove_child(&searcher);
        searcher.resize(Vector2(150.0, 150.0)); //TODO
        Self{logger,display_object,graph_editor,searcher,documentation}
    }

    fn set_documentation_visibility(&self, is_visible:bool) {
        if is_visible { self.display_object.remove_child(&self.documentation) }
        else          { self.display_object.add_child(&self.documentation)    }
    }

    fn show_searcher_under_node(&self, node_id:NodeId) {
        if let Some(node) = self.graph_editor.model.nodes.get_cloned_ref(&node_id) {
            if !self.is_searcher_visible() {
                self.display_object.add_child(&self.searcher);
            }
            let x = node.position().x + 100.0;
            let y = node.position().y - 100.0; //TODO
            self.searcher.set_position_xy(Vector2(x,y));
        } else {
            error!(self.logger, "Trying to show searcher under nonexisting node");
        }
    }

    fn hide_searcher(&self) {
        self.searcher.unset_parent();
        self.searcher.set_entries(list_view::entry::AnyModelProvider::default());
    }

    fn is_searcher_visible(&self) -> bool {
        self.searcher.has_parent()
    }

    pub fn is_documentation_visible(&self) -> bool {
        self.documentation.has_parent()
    }

    fn add_node_and_edit(&self) {
        self.graph_editor.frp.inputs.add_node_at_cursor.emit(());
        let created_node_id = self.graph_editor.frp.outputs.node_added.value();
        debug!(self.logger, "New node view id: {created_node_id}");
        self.graph_editor.frp.inputs.set_node_expression.emit(&(created_node_id,Expression::default()));
        self.graph_editor.frp.inputs.edit_node.emit(&created_node_id);

    }
}

// ===========
// === FRP ===
// ===========

ensogl::def_command_api! { Commands
    /// Add new node and start editing it's expression.
    add_new_node,
    /// Abort currently node edit. If it was added node, it will be removed, if the existing node was edited, its old expression will be restored.
    abort_node_editing,
}

impl application::command::CommandApi for View {
    fn command_api_docs() -> Vec<application::command::EndpointDocs> {
        Commands::command_api_docs()
    }

    fn command_api(&self) -> Vec<application::command::CommandEndpoint> {
        self.frp.input.command.command_api()
    }
}

ensogl_text::define_endpoints! {
    Commands { Commands }
    Input {
        // resize           (Vector2<f32>),
        set_documentation_data (visualization::Data),
        set_suggestions        (list_view::entry::AnyModelProvider),
    }
    Output {
        documentation_visible         (bool),
        adding_new_node               (bool),
        edited_node                   (Option<NodeId>),
        old_expression_of_edited_node (Expression),
        editing_aborted               (NodeId),
        editing_committed             (NodeId),
    }
}

/// The main view of single project opened in IDE.
#[allow(missing_docs)]
#[derive(Clone,CloneRef,Debug)]
pub struct View {
    model   : Model,
    pub frp : Frp,
}

impl View {
    /// Constructor.
    pub fn new(app:&Application) -> Self {
        let model    = Model::new(app);
        let frp      = Frp::new_network();
        let searcher = &model.searcher.frp;
        let graph    = &model.graph_editor.frp;

        let network = &frp.network;

        frp::extend!{ TRACE_ALL network
            // === Documentation Set ===

            eval frp.set_documentation_data ((data) model.documentation.frp.send_data.emit(data));
            eval frp.set_suggestions        ((provider) model.searcher.frp.set_entries(provider));


            // === Editing ===

            // The order of instructions below is important to properly distinguish between
            // committing and aborting node editing.

            // This node is false when received "abort_node_editing" signal, and should get true
            // once processing of "edited_node" event from graph is performed.
            editing_aborted <- any(...);
            editing_aborted <+ frp.abort_node_editing.constant(true);
            should_finish_editing <- any(frp.abort_node_editing,searcher.editing_committed);
            eval should_finish_editing ([graph](()) {
                graph.inputs.edit_mode_off.emit(());
            });
            eval graph.outputs.edited_node ([model](edited_node_id) {
                if let Some(id) = edited_node_id {
                    model.show_searcher_under_node(*id);
                } else {
                    model.hide_searcher();
                }
            });
            editing_not_aborted <- editing_aborted.map(|b| !b);
            trace graph.outputs.node_edit_mode_turned_off;
            frp.source.editing_committed <+ graph.outputs.node_edit_mode_turned_off.gate(&editing_not_aborted);
            frp.source.editing_aborted   <+ graph.outputs.node_edit_mode_turned_off.gate(&editing_aborted);
            editing_aborted              <+ graph.outputs.edited_node.constant(false);


            // === Adding New Node ===

            frp.source.adding_new_node <+ frp.add_new_node.constant(true);
            eval frp.add_new_node ((()) model.add_node_and_edit());

            adding_committed           <- frp.editing_committed.gate(&frp.adding_new_node);
            adding_aborted             <- frp.editing_aborted.gate(&frp.adding_new_node);
            frp.source.adding_new_node <+ any(&adding_committed,&adding_aborted).constant(false);
            eval adding_aborted ((node) graph.remove_node.emit(node));


            // === OUTPUTS REBIND ===

            eval frp.documentation_visible ((vis) model.set_documentation_visibility(*vis));
        }

        let mock_documentation = visualization::MockDocGenerator::default();
        let data               = mock_documentation.generate_data();
        let content            = serde_json::to_value(data).unwrap_or_default();
        let data               = visualization::Data::from(content);
        frp.set_documentation_data(data);

        Self{model,frp}
    }

    /// Graph Editor View.
    pub fn graph(&self) -> &GraphEditor {
        &self.model.graph_editor
    }

    /// Searcher View.
    pub fn searcher(&self) -> &searcher::View {
        &self.model.searcher
    }
}

impl display::Object for View {
    fn display_object(&self) -> &display::object::Instance { &self.model.display_object }
}

impl application::command::FrpNetworkProvider for View {
    fn network(&self) -> &frp::Network { &self.frp.network }
}

impl application::command::Provider for View {
    fn label() -> &'static str { "ProjectView" }
}

impl application::View for View {
    fn new(app:&Application) -> Self { View::new(app) }
}

impl application::shortcut::DefaultShortcutProvider for View {
    fn default_shortcuts() -> Vec<application::shortcut::Shortcut> {
        use frp::io::keyboard::Key;
        vec!
        [ Self::self_shortcut(shortcut::Action::press(&[Key::Control,Key::Tab],&[]) , "add_new_node")
        , Self::self_shortcut(shortcut::Action::press(&[Key::Escape],&[])           , "abort_node_editing")
        ]
    }
}
