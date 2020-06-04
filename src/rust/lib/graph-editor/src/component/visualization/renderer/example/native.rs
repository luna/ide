//! Examples of defining visualization in Rust using web_sys or ensogl.

use crate::prelude::*;

use crate::component::visualization::*;
use crate::component::visualization::traits::SymbolWithLayout;
use crate::component::visualization::traits::TargetLayer;

use ensogl::data::color::Rgba;
use ensogl::display::DomSymbol;
use ensogl::display::Symbol;
use ensogl::display::scene::Scene;
use ensogl::display;
use ensogl::gui::component;
use ensogl::system::web;
use ensogl::display::object::ObjectOps;



// ===================
// === BubbleChart ===
// ===================

/// Bubble shape definition.
pub mod shape {
    use super::*;
    use ensogl::display::shape::*;
    use ensogl::display::scene::Scene;
    use ensogl::display::Sprite;
    use ensogl::display::Buffer;
    use ensogl::display::Attribute;

    ensogl::define_shape_system! {
        (position:Vector2<f32>,radius:f32) {
            let node = Circle(radius);
            let node = node.fill(Rgba::new(0.17,0.46,0.15,1.0));
            let node = node.translate(("input_position.x","input_position.y"));
            node.into()
        }
    }
}

/// Sample implementation of a Bubble Chart.
#[derive(Debug)]
#[allow(missing_docs)]
pub struct BubbleChart {
    pub display_object : display::object::Instance,
    pub scene          : Scene,
        frp            : DataRendererFrp,
        views          : RefCell<Vec<component::ShapeView<shape::Shape>>>,
        logger         : Logger,
        size           : Cell<V2>,
}

#[allow(missing_docs)]
impl BubbleChart {
    pub fn new(scene:&Scene) -> Self {
        let logger         = Logger::new("bubble");
        let display_object = display::object::Instance::new(&logger);
        let views          = RefCell::new(vec![]);
        let frp            = default();
        let size           = default();
        let scene          = scene.clone_ref();
        BubbleChart { display_object,views,logger,frp,size,scene }
    }
}

impl DataRenderer for BubbleChart {

    fn receive_data(&self, data:Data) -> Result<(),DataError> {
        let data_inner: Rc<Vec<Vector3<f32>>> = data.as_binary()?;
        // Avoid re-creating views, if we have already created some before.
        let mut views = self.views.borrow_mut();
        views.resize_with(data_inner.len(),|| component::ShapeView::new(&self.logger,&self.scene));

        // TODO[mm] this is somewhat inefficient, as the canvas for each bubble is too large.
        // But this ensures that we can get a cropped view area and avoids an issue with the data
        // and position not matching up.
        views.iter().zip(data_inner.iter()).for_each(|(view,item)| {
            let size : Vector2<f32> = self.size.get().into();
            view.display_object.set_parent(&self.display_object);
            view.shape.sprite.size().set(size);
            view.shape.radius.set(item.z);
            view.shape.position.set(Vector2::new(item.x,item.y) - size / 2.0);
        });
        Ok(())
    }

    fn set_size(&self, size:V2) {
        self.size.set(size);
    }

    fn frp(&self) -> &DataRendererFrp {
        &self.frp
    }
}

impl display::Object for BubbleChart {
    fn display_object(&self) -> &display::object::Instance {
        &self.display_object.display_object()
    }
}



// ===============
// === RawText ===
// ===============

/// Sample visualization that renders the given data as text. Useful for debugging and testing.
#[derive(Debug)]
#[allow(missing_docs)]
pub struct RawText {
    dom    : DomSymbol,
    size   : Cell<V2>,
    frp    : DataRendererFrp,
    logger : Logger,
}

impl RawText {
    /// Constructor.
    pub fn new(scene:&Scene) -> Self {
        let logger = Logger::new("RawText");
        let div    = web::create_div();
        let dom    = DomSymbol::new(&div);
        let frp    = default();
        let size   = default();

        // FIXME It seems by default the text here is mirrored.
        // FIXME This should be fixed in the DOMSymbol directly and removed here.
        dom.set_rotation(Vector3::new(180.0_f32.to_radians(), 0.0, 0.0));

        scene.dom.layers.main.manage(&dom);
        RawText{dom,logger,frp,size}.init()
    }

    fn init(self) -> Self {
        self.reload_style();
        self
    }

    fn reload_style(&self) {
        let style = vec!
            [ "white-space:pre;"
            , "overflow-y:auto;"
            , "overflow-x:auto;"
            , "font-family:dejavuSansMono;"
            , "font-size:11px;"
            , "margin-left:12px;"
            , "color:rgba(255,255,255,0.7);"
            , &format!("height:{}px;",self.size.get().x)
            , &format!("width:{}px;",self.size.get().y)
            , "pointer-events:auto;"
            ].join("");
        self.dom.set_attribute("style",&style).unwrap();
    }
}

impl DataRenderer for RawText {
    fn receive_data(&self, data:Data) -> Result<(),DataError> {
        let data_inner = match data {
            Data::Json {content} => content,
            _ => todo!() // FIXME
        };
        let data_str = serde_json::to_string_pretty(&data_inner);
        let data_str = data_str.unwrap_or_else(|e| format!("<Cannot render data: {}>", e));
        let data_str = format!("\n{}",data_str);
        self.dom.dom().set_inner_text(&data_str);
        Ok(())
    }

    fn set_size(&self, size:V2) {
        self.size.set(size);
        self.reload_style();
    }

    fn frp(&self) -> &DataRendererFrp {
        &self.frp
    }
}

impl display::Object for RawText {
    fn display_object(&self) -> &display::object::Instance {
        &self.dom.display_object()
    }
}
