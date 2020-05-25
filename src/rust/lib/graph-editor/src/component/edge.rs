//! Definition of the Edge component.

#![allow(missing_docs)]
// WARNING! UNDER HEAVY DEVELOPMENT. EXPECT DRASTIC CHANGES.

use crate::prelude::*;

use enso_frp;
use enso_frp as frp;
use ensogl::data::color;
use ensogl::display::Attribute;
use ensogl::display::Buffer;
use ensogl::display::Sprite;
use ensogl::display::scene::Scene;
use ensogl::display::shape::*;
use ensogl::display::traits::*;
use ensogl::display;
use ensogl::gui::component;

use super::node;



macro_rules! define_corner {() => {
    /// Shape definition.
    pub mod corner {
        use super::*;
        ensogl::define_shape_system! {
            (radius:f32, start_angle:f32, angle:f32) {
                let radius = 1.px() * radius;
                let width  = LINE_WIDTH.px();
                let width2 = width / 2.0;
                let ring   = Circle(&radius + &width2) - Circle(radius-width2);
                let right : Var<f32> = (std::f32::consts::PI/2.0).into();
                let rot    = right - &angle/2.0;
                let mask   = Plane().cut_angle_fast(angle).rotate(rot);
                let shape  = ring * mask;
                let shape  = shape.fill(color::Rgba::from(color::Lcha::new(0.6,0.5,0.76,1.0)));
                shape.into()
            }
        }
    }
}}

macro_rules! define_line {() => {
    /// Shape definition.
    pub mod line {
        use super::*;
        ensogl::define_shape_system! {
            () {
                let width  = LINE_WIDTH.px();
                let height : Var<Distance<Pixels>> = "input_size.y".into();
                let shape  = Rect((width,height));
                let shape  = shape.fill(color::Rgba::from(color::Lcha::new(0.6,0.5,0.76,1.0)));
                shape.into()
            }
        }
    }
}}


// ============
// === Edge ===
// ============

pub mod front {
    use super::*;
    define_corner!();
    define_line!();
}

pub mod back {
    use super::*;
    define_corner!();
    define_line!();
}

/// Canvas node shape definition.
pub mod helper {
    use super::*;

    ensogl::define_shape_system! {
        () {
            let shape = Circle(2.px());
            let shape = shape.fill(color::Rgba::new(1.0,0.0,0.0,1.0));
            shape.into()
        }
    }
}


const LINE_WIDTH : f32 = 4.0;
const PADDING    : f32 = 5.0;



// ============
// === Edge ===
// ============

/// Edge definition.
#[derive(AsRef,Clone,CloneRef,Debug,Deref)]
pub struct Edge {
    data : Rc<EdgeData>,
}

impl AsRef<Edge> for Edge {
    fn as_ref(&self) -> &Self {
        self
    }
}


#[derive(Clone,CloneRef,Debug)]
pub struct InputEvents {
    pub network         : frp::Network,
    pub source_width    : frp::Source<f32>,
    pub target_position : frp::Source<frp::Position>,
    pub target_attached : frp::Source<bool>,
}

impl InputEvents {
    pub fn new() -> Self {
        frp::new_network! { network
            def source_width    = source();
            def target_position = source();
            def target_attached = source();
        }
        Self {network,source_width,target_position,target_attached}
    }
}

impl Default for InputEvents {
    fn default() -> Self {
        Self::new()
    }
}


pub fn sort_hack_1(scene:&Scene) {
    let logger = Logger::new("hack");
    component::ShapeView::<back::corner::Shape>::new(&logger,scene);
    component::ShapeView::<back::line::Shape>::new(&logger,scene);
}

pub fn sort_hack_2(scene:&Scene) {
    let logger = Logger::new("hack");
    component::ShapeView::<front::line::Shape>::new(&logger,scene);
}


macro_rules! define_components {
    ($name:ident {
        $($field:ident : $field_type:ty),* $(,)?
    }) => {
        #[derive(Debug,Clone,CloneRef)]
        pub struct $name {
            pub logger         : Logger,
            pub display_object : display::object::Instance,
            $(pub $field : component::ShapeView<$field_type>),*
        }

        impl $name {
            pub fn new(logger:Logger, scene:&Scene) -> Self {
                let display_object = display::object::Instance::new(&logger);
                $(let $field = component::ShapeView::new(&logger.sub(stringify!($field)),scene);)*
                $(display_object.add_child(&$field);)*
                Self {logger,display_object,$($field),*}
            }
        }

        impl display::Object for $name {
            fn display_object(&self) -> &display::object::Instance {
                &self.display_object
            }
        }
    }
}

define_components!{
    Front {
        corner    : front::corner::Shape,
        side_line : front::line::Shape,
        main_line : front::line::Shape,
        port_line : front::line::Shape,
    }
}

define_components!{
    Back {
        corner    : back::corner::Shape,
        side_line : back::line::Shape,
        main_line : back::line::Shape,
    }
}


/// Internal data of `Edge`
#[derive(Debug)]
#[allow(missing_docs)]
pub struct EdgeData {
    pub object          : display::object::Instance,
    pub logger          : Logger,
    pub events          : InputEvents,
    pub front           : Front,
    pub back            : Back,
    pub source_width    : Rc<Cell<f32>>,
    pub target_position : Rc<Cell<frp::Position>>,
    pub target_attached : Rc<Cell<bool>>,
}

const END_OFFSET : f32 = 2.0;

impl Edge {
    /// Constructor.
    pub fn new(scene:&Scene) -> Self {
        let logger    = Logger::new("edge");
        let object    = display::object::Instance::new(&logger);
        let front     = Front::new(logger.sub("front"),scene);
        let back      = Back::new(logger.sub("back"),scene);

        object.add_child(&front);
        object.add_child(&back);

        front . side_line.mod_rotation(|r| r.z = std::f32::consts::PI/2.0);
        back  . side_line.mod_rotation(|r| r.z = std::f32::consts::PI/2.0);

        let input = InputEvents::new();
        let network = &input.network;

        let source_width : Rc<Cell<f32>> = default();
        let target_position = Rc::new(Cell::new(frp::Position::default()));
        source_width.set(100.0);

        let target_attached : Rc<Cell<bool>> = default();

        let port_line_height = node::NODE_HEIGHT/2.0 + node::SHADOW_SIZE;
        front . port_line.shape.sprite.size().set(Vector2::new(10.0,port_line_height-END_OFFSET));

        frp::extend! { network
            eval input.target_position ((t) target_position.set(*t));
            eval input.target_attached ((t) target_attached.set(*t));
            eval input.source_width    ((t) source_width.set(*t));
            on_change <- any_ (input.source_width, input.target_position, input.target_attached);
            eval_ on_change ([target_attached,source_width,target_position,object,front,back] {
                let target = target_position.get();
                let target = Vector2::new(target.x - object.position().x, target.y - object.position().y + port_line_height);
                let radius = 14.0;
                let width  = source_width.get() / 2.0;

                let side_circle_x = width - radius;
                let side          = target.x.signum();
                let target        = Vector2::new(target.x.abs(),target.y);

                let corner_grow   = ((target.x - width) * 0.6).max(0.0);
                let corner_radius = 20.0 + corner_grow;
                let corner_radius = corner_radius.min(target.y.abs());
                let corner_x      = target.x - corner_radius;


                let x = (corner_x - side_circle_x).clamp(-corner_radius,radius);
                let y = (radius*radius + corner_radius*corner_radius - x*x).sqrt();


                let angle1        = f32::atan2(y,x);
                let angle2        = f32::atan2(radius,corner_radius);
                let corner_angle  = std::f32::consts::PI - angle1 - angle2;
                let angle_overlap = if corner_x > width { 0.0 } else { 0.1 };

                front.corner.shape.angle.set((corner_angle + angle_overlap) * side);


                let corner_y    = - y;
                let corner_side = (corner_radius + PADDING) * 2.0;
                front.corner.shape.sprite.size().set(Vector2::new(corner_side,corner_side));
                front.corner.shape.radius.set(corner_radius);
                front.corner.mod_position(|t| t.x = corner_x * side);
                front.corner.mod_position(|t| t.y = corner_y);

                let line_overlap = 2.0;
                front.side_line.shape.sprite.size().set(Vector2::new(10.0,corner_x - width + line_overlap));
                front.side_line.mod_position(|p| p.x = side*(width + corner_x)/2.0);

                let main_line_x = side * target.x;
                let main_line_y = (target.y + corner_y) / 2.0;
                let main_line_size = Vector2::new(10.0,corner_y - target.y + line_overlap);
                let main_line_position = Vector3::new(main_line_x,main_line_y,0.0);

                if target_attached.get() {
                    front.main_line.shape.sprite.size().set(Vector2::new(0.0,0.0));
                    back.main_line.shape.sprite.size().set(main_line_size);
                    back.main_line.set_position(main_line_position);
                } else {
                    back.main_line.shape.sprite.size().set(Vector2::new(0.0,0.0));
                    front.main_line.shape.sprite.size().set(main_line_size);
                    front.main_line.set_position(main_line_position);
                }

                front.port_line.mod_position(|p| {
                    p.x = main_line_x;
                    p.y = target.y - port_line_height / 2.0 + END_OFFSET;
                });
            });
        }

        let events = input;
        let data = Rc::new(EdgeData {object,logger,events,front,back
                                          ,source_width,target_position,target_attached});
        Self {data}
    }
}

impl display::Object for Edge {
    fn display_object(&self) -> &display::object::Instance {
        &self.object
    }
}
