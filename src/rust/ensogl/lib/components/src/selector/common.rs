//! Common functionality for both the Number and Range selector.
use crate::prelude::*;

use enso_frp as frp;
use enso_frp::Network;
use ensogl_core::frp::io::Mouse;
use ensogl_core::gui::component::ShapeViewEvents;

pub mod base_frp;
pub mod model;
pub mod shape;

pub use base_frp::*;
pub use model::*;



// ==============
// === Bounds ===
// ==============

/// Bounds of a selection. This indicates the lowest and highest value that can be selected in a
/// selection component.
#[derive(Clone,Copy,Debug,Default)]
pub struct Bounds {
    /// Start of the bounds interval (inclusive).
    pub start : f32,
    /// End of the bounds interval (inclusive).
    pub end   : f32,
}

impl Bounds {
    /// Constructor.
    pub fn new(start:f32,end:f32) -> Self {
        Bounds{start,end}
    }

    /// Return the `Bound` with the lower bound as `start` and the upper bound as `end`.
    pub fn sorted(&self) -> Self {
        if self.start > self.end {
            Bounds{start:self.end,end:self.start}
        } else {
            self.clone()
        }
    }

    /// Return the distance between start and end point.
    pub fn width(&self) -> f32 {
        (self.end - self.start)
    }
}

impl From<(f32,f32)> for Bounds {
    fn from((start,end): (f32, f32)) -> Self {
        Bounds{start,end}
    }
}

/// Frp utility method to normalise the given value to the given Bounds.
pub fn normalise_value((value,bounds):&(f32,Bounds)) -> f32 {
    let width = bounds.width();
    if width == 0.0 { return 0.0 }
    (value - bounds.start) / width
}

/// Frp utility method to compute the absolute value from a normalised value.
/// Inverse of `normalise_value`.
pub fn absolute_value((bounds,normalised_value):&(Bounds,f32)) -> f32 {
    ((normalised_value * bounds.width()) + bounds.start)
}

/// Returns the normalised value that correspond to the click position on the shape.
/// Note that the shape is centered on (0,0), thus half the width extends into the negative values.
/// For use in FRP `map` method, thus taking references.
#[allow(clippy::trivially_copy_pass_by_ref)]
pub fn position_to_normalised_value(pos:&Vector2,width:&f32) -> f32 {
    if *width == 0.0 { return 0.0 }
    ((pos.x / (width / 2.0)) + 1.0) / 2.0
}

/// Check whether the given value is within the given bounds.
fn value_in_bounds(value:f32, bounds:Bounds) -> bool {
    let bounds_sorted = bounds.sorted();
    value >= bounds_sorted.start && value <= bounds_sorted.end
}

/// Check whether the given bounds are completely contained in the second bounds.
pub fn bounds_in_bounds(bounds_inner:Bounds, bounds_outer:Bounds) -> bool {
    value_in_bounds(bounds_inner.start,bounds_outer)
        && value_in_bounds(bounds_inner.end,bounds_outer)
}

/// Clamp `value` to the `overflow_bounds`, or to [0, 1] if no bounds are given.
/// For use in FRP `map` method, thus taking references.
#[allow(clippy::trivially_copy_pass_by_ref)]
pub fn clamp_with_overflow(value:&f32, overflow_bounds:&Option<Bounds>) -> f32 {
    if let Some(overflow_bounds) = overflow_bounds{
        value.clamp(overflow_bounds.start,overflow_bounds.end)
    } else {
        value.clamp(0.0,1.0)
    }
}

/// Indicates whether the `bounds` would be clamped when given to `clamp_with_overflow`.
/// For use in FRP `map` method, thus taking references.
#[allow(clippy::trivially_copy_pass_by_ref)]
pub fn should_clamp_with_overflow(bounds:&Bounds, overflow_bounds:&Option<Bounds>) -> bool {
    if let Some(overflow_bounds) = overflow_bounds {
        bounds_in_bounds(*bounds,*overflow_bounds)
    } else {
        bounds_in_bounds(*bounds,(0.0,1.0).into())
    }
}



// =======================
// === Shape Utilities ===
// =======================


/// Return whether a dragging action has been started from the given shape.
/// A dragging action is started by a mouse down on a shape, followed by a movement of the mouse.
/// It is ended by a mouse up.
pub fn shape_is_dragged
(network:&Network, shape:&ShapeViewEvents, mouse:&Mouse) -> frp::Stream<bool>  {
    frp::extend! { network
        mouse_up              <- mouse.up.constant(());
        mouse_down            <- mouse.down.constant(());
        over_shape            <- bool(&shape.mouse_out,&shape.mouse_over);
        mouse_down_over_shape <- mouse_down.gate(&over_shape);
        is_dragging_shape     <- bool(&mouse_up,&mouse_down_over_shape);
    }
    is_dragging_shape
}

/// Returns the position of a mouse down on a shape. The position is given relative to the origin
/// of the shape position.
pub fn relative_shape_click_position
(base_position:impl Fn() -> Vector2 + 'static, network:&Network, shape:&ShapeViewEvents, mouse:&Mouse) -> frp::Stream<Vector2>  {
    frp::extend! { network
        mouse_down               <- mouse.down.constant(());
        over_shape               <- bool(&shape.mouse_out,&shape.mouse_over);
        mouse_down_over_shape    <- mouse_down.gate(&over_shape);
        background_click_positon <- mouse.position.sample(&mouse_down_over_shape);
        background_click_positon <- background_click_positon.map(move |pos|
            pos - base_position()
        );
    }
    background_click_positon
}


#[cfg(test)]
mod tests {
    use super::*;
    use float_eq::assert_float_eq;
    use std::f32::NAN;
    use enso_frp::stream::ValueProvider;
    use enso_frp::stream::EventEmitter;
    use enso_frp::io::mouse::Button;

    #[test]
    fn test_normalise_value() {
        let test = |start,end,value,expected| {
            let bounds = Bounds::new(start,end);
            let normalised = normalise_value(&(value,bounds));
            assert_float_eq!(normalised,expected,ulps<=7)
        };

        test(0.0,1.0,0.0,0.0);
        test(0.0,1.0,0.1,0.1);
        test(0.0,1.0,0.2,0.2);
        test(0.0,1.0,0.3,0.3);
        test(0.0,1.0,0.4,0.4);
        test(0.0,1.0,0.5,0.5);
        test(0.0,1.0,0.6,0.6);
        test(0.0,1.0,0.7,0.7);
        test(0.0,1.0,0.7,0.7);
        test(0.0,1.0,0.7,0.7);
        test(0.0,1.0,1.0,1.0);

        test(0.0,1.0,-2.0,-2.0);
        test(0.0,1.0,-1.0,-1.0);
        test(0.0,1.0,2.0,2.0);
        test(0.0,1.0,3.0,3.0);

        test(-1.0,1.0,-1.0,0.0);
        test(-1.0,1.0,-0.5,0.25);
        test(-1.0,1.0,0.0,0.5);
        test(-1.0,1.0,0.5,0.75);
        test(-1.0,1.0,1.0,1.0);

        test(1.0,-1.0,-1.0,1.0);
        test(1.0,-1.0,-0.5,0.75);
        test(1.0,-1.0,0.0,0.5);
        test(1.0,-1.0,0.5,0.25);
        test(1.0,-1.0,1.0,0.0);

        test(-10.0,20.0,-10.0,0.0);
        test(-10.0,20.0,20.0,1.0);
        test(-10.0,20.0,0.0,0.33333333);

        test(-999999999.0,999999999.0,-999999999.0,0.0);
        test(-999999999.0,999999999.0,0.0,0.5);
        test(-999999999.0,999999999.0,999999999.0,1.0);

        test(0.0,0.0,1.0,0.0);
        test(0.0,0.0,0.0,0.0);
        test(0.0,0.0,-1.0,0.0);
    }

    #[test]
    fn test_absolute_value() {
        let test = |start,end,value,expected| {
            let bounds = Bounds::new(start,end);
            let normalised = absolute_value(&(bounds,value));
            assert_float_eq!(normalised,expected,ulps<=7)
        };

        test(0.0,1.0,0.0,0.0);
        test(0.0,1.0,0.1,0.1);
        test(0.0,1.0,0.2,0.2);
        test(0.0,1.0,0.3,0.3);
        test(0.0,1.0,0.4,0.4);
        test(0.0,1.0,0.5,0.5);
        test(0.0,1.0,0.6,0.6);
        test(0.0,1.0,0.7,0.7);
        test(0.0,1.0,0.7,0.7);
        test(0.0,1.0,0.7,0.7);
        test(0.0,1.0,1.0,1.0);

        test(0.0,1.0,-2.0,-2.0);
        test(0.0,1.0,-1.0,-1.0);
        test(0.0,1.0,2.0,2.0);
        test(0.0,1.0,3.0,3.0);

        test(-1.0,1.0,0.0,-1.0);
        test(-1.0,1.0,0.25,-0.5);
        test(-1.0,1.0,0.5,0.0);
        test(-1.0,1.0,0.75,0.5);
        test(-1.0,1.0,1.0,1.0);

        test(1.0,-1.0,1.0,-1.0);
        test(1.0,-1.0,0.75,-0.5);
        test(1.0,-1.0,0.5,0.0);
        test(1.0,-1.0,0.25,0.5);
        test(1.0,-1.0,0.0,1.0);

        test(-10.0,20.0,0.0,-10.0);
        test(-10.0,20.0,1.0,20.0);
        test(-10.0,20.0,0.33333333,0.0);

        test(-999999999.0,999999999.0,0.0,-999999999.0);
        test(-999999999.0,999999999.0,0.5,0.0);
        test(-999999999.0,999999999.0,1.0,999999999.0);

        test(0.0,0.0,1.0,0.0);
        test(1.0,1.0,1.0,1.0);
        test(1.0,1.0,2.0,1.0);
        test(1.0,1.0,-2.0,1.0);
    }


    #[test]
    fn test_position_to_normalised_value() {
        let test = |pos,width,expected| {
            let result = position_to_normalised_value(&pos,&width);
            assert_float_eq!(result,expected,ulps<=7)
        };

        for &y in &[-100.0, 0.0, 100.0, NAN] {
            test(Vector2::new(50.0,y),100.0,1.0);
            test(Vector2::new(0.0,y),100.0,0.5);
            test(Vector2::new(-50.0,y),100.0,0.0);

            test(Vector2::new(100.0,y),100.0,1.5);
            test(Vector2::new(-100.0,y),100.0,-0.5);
            test(Vector2::new(150.0,y),100.0,2.0);
            test(Vector2::new(-150.0,y),100.0,-1.0);
            test(Vector2::new(200.0,y),100.0,2.5);
            test(Vector2::new(-200.0,y),100.0,-1.5);

            test(Vector2::new(-200.0,y),0.0,0.0);
        }
    }

    #[test]
    fn test_value_in_bounds() {
        let test = |start,end,value,expected| {
            let result = value_in_bounds(value,Bounds::new(start,end));
            assert_eq!(result,expected, "Testing whether {} in ]{},{}[", value,start,end)
        };

        test(0.0,1.0,0.0,true);
        test(0.0,1.0,0.5,true);
        test(0.0,1.0,1.0,true);
        test(0.0,1.0,1.00001,false);
        test(0.0,1.0,-0.00001,false);

        test(0.0,10.0,10.0,true);
        test(0.0,10.0,9.999999,true);
        test(0.0,10.0,11.0,false);

        test(-100.0,10.0,11.0,false);
        test(-101.0,10.0,-100.0,true);
        test(-101.0,10.0,-101.0,true);
        test(-101.0,10.0,-101.1,false);

        test(0.0,0.0,0.0,true);
        test(0.0,0.0,1.0,false);
        test(0.0,0.0,-1.0,false);
    }

    #[test]
    fn test_bounds_in_bounds() {
        let test = |start1,end1,start2,end2,expected| {
            let result = bounds_in_bounds(Bounds::new(start1,start2),Bounds::new(start2,end2));
            assert_eq!(result,expected,
                       "Testing whether ]{},{}[ in ]{},{}[", start1,end1,start2,end2);
        };

        test(0.0,1.0,0.0,1.0,true);
        test(0.0,1.0,1.0,2.0,false);
        test(0.0,1.0,0.5,2.0,false);
        test(0.0,1.0,-100.0,100.0,true);
        test(0.0,1.0,-100.0,-99.0,false);
        test(0.0,1.0,0.1,0.9,false);
        test(-100.0,200.0,50.0,75.0,false);
        test(-100.0,200.0,-50.0,75.0,false);
        test(-100.0,200.0,-50.0,-75.0,false);
        test(-100.0,200.0,-50.0,99999.0,false);
        test(-100.0,200.0,-99999.0,0.0,true);
        test(-100.0,200.0,-99999.0,99999.0,true);

        test(0.0,0.0,0.0,0.0,true);
        test(0.0,0.0,-1.0,2.0,true);
        test(0.0,0.0,1.0,2.0,false);
    }

    #[test]
    fn test_clamp_with_overflow() {
        let test = |value,bounds,expected| {
            let result = clamp_with_overflow(&value,&bounds);
            assert_float_eq!(result,expected,ulps<=7)
        };

        test(0.0,Some(Bounds::new(0.0,1.0)), 0.0);
        test(-1.0,Some(Bounds::new(0.0,1.0)), 0.0);
        test(2.0,Some(Bounds::new(0.0,1.0)), 1.0);

        test(-1.0,None, 0.0);
        test(2.0,None,1.0);

        test(-999.0,Some(Bounds::new(-1.0,100.0)), -1.0);
        test(999.0,Some(Bounds::new(-1.0,100.0)), 100.0);
        test(-1.0,Some(Bounds::new(-1.0,100.0)), -1.0);
        test(0.0,Some(Bounds::new(-1.0,100.0)), 0.0);
        test(99.0,Some(Bounds::new(-1.0,100.0)), 99.0);
        test(100.0,Some(Bounds::new(-1.0,100.0)), 100.0);
        test(100.01,Some(Bounds::new(-1.0,100.0)), 100.0);
    }

    #[test]
    fn test_should_clamp_with_overflow() {
        let test = |inner,outer,expected| {
            let result = should_clamp_with_overflow(&inner,&outer);
            assert_eq!(result,expected);
        };

        test(Bounds::new(0.0,1.0),Some(Bounds::new(0.0,1.0)),true);
        test(Bounds::new(0.0,1.0),Some(Bounds::new(1.0,2.0)),false);
        test(Bounds::new(0.0,1.0),Some(Bounds::new(0.5,2.0)),false);
        test(Bounds::new(0.0,1.0),Some(Bounds::new(-100.0,100.0)),true);
        test(Bounds::new(0.0,1.0),Some(Bounds::new(-100.0,-99.0)),false);
        test(Bounds::new(0.0,1.0),Some(Bounds::new(0.1,0.9)),false);
        test(Bounds::new(-100.0,200.0),Some(Bounds::new(50.0,75.0)),false);
        test(Bounds::new(-100.0,200.0),Some(Bounds::new(-50.0,75.0)),false);
        test(Bounds::new(-100.0,200.0),Some(Bounds::new(-50.0,-75.0)),false);
        test(Bounds::new(-100.0,200.0),Some(Bounds::new(-50.0,99999.0)),false);
        test(Bounds::new(-100.0,200.0),Some(Bounds::new(-99999.0,0.0)),false);
        test(Bounds::new(-100.0,200.0),Some(Bounds::new(-99999.0,99999.0)),true);
        test(Bounds::new(-100.0,0.0),None,false);
        test(Bounds::new(0.1,1.1),None,false);
        test(Bounds::new(-9.1,2.1),None,false);
        test(Bounds::new(0.25,0.7),None,true);

        test(Bounds::new(0.0,0.0),None,true);
    }

    #[test]
    fn test_shape_is_dragged() {
        let network = frp::Network::new("TestNetwork");
        let mouse   = frp::io::Mouse::default();
        let shape   = ShapeViewEvents::default();

        let is_dragged = shape_is_dragged(&network,&shape,&mouse);
        let _watch = is_dragged.register_watch();


        // Default is false.
        assert_eq!(is_dragged.value(),false);

        // Mouse down over shape activates dragging.
        shape.mouse_over.emit(());
        mouse.down.emit(Button::from_code(0));
        assert_eq!(is_dragged.value(),true);

        // Release mouse stops dragging.
        mouse.up.emit(Button::from_code(0));
        assert_eq!(is_dragged.value(),false);

        // Mouse down while not over shape  does not activate dragging.
        shape.mouse_out.emit(());
        mouse.down.emit(Button::from_code(0));
        assert_eq!(is_dragged.value(),false);
    }

    #[test]
    fn test_relative_shape_click_position() {
        let network = frp::Network::new("TestNetwork");
        let mouse   = frp::io::Mouse::default();
        let shape   = ShapeViewEvents::default();

        let base_position = || Vector2::new(-10.0,200.0);
        let click_position = relative_shape_click_position(base_position, &network,&shape,&mouse);
        let _watch = click_position.register_watch();

        shape.mouse_over.emit(());
        mouse.position.emit(Vector2::new(-10.0,200.0));
        mouse.down.emit(Button::from_code(0));
        assert_float_eq!(click_position.value().x,0.0,ulps<=7);
        assert_float_eq!(click_position.value().y,0.0,ulps<=7);

        mouse.position.emit(Vector2::new(0.0,0.0));
        mouse.down.emit(Button::from_code(0));
        assert_float_eq!(click_position.value().x,10.0,ulps<=7);
        assert_float_eq!(click_position.value().y,-200.0,ulps<=7);

        mouse.position.emit(Vector2::new(400.0,0.5));
        mouse.down.emit(Button::from_code(0));
        assert_float_eq!(click_position.value().x,410.0,ulps<=7);
        assert_float_eq!(click_position.value().y,-199.5,ulps<=7);
    }
}