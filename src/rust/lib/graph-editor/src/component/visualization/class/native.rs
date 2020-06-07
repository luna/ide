use crate::prelude::*;

use super::*;

use ensogl::display::Scene;



// ==============
// === Native ===
// ==============

/// Type alias for a function that can create a `Visualization`.
pub trait VisualizationConstructor = Fn(&Scene) -> InstantiationResult;

/// Constructor that instantiates visualisations from a given `VisualizationConstructor`. Can be
/// used to wrap the constructor of visualizations defined in Rust.
#[derive(Clone,Derivative)]
#[derivative(Debug)]
#[allow(missing_docs)]
pub struct Native {
    #[derivative(Debug="ignore")]
    constructor : Rc<dyn VisualizationConstructor>,
    signature   : Signature,
}

impl Native {
    /// Create a visualization source from a closure that returns a `Visualization`.
    pub fn new<T>(signature:Signature, constructor:T) -> Self
    where T: Fn(&Scene) -> InstantiationResult + 'static {
        let constructor = Rc::new(constructor);
        Native{signature,constructor}
    }
}

impl Class for Native {
    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn instantiate(&self, scene:&Scene) -> InstantiationResult {
        self.constructor.call((scene,))
    }
}
