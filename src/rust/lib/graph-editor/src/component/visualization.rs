//! This module defines the visualization widgets.
use crate::prelude::*;

use crate::frp;

use ensogl::display::DomSymbol;
use ensogl::display::object::class::Object;
use ensogl::display::object::class::ObjectOps;
use ensogl::display;
use ensogl::system::web;
use web::StyleSetter;


// ============================================
// === Wrapper for Visualisation Input Data ===
// ============================================

/// Wrapper for data that can be consumed by a visualisation.
/// TODO replace with better typed data wrapper.
#[derive(Clone,Debug)]
#[allow(missing_docs)]
pub enum Data {
    JSON { content : String },
    Empty,
}

impl Data {
    /// Render the data as JSON.
    pub fn as_json(&self) -> String {
        match &self {
            Data::JSON { content } => content.clone(),
            Data::Empty => { "{}".to_string() },
        }
    }
}

impl Default for Data{
    fn default() -> Self {
        Data::Empty
    }
}



// =============================================
// === Internal Visualisation Representation ===
// =============================================

/// Content that can be used in a visualization.
#[derive(Clone,CloneRef,Debug)]
#[allow(missing_docs)]
pub enum Visualization {
    Html   { content : Rc<DomSymbol>                 },
    Native { content : Rc<display::object::Instance> },
    Empty,
}

impl Visualization {
    /// Update the visualisation with the given data.
    pub fn update_data(&self, data:Data){
        match &self {
            Visualization::Html { content } => {
                content.dom().set_inner_html(
                    &format!(r#"
<svg>
  <circle style="fill: #69b3a2" stroke="black" cx=50 cy=50 r={}></circle>
</svg>
"#, data.as_json()));
            },
            Visualization::Native { .. } => {},
            Visualization::Empty => {},
        }
    }
}

impl Default for Visualization {
    fn default() -> Self {
        Visualization::Empty
    }
}

impl From<DomSymbol> for Visualization {
    fn from(symbol: DomSymbol) -> Self {
        Visualization::Html { content : Rc::new(symbol) }
    }
}

impl From<Rc<DomSymbol>> for Visualization {
    fn from(symbol: Rc<DomSymbol>) -> Self {
        Visualization::Html { content : symbol }
    }
}



// ============================
// === Visualization Events ===
// ============================

/// Visualization events.
#[derive(Clone,CloneRef,Debug)]
#[allow(missing_docs)]
pub struct Events {
    pub network           : frp::Network,
    pub show              : frp::Source,
    pub hide              : frp::Source,
    pub toggle_visibility : frp::Source,
    pub update_content    : frp::Source<Visualization>,
    pub update_data       : frp::Source<Data>,
}

impl Default for Events {
    fn default() -> Self {
        frp::new_network! { visualization_events
            def show              = source::<()>            ();
            def hide              = source::<()>            ();
            def toggle_visibility = source::<()>            ();
            def update_content    = source::<Visualization> ();
            def update_data       = source::<Data>          ();
        };
        let network = visualization_events;
        Self {network,show,hide,update_content,toggle_visibility,update_data}
    }
}



// ================================
// === Visualizations Container ===
// ================================

/// Container that wraps a `Visualisation` for rendering and interaction in the gui.
#[derive(Clone,CloneRef,Debug)]
#[allow(missing_docs)]
pub struct Container {
    pub data : Rc<ContainerData>
}

/// Weak version of `Visualization`.
#[derive(Clone,CloneRef,Debug)]
pub struct WeakContainer {
    data : Weak<ContainerData>
}

/// Internal data of a `Container`.
#[derive(Debug,Clone)]
#[allow(missing_docs)]
pub struct ContainerData {
    pub logger : Logger,
    pub events : Events,

    node       : display::object::Instance,
    size       : Cell<Vector2<f32>>,
    position   : Cell<Vector3<f32>>,
    visible    : Cell<bool>,

    content   : RefCell<Visualization>,
}

impl Container {
    /// Constructor.
    pub fn new() -> Self {

        let logger   = Logger::new("visualization");
        let events   = Events::default();
        // TODO replace with actual content;
        let content  = RefCell::new(Visualization::default());
        let size     = Cell::new(Vector2::new(100.0, 100.0));
        let position = Cell::new(Vector3::new(  0.0,-110.0, 0.0));
        let visible  = Cell::new(true);
        let node     = display::object::Instance::new(&logger);

        let data     = ContainerData {logger,events,content,size,position,visible,node};
        let data     = Rc::new(data);
        Self {data} . init_frp()
    }

    /// Dummy content for testing.
    // FIXME remove this when actual content is available.
    pub fn default_content() -> DomSymbol {
        let div = web::create_div();
        div.set_style_or_panic("width","100px");
        div.set_style_or_panic("height","100px");
        div.set_style_or_panic("overflow","hidden");

        let content = web::create_element("div");
        content.set_inner_html(
r#"<svg>
  <circle style="fill: #69b3a2" stroke="black" cx=50 cy=50 r=20></circle>
</svg>
"#);
        content.set_attribute("width","100%").unwrap();
        content.set_attribute("height","100%").unwrap();

        div.append_child(&content).unwrap();

        let r          = 102_u8;
        let g          = 153_u8;
        let b          = 194_u8;
        let color      = iformat!("rgb({r},{g},{b})");
        div.set_style_or_panic("background-color",color);

        let symbol = DomSymbol::new(&div);
        symbol.dom().set_attribute("id","vis").unwrap();
        symbol

    }

    /// Update the content properties with the values from the `VisualizationData`.
    ///
    /// Needs to called when those values change or new content has been set.
    fn set_content_properties(&self) {
        let size       = self.data.size.get();
        let position   = self.data.position.get();

        match self.data.content.borrow().deref() {
            Visualization::Html { content } => {
                content.set_size(size);
                content.set_position(position);
            },
            Visualization::Native { content  } => {
                // TODO ensure correct size
                // content.display_object().rc.set_scale(size);
                content.display_object().rc.set_position(position);
            },
            Visualization::Empty => {},
        }
    }

    /// Get the visualization content.
    pub fn content(&self) -> Visualization {
        self.data.content.borrow().clone()
    }

    /// Set the visualization content.
    pub fn set_content(&self, content: Visualization) {
        match &content {
            Visualization::Html { content } => self.display_object().add_child(content.as_ref()),
            Visualization::Native { content }      => self.display_object().add_child(content.as_ref()),
            Visualization::Empty => {},
        }
        self.data.content.replace(content);
        self.set_content_properties();
    }

    fn init_frp(self) -> Self {
        let network = &self.data.events.network;

        frp::extend! { network
            let weak_vis = self.downgrade();
            def _f_show = self.data.events.show.map(move |_| {
               if let Some(vis) = weak_vis.upgrade() {
                    vis.set_visibility(true)
               }
            });

            let weak_vis = self.downgrade();
            def _f_hide = self.data.events.hide.map(move |_| {
                if let Some(vis) = weak_vis.upgrade() {
                    vis.set_visibility(false)
               }
            });

            let weak_vis = self.downgrade();
            def _f_toggle = self.data.events.hide.map(move |_| {
                if let Some(vis) = weak_vis.upgrade() {
                    vis.toggle_visibility()
               }
            });

            let weak_vis = self.downgrade();
            def _f_hide = self.data.events.update_content.map(move |content| {
                if let Some(vis) = weak_vis.upgrade() {
                    vis.set_content(content.clone());
                }
            });

            let weak_vis = self.downgrade();
            def _f_hide = self.data.events.update_data.map(move |data| {
                if let Some(vis) = weak_vis.upgrade() {
                    vis.set_data(data.clone());
                }
            });
        }
        self
    }

    /// Toggle visibility on or off.
    pub fn set_visibility(&self, visible: bool) {
        self.data.visible.set(visible)  ;
        match (self.data.content.borrow().deref(),visible)  {
            (Visualization::Html { content }, true)  => content.dom().set_style_or_panic("visibility", "visible"),
            (Visualization::Html { content }, false) => content.dom().set_style_or_panic("visibility", "hidden"),
            // TODO investigate why this is not working.
            (Visualization::Native { content }, true)  => content.display_object().rc.show(),
            (Visualization::Native { content }, false) => content.display_object().rc.hide(),

            (&Visualization::Empty,_)   => {}
        }
    }

    /// Toggle visibility.
    pub fn toggle_visibility(&self) {
        self.set_visibility(!self.data.visible.get())
    }

    /// Update the data in the inner visualisation.
    pub fn set_data(&self, data: Data) {
        self.data.content.borrow().update_data(data)
    }

}

impl Default for Container {
    fn default() -> Self {
        Container::new()
    }
}

impl StrongRef for Container {
    type WeakRef = WeakContainer;
    fn downgrade(&self) -> WeakContainer {
        WeakContainer {data:Rc::downgrade(&self.data)}
    }
}

impl WeakRef for WeakContainer {
    type StrongRef = Container;
    fn upgrade(&self) -> Option<Container> {
        self.data.upgrade().map(|data| Container {data})
    }
}

impl Object for Container {
    fn display_object(&self) -> &display::object::Instance {
        &self.data.node
    }
}
