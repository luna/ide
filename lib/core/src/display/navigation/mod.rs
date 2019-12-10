mod panning;
use panning::Panning;

mod zooming;
use zooming::Zooming;
use zooming::Zoom;

use crate::prelude::*;

use nalgebra::{Vector3, Vector2, clamp};

use crate::display::rendering::Camera;

// ==================
// === Navigation ===
// ==================

pub struct Navigation {
    panning : Rc<Panning>,
    zooming : Rc<Zooming>
}

impl Navigation {
    pub fn new() -> Self {
        let panning = Panning::new();
        let zooming = Zooming::new();
        Navigation { panning, zooming }
    }

    fn pan(&self, camera:&mut Camera, panning:Vector2<f32>) {
        let scale = camera.transform().scale();
        let x = panning.x * scale.x;
        let y = panning.y * scale.y;
        *camera.transform_mut().translation_mut() += Vector3::new(x, y, 0.0)
    }

    fn zoom(&self, camera:&mut Camera, zooming:Zoom) {
        //*scale = Vector3::new(scale.x.max(0.1), scale.y.max(0.1), scale.z.max(0.1));
        // partial max here?
        self.pan(camera, zooming.panning);

        let scale = camera.transform_mut().scale_mut();
        *scale *= zooming.amount;
    }

    pub fn navigate(&self, camera:&mut Camera) {
        if let Some(panning) = self.panning.consume() {
            self.pan(camera, panning);
        }

        if let Some(zooming) = self.zooming.consume(0.01) {
            self.zoom(camera, zooming);
        }
    }
}