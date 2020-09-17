//! A Searcher component.
//!
//! This component wraps the plain ListView in some searcher-specific logic, like committing
//! editing, or picking suggestion with Tab.

use crate::prelude::*;

use crate::documentation;

use enso_frp as frp;
use ensogl::application;
use ensogl::application::{Application, shortcut};
use ensogl::display;
use ensogl_gui_list_view as list_view;
use ensogl_gui_list_view::ListView;

pub use ensogl_gui_list_view::entry;
use enso_frp::io::keyboard::Key;
use ensogl::gui::component::Animation;


// =================
// === Constants ===
// =================

/// Width of searcher panel in pixels.
pub const SEARCHER_WIDTH:f32 = 540.0;
/// Height of searcher panel in pixels.
///
/// Because we don't implement clipping yet, the best UX is when searcher height is almost multiple
/// of entry height.
pub const SEARCHER_HEIGHT:f32 = 179.5;

const SUGGESTION_LIST_WIDTH : f32 = 240.0;
const DOCUMENTATION_WIDTH   : f32 = SEARCHER_WIDTH - SUGGESTION_LIST_WIDTH;
const SUGGESTION_LIST_X     : f32 = (SUGGESTION_LIST_WIDTH - SEARCHER_WIDTH) / 2.0;
const DOCUMENTATION_X       : f32 = (SEARCHER_WIDTH - DOCUMENTATION_WIDTH) / 2.0;



// ==============================
// === Documentation Provider ===
// ==============================

pub trait DocumentationProvider : Debug {
    fn get_for_entry(&self, id:list_view::entry::Id) -> Option<String>;
}

impl DocumentationProvider for list_view::entry::EmptyProvider {
    fn get_for_entry(&self, _:list_view::entry::Id) -> Option<String> { None }
}


// === AnyDocumentationProvider ===

#[derive(Clone,CloneRef,Debug,Deref)]
pub struct AnyDocumentationProvider {rc:Rc<dyn DocumentationProvider>}

impl Default for AnyDocumentationProvider {
    fn default() -> Self { list_view::entry::EmptyProvider.into() }
}

impl<T:DocumentationProvider + 'static> From<T> for AnyDocumentationProvider {
    fn from(provider:T) -> Self { Self {rc:Rc::new(provider)} }
}

impl<T:DocumentationProvider + 'static> From<Rc<T>> for AnyDocumentationProvider {
    fn from(provider:Rc<T>) -> Self { Self {rc:provider} }
}


// =============
// === Model ===
// =============

#[derive(Clone,CloneRef,Debug)]
struct Model {
    logger         : Logger,
    display_object : display::object::Instance,
    list           : ListView,
    documentation  : documentation::View,
    doc_provider   : Rc<CloneRefCell<AnyDocumentationProvider>>,
}

impl Model {
    fn new(app:&Application) -> Self {
        let logger         = Logger::new("SearcherView");
        let scene          = app.display.scene();
        let display_object = display::object::Instance::new(&logger);
        let list           = app.new_view::<ListView>();
        let documentation  = documentation::View::new(&scene);
        let doc_provider   = default();
        display_object.add_child(&documentation);
        display_object.add_child(&list);
        list.set_position_x(SUGGESTION_LIST_X);
        documentation.set_position_x(DOCUMENTATION_X);
        Self{logger,display_object,list,documentation,doc_provider}
    }

    fn documentation_for_entry(&self, id:Option<list_view::entry::Id>) -> String {
        let doc     = id.and_then(|id| self.doc_provider.get().get_for_entry(id));
        doc.unwrap_or_default()
    }

    fn set_height(&self, h:f32) {
        self.list.resize(Vector2(SUGGESTION_LIST_WIDTH,h));
        self.documentation.visualization_frp.inputs.set_size.emit(Vector2(DOCUMENTATION_WIDTH,h));
    }
}



// ===========
// === FRP ===
// ===========

ensogl::def_command_api!( Commands
    /// Pick the selected suggestion and add it to the current input.
    pick_suggestion,
);

ensogl_text::define_endpoints! {
    Commands { Commands }
    Input {
        set_suggestions (list_view::entry::AnyModelProvider,AnyDocumentationProvider),
        show            (),
        hide            (),
    }
    Output {
        selected_entry    (Option<entry::Id>),
        picked_entry      (Option<entry::Id>),
        editing_committed (),
        size              (Vector2<f32>),
        is_visible        (bool),
    }
}



// ============
// === View ===
// ============

/// The Searcher Component.
///
/// This component covers only the list of suggestions. The Searcher input is displayed as an
/// additional graph node in edit mode, so we could easily display e.g. connections between selected
/// node and searcher input.
#[allow(missing_docs)]
#[derive(Clone,CloneRef,Debug)]
pub struct View {
    model   : Model,
    pub frp : Frp,
}

impl Deref for View {
    type Target = Frp;
    fn deref(&self) -> &Self::Target { &self.frp }
}

impl View {
    /// Create new component.
    pub fn new(app:&Application) -> Self {
        let model = Model::new(app);
        let frp   = Frp::new_network();
        Self{model,frp}.init()
    }

    /// Initialize the FRP network.
    fn init(self) -> Self {
        self.model.set_height(0.0);
        let network = &self.frp.network;
        let model   = &self.model;
        let frp     = &self.frp;
        let source  = &self.frp.source;

        let height = Animation::<f32>::new(&network);

        frp::extend! { network
            eval frp.set_suggestions (((entries,docs)) {
                model.doc_provider.set(docs.clone_ref());
                model.list.set_entries(entries);
            });
            source.selected_entry <+ model.list.selected_entry;
            source.size           <+ model.list.size;
            source.is_visible     <+ model.list.size.map(|size| size.x * size.y > std::f32::EPSILON);

            eval height.value ((h)  model.set_height(*h));
            eval frp.show     ((()) height.set_target_value(SEARCHER_HEIGHT));
            eval frp.hide     ((()) height.set_target_value(0.0));

            is_selected         <- model.list.selected_entry.map(|e| e.is_some());
            displayed_doc       <- model.list.selected_entry.map(f!((id) model.documentation_for_entry(*id)));
            opt_picked_entry    <- model.list.selected_entry.sample(&frp.pick_suggestion);
            source.picked_entry <+ opt_picked_entry.gate(&is_selected);
            // Order of the two below is important: we want pick the entry first, and then commit
            // editing.
            source.picked_entry      <+ model.list.chosen_entry.gate(&is_selected);
            source.editing_committed <+ model.list.chosen_entry.gate(&is_selected).constant(());

            eval displayed_doc ((data) model.documentation.frp.display_documentation_pure(data));
        };

        self
    }

    pub fn set_suggestions
    (&self, provider:Rc<impl list_view::entry::ModelProvider + DocumentationProvider + 'static>) {
        let entries       : list_view::entry::AnyModelProvider = provider.clone_ref().into();
        let documentation : AnyDocumentationProvider           = provider.into();
        self.frp.set_suggestions(entries,documentation);
    }

    pub fn unset_suggestions(&self) {
        let provider = Rc::new(list_view::entry::EmptyProvider);
        self.set_suggestions(provider);
    }
}
impl display::Object for View {
    fn display_object(&self) -> &display::object::Instance { &self.model.display_object }
}

impl application::command::FrpNetworkProvider for View {
    fn network(&self) -> &frp::Network {
        &self.frp.network
    }
}

impl application::command::CommandApi for View {
    fn command_api_docs() -> Vec<application::command::EndpointDocs> {
        Commands::command_api_docs()
    }

    fn command_api(&self) -> Vec<application::command::CommandEndpoint> {
        self.frp.input.command.command_api()
    }
}

impl application::command::Provider for View {
    fn label() -> &'static str { "Searcher" }
}

impl application::View for View {
    fn new(app: &Application) -> Self { Self::new(app) }
}

impl application::shortcut::DefaultShortcutProvider for View {
    fn default_shortcuts() -> Vec<shortcut::Shortcut> {
        vec!
        [ Self::self_shortcut(shortcut::Action::press   (&[Key::Tab], &[]) , "pick_suggestion"),
        ]
    }
}
