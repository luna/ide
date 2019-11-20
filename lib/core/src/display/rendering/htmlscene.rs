use super::HTMLObject;
use super::Scene;
use crate::data::opt_vec::OptVec;
use crate::system::web::Result;
use crate::system::web::StyleSetter;
use crate::prelude::*;

/// A collection for holding 3D `HTMLObject`s.
#[derive(Shrinkwrap)]
#[shrinkwrap(mutable)]
pub struct HTMLScene {
    #[shrinkwrap(main_field)]
    pub scene   : Scene,
    pub div     : HTMLObject,
    pub camera  : HTMLObject,
    pub objects : OptVec<HTMLObject>,
}

impl HTMLScene {
    /// Searches for a HtmlElement identified by id and appends to it.
    ///
    /// # Arguments
    /// * id - the HtmlElement container's id
    pub fn new(id: &str) -> Result<Self> {
        let scene = Scene::new(id)?;
        scene.container.set_property_or_panic("overflow", "hidden");
        let view_dim = scene.get_dimensions();
        let width  = format!("{}px", view_dim.x);
        let height = format!("{}px", view_dim.y);

        let div = HTMLObject::new("div")?;

        div.element.set_property_or_panic("width", &width);
        div.element.set_property_or_panic("height", &height);

        scene.container.append_child(&div.element)
                       .expect("Failed to append div");

        let camera = HTMLObject::new("div")?;

        camera.element.set_property_or_panic("width", width);
        camera.element.set_property_or_panic("height", height);

        div.element.append_child(&camera.element)
                   .expect("Failed to append camera to HTMLScene");

        let objects = OptVec::new();

        Ok(Self { scene, div, camera, objects })
    }

    /// Moves a HTMLObject to the Scene and returns an index to it.
    pub fn add(&mut self, object: HTMLObject) -> usize {
        self.camera.element.append_child(&object.element)
                           .expect("append child");

        self.objects.insert(|_| object)
    }

    /// Removes and retrieves a HTMLObject based on the index provided by
    /// HTMLScene::add.
    /// # Example
    /// ```rust,no_run
    /// use basegl::display::rendering::{HTMLScene, HTMLObject};
    /// let mut scene = HTMLScene::new("an_existing_html_element_id")
    ///                           .expect("scene creation failed");
    /// let object = HTMLObject::new("code").expect("<code> creation failed");
    /// let object_id = scene.add(object);
    /// match scene.remove(object_id) {
    ///     Some(object) => println!("We got the code back! :)"),
    ///     None => println!("Omg! Where is my code? :(")
    /// }
    /// ```
    pub fn remove(&mut self, index: usize) -> Option<HTMLObject> {
        if let Some(object) = self.objects.remove(index) {
            self.camera.element.remove_child(&object.element)
                               .expect("remove child");
            Some(object)
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.objects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.objects.len() == 0
    }
}
