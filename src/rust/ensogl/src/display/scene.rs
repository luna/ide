#![allow(missing_docs)]

#[warn(missing_docs)]
pub mod dom;

pub use crate::display::symbol::registry::SymbolId;

use crate::prelude::*;
use crate::display::traits::*;

use crate::closure;
use crate::control::callback::CallbackHandle;
use crate::control::callback::DynEvent;
use crate::control::io::mouse::MouseFrpCallbackHandles;
use crate::control::io::mouse::MouseManager;
use crate::control::io::mouse;
use crate::data::dirty::traits::*;
use crate::data::dirty;
use crate::debug::stats::Stats;
use crate::display::camera::Camera2d;
use crate::display::render::RenderComposer;
use crate::display::render::RenderPipeline;
use crate::display::symbol::registry::SymbolRegistry;
use crate::display::symbol::Symbol;
use crate::display;
use crate::system::gpu::data::uniform::Uniform;
use crate::system::gpu::data::uniform::UniformScope;
use crate::system::gpu::shader::Context;
use crate::system::gpu::types::*;
use crate::display::scene::dom::DomScene;
use crate::system::web::NodeInserter;
use crate::system::web::resize_observer::ResizeObserver;
use crate::system::web::StyleSetter;
use crate::system::web;
use crate::display::shape::primitive::system::ShapeSystem;

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsValue;
use web_sys::HtmlElement;

use enso_frp;
use enso_frp::core::node::class::EventEmitterPoly;


pub trait SceneBasedConstructor {
    fn new(scene:&Scene) -> Self;
}


pub trait Component : CloneRef + 'static {
    type ComponentSystem : SceneBasedConstructor + CloneRef;
}

pub type ComponentSystem<T> = <T as Component>::ComponentSystem;


pub trait MouseTarget : Debug + 'static {
    fn mouse_down(&self) -> &enso_frp::Dynamic<()>;
}


// =====================
// === ShapeRegistry ===
// =====================

use std::any::TypeId;

shared! { ShapeRegistry
#[derive(Debug,Default)]
pub struct ShapeRegistryData {
    scene            : Option<Scene>,
    shape_system_map : HashMap<TypeId,Box<dyn Any>>,
    mouse_target_map : HashMap<usize,Rc<dyn MouseTarget>>,
}

impl {
    pub fn get<T:Component>(&self, tp:PhantomData<T>) -> Option<ComponentSystem<T>> {
        let id = TypeId::of::<T>();
        self.shape_system_map.get(&id).and_then(|any| any.downcast_ref::<ComponentSystem<T>>()).map(|t| t.clone_ref())
    }

    pub fn insert<T:Component>(&mut self, tp:PhantomData<T>, system:ComponentSystem<T>) {
        let id     = TypeId::of::<T>();
        let system = Box::new(system) as Box<dyn Any>;
        self.shape_system_map.insert(id,system);
    }

    pub fn register<T:Component>(&mut self, tp:PhantomData<T>) {
        let id     = TypeId::of::<T>();
        let system = <ComponentSystem<T>>::new(self.scene.as_ref().unwrap());
        let system = Box::new(system) as Box<dyn Any>;
        self.shape_system_map.insert(id,system);
    }

    pub fn insert_mouse_target<T:MouseTarget>(&mut self, id:usize, target:T) {
        let target = Rc::new(target);
        self.mouse_target_map.insert(id,target);
    }

    pub fn remove_mouse_target(&mut self, id:&usize) {
        self.mouse_target_map.remove(id);
    }

    pub fn get_mouse_target(&mut self, id:&usize) -> Option<Rc<dyn MouseTarget>> {
        self.mouse_target_map.get(&id).map(|t| t.clone_ref())
    }
}}



// ==============
// === Target ===
// ==============

#[derive(Debug,Clone,Copy,Eq,PartialEq)]
pub enum Target {
    Background,
    Symbol {
        symbol_id   : u32,
        instance_id : u32,
    }
}

impl Target {
    fn to_internal(&self) -> Vector4<u32> {
        match self {
            Self::Background                     => Vector4::new(0,0,0,0),
            Self::Symbol {symbol_id,instance_id} => Vector4::new(*symbol_id,*instance_id,0,1),
        }
    }

    fn from_internal(v:Vector4<u32>) -> Self {
        if v.z != 0 {
            panic!("Wrong internal format for mouse target.")
        }
        if v.w == 0 {
            Self::Background
        }
        else if v.w == 1 {
            let symbol_id   = v.x;
            let instance_id = v.y;
            Self::Symbol {symbol_id,instance_id}
        } else {
            panic!("Wrong internal format alpha for mouse target.")
        }
    }
}

impl Default for Target {
    fn default() -> Self {
        Self::Background
    }
}



// =============
// === Mouse ===
// =============

pub trait MouseEventFn      = Fn(JsValue) + 'static;
pub type  MouseEventClosure = Closure<dyn Fn(JsValue)>;

fn mouse_event_closure<F:MouseEventFn>(f:F) -> MouseEventClosure {
    Closure::wrap(Box::new(f))
}

#[derive(Clone,Debug)]
pub struct Mouse {
    pub mouse_manager   : MouseManager,
    pub position        : Uniform<Vector2<i32>>,
    pub hover_ids       : Uniform<Vector4<u32>>,
    pub button0_pressed : Uniform<bool>,
    pub button1_pressed : Uniform<bool>,
    pub button2_pressed : Uniform<bool>,
    pub button3_pressed : Uniform<bool>,
    pub button4_pressed : Uniform<bool>,
    pub target          : Rc<Cell<Target>>,
    pub handles         : Rc<Vec<CallbackHandle>>,
    pub frp             : enso_frp::Mouse,
}

impl CloneRef for Mouse {
    fn clone_ref(&self) -> Self {
        let mouse_manager   = self.mouse_manager.clone_ref();
        let position        = self.position.clone_ref();
        let hover_ids       = self.hover_ids.clone_ref();
        let button0_pressed = self.button0_pressed.clone_ref();
        let button1_pressed = self.button1_pressed.clone_ref();
        let button2_pressed = self.button2_pressed.clone_ref();
        let button3_pressed = self.button3_pressed.clone_ref();
        let button4_pressed = self.button4_pressed.clone_ref();
        let target          = self.target.clone_ref();
        let handles         = self.handles.clone_ref();
        let frp             = self.frp.clone_ref();
        Self {mouse_manager,position,hover_ids,button0_pressed,button1_pressed,button2_pressed
             ,button3_pressed,button4_pressed,target,handles,frp}
    }
}

impl Mouse {
    pub fn new(shape:&web::dom::Shape, variables:&UniformScope) -> Self {

        let target          = Target::default();
        let position        = variables.add_or_panic("mouse_position",Vector2::new(0,0));
        let hover_ids       = variables.add_or_panic("mouse_hover_ids",target.to_internal());
        let button0_pressed = variables.add_or_panic("mouse_button0_pressed",false);
        let button1_pressed = variables.add_or_panic("mouse_button1_pressed",false);
        let button2_pressed = variables.add_or_panic("mouse_button2_pressed",false);
        let button3_pressed = variables.add_or_panic("mouse_button3_pressed",false);
        let button4_pressed = variables.add_or_panic("mouse_button4_pressed",false);
        let target          = Rc::new(Cell::new(target));
        let document        = web::dom::WithKnownShape::new(&web::document().body().unwrap());
        let mouse_manager   = MouseManager::new(&document.into());

        let shape_ref       = shape.clone_ref();
        let position_ref    = position.clone_ref();
        let on_move_handle  = mouse_manager.on_move.add(move |event:&mouse::event::OnMove| {
            let pixel_ratio = shape_ref.pixel_ratio() as i32;
            let screen_x    = event.offset_x();
            let screen_y    = event.offset_y();
            let canvas_x    = pixel_ratio * screen_x;
            let canvas_y    = pixel_ratio * screen_y;
            position_ref.set(Vector2::new(canvas_x,canvas_y))
        });

        let button0_pressed_ref = button0_pressed.clone_ref();
        let button1_pressed_ref = button1_pressed.clone_ref();
        let button2_pressed_ref = button2_pressed.clone_ref();
        let button3_pressed_ref = button3_pressed.clone_ref();
        let button4_pressed_ref = button4_pressed.clone_ref();
        let on_down_handle      = mouse_manager.on_down.add(move |event:&mouse::event::OnDown| {
            match event.button() {
                mouse::Button0 => button0_pressed_ref.set(true),
                mouse::Button1 => button1_pressed_ref.set(true),
                mouse::Button2 => button2_pressed_ref.set(true),
                mouse::Button3 => button3_pressed_ref.set(true),
                mouse::Button4 => button4_pressed_ref.set(true),
            }
        });

        let button0_pressed_ref = button0_pressed.clone_ref();
        let button1_pressed_ref = button1_pressed.clone_ref();
        let button2_pressed_ref = button2_pressed.clone_ref();
        let button3_pressed_ref = button3_pressed.clone_ref();
        let button4_pressed_ref = button4_pressed.clone_ref();
        let on_up_handle        = mouse_manager.on_up.add(move |event:&mouse::event::OnUp| {
            match event.button() {
                mouse::Button0 => button0_pressed_ref.set(false),
                mouse::Button1 => button1_pressed_ref.set(false),
                mouse::Button2 => button2_pressed_ref.set(false),
                mouse::Button3 => button3_pressed_ref.set(false),
                mouse::Button4 => button4_pressed_ref.set(false),
            }
        });

        let handles = Rc::new(vec![on_move_handle,on_down_handle,on_up_handle]);

        let frp = enso_frp::Mouse::new();

        let event = frp.position.event.clone_ref();
        mouse_manager.on_move.add(move |e:&mouse::OnMove| {
            let position = enso_frp::Position::new(e.client_x(),e.client_y());
            event.emit(position);
        }).forget();

        let event = frp.on_down.event.clone_ref();
        mouse_manager.on_down.add(move |_:&mouse::OnDown| {
            event.emit(());
        }).forget();

        let event = frp.on_up.event.clone_ref();
        mouse_manager.on_up.add(move |_:&mouse::OnUp| {
            event.emit(());
        }).forget();

        Self {mouse_manager,position,hover_ids,button0_pressed,button1_pressed,button2_pressed,button3_pressed
             ,button4_pressed,target,handles,frp}
    }
}



// ===========
// === Dom ===
// ===========

/// DOM element manager
#[derive(Clone,Debug)]
pub struct Dom {
    /// Root DOM element of the scene.
    pub root : web::dom::WithKnownShape<web::HtmlDivElement>,
    /// Layers of the scene.
    pub layers : Layers,
}

impl CloneRef for Dom {}

impl Dom {
    /// Constructor.
    pub fn new(logger:&Logger) -> Self {
        let root   = web::create_div();
        let layers = Layers::new(&logger,&root);
        root.set_class_name("scene");
        root.set_style_or_panic("height"  , "100vh");
        root.set_style_or_panic("width"   , "100vw");
        root.set_style_or_panic("display" , "block");
        let root = web::dom::WithKnownShape::new(&root);
        Self {root,layers}
    }

    pub fn shape(&self) -> &web::dom::Shape {
        self.root.shape()
    }

    pub fn recompute_shape_with_reflow(&self) {
        self.shape().set_from_element_with_reflow(&self.root);
    }
}



// ==============
// === Layers ===
// ==============

/// DOM Layers of the scene. It contains a 2 CSS 3D layers and a canvas layer in the middle. The
/// CSS layers are used to manage DOM elements and to simulate depth-sorting of DOM and canvas
/// elements.
#[derive(Clone,Debug)]
pub struct Layers {
    /// Front DOM scene layer.
    pub front : DomScene,
    /// The WebGL scene layer.
    pub canvas : web_sys::HtmlCanvasElement,
    /// Back DOM scene layer.
    pub back : DomScene,
}

impl Layers {
    /// Constructor.
    pub fn new(logger:&Logger, dom:&web_sys::HtmlDivElement) -> Self {
        let canvas = web::create_canvas();
        let front  = DomScene::new(&logger);
        let back   = DomScene::new(&logger);
        canvas.set_style_or_panic("height"  , "100vh");
        canvas.set_style_or_panic("width"   , "100vw");
        canvas.set_style_or_panic("display" , "block");
        front.dom.set_class_name("front");
        back.dom.set_class_name("back");
        dom.append_or_panic(&front.dom);
        dom.append_or_panic(&canvas);
        dom.append_or_panic(&back.dom);
        back.set_z_index(-1);
        Self {front,canvas,back}
    }
}



// ================
// === Uniforms ===
// ================

/// Uniforms owned by the scene.
#[derive(Clone,Debug)]
pub struct Uniforms {
    /// Pixel ratio of the screen used to display the scene.
    pub pixel_ratio : Uniform<f32>,
    /// Zoom of the camera to objects on the scene. Zoom of 1.0 means that unit distance is 1 px.
    pub zoom : Uniform<f32>,
}

impl Uniforms {
    /// Constructor.
    pub fn new(scope:&UniformScope) -> Self {
        let pixel_ratio = scope.add_or_panic("pixel_ratio" , 1.0);
        let zoom        = scope.add_or_panic("zoom"        , 1.0);
        Self {pixel_ratio,zoom}
    }
}

impl CloneRef for Uniforms {
    fn clone_ref(&self) -> Self {
        let pixel_ratio = self.pixel_ratio.clone_ref();
        let zoom        = self.zoom.clone_ref();
        Self {pixel_ratio,zoom}
    }
}



// =============
// === Dirty ===
// =============

pub type ShapeDirty          = dirty::SharedBool<Box<dyn Fn()>>;
pub type SymbolRegistryDirty = dirty::SharedBool<Box<dyn Fn()>>;

#[derive(Clone,Debug)]
pub struct Dirty {
    symbols : SymbolRegistryDirty,
    shape   : ShapeDirty,
}

impl CloneRef for Dirty {
    fn clone_ref(&self) -> Self {
        let symbols = self.symbols.clone_ref();
        let shape   = self.shape.clone_ref();
        Self {symbols,shape}
    }
}



// =================
// === Callbacks ===
// =================

#[derive(Clone,Debug)]
pub struct Callbacks {
    on_zoom   : CallbackHandle,
    on_resize : CallbackHandle,
}

impl CloneRef for Callbacks {
    fn clone_ref(&self) -> Self {
        let on_zoom   = self.on_zoom.clone_ref();
        let on_resize = self.on_resize.clone_ref();
        Self {on_zoom,on_resize}
    }
}



// ================
// === Renderer ===
// ================

#[derive(Clone,Debug)]
pub struct Renderer {
    logger    : Logger,
    dom       : Dom,
    context   : Context,
    variables : UniformScope,

    pub pipeline : Rc<CloneCell<RenderPipeline>>,
    pub composer : Rc<CloneCell<RenderComposer>>,
}

impl Renderer {
    pub fn new(logger:&Logger, dom:&Dom, context:&Context, variables:&UniformScope) -> Self {
        let logger    = logger.sub("renderer");
        let dom       = dom.clone_ref();
        let context   = context.clone_ref();
        let variables = variables.clone_ref();
        let pipeline  = default();
        let width     = dom.shape().current().device_pixels().width()  as i32;
        let height    = dom.shape().current().device_pixels().height() as i32;
        let composer  = RenderComposer::new(&pipeline,&context,&variables,width,height);
        let pipeline  = Rc::new(CloneCell::new(pipeline));
        let composer  = Rc::new(CloneCell::new(composer));

        context.enable(Context::BLEND);
        // To learn more about the blending equations used here, please see the following articles:
        // - http://www.realtimerendering.com/blog/gpus-prefer-premultiplication
        // - https://www.khronos.org/opengl/wiki/Blending#Colors
        context.blend_equation_separate ( Context::FUNC_ADD, Context::FUNC_ADD );
        context.blend_func_separate     ( Context::ONE , Context::ONE_MINUS_SRC_ALPHA
                                        , Context::ONE , Context::ONE_MINUS_SRC_ALPHA );

        Self {logger,dom,context,variables,pipeline,composer}
    }

    pub fn set_pipeline<P:Into<RenderPipeline>>(&self, pipeline:P) {
        self.pipeline.set(pipeline.into());
        self.reload_composer();
    }

    pub fn reload_composer(&self) {
        let width    = self.dom.shape().current().device_pixels().width()  as i32;
        let height   = self.dom.shape().current().device_pixels().height() as i32;
        let pipeline = self.pipeline.get();
        let composer = RenderComposer::new(&pipeline,&self.context,&self.variables,width,height);
        self.composer.set(composer);
    }

    /// Run the renderer.
    pub fn run(&self) {
        group!(self.logger, "Running.", {
            self.composer.get().run();
        })
    }
}

impl CloneRef for Renderer {
    fn clone_ref(&self) -> Self {
        let logger    = self.logger.clone_ref();
        let dom       = self.dom.clone_ref();
        let context   = self.context.clone_ref();
        let variables = self.variables.clone_ref();
        let pipeline  = self.pipeline.clone_ref();
        let composer  = self.composer.clone_ref();
        Self {logger,dom,context,variables,pipeline,composer}
    }
}



// ============
// === View ===
// ============

// === Definition ===

#[derive(Debug,Clone)]
pub struct View {
    data : Rc<ViewData>
}

#[derive(Debug,Clone)]
pub struct WeakView {
    data : Weak<ViewData>
}

#[derive(Debug,Clone)]
pub struct ViewData {
    logger  : Logger,
    pub camera  : Camera2d,
    symbols : RefCell<Vec<SymbolId>>,
}

impl CloneRef for View {}
impl CloneRef for WeakView {}

impl AsRef<ViewData> for View {
    fn as_ref(&self) -> &ViewData {
        &self.data
    }
}

impl std::borrow::Borrow<ViewData> for View {
    fn borrow(&self) -> &ViewData {
        &self.data
    }
}

impl Deref for View {
    type Target = ViewData;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}


// === API ===

impl View {
    pub fn new(logger:&Logger, width:f32, height:f32) -> Self {
        let data = ViewData::new(logger,width,height);
        let data = Rc::new(data);
        Self {data}
    }

    pub fn downgrade(&self) -> WeakView {
        let data = Rc::downgrade(&self.data);
        WeakView {data}
    }

    pub fn add(&self, symbol:&Symbol) {
        self.symbols.borrow_mut().push(symbol.id as usize); // TODO strange conversion
    }

    pub fn remove(&self, symbol:&Symbol) {
        self.symbols.borrow_mut().remove_item(&(symbol.id as usize)); // TODO strange conversion
    }
}

impl WeakView {
    pub fn upgrade(&self) -> Option<View> {
        self.data.upgrade().map(|data| View{data})
    }
}

impl ViewData {
    pub fn new(logger:&Logger, width:f32, height:f32) -> Self {
        let logger  = logger.sub("view");
        let camera  = Camera2d::new(&logger,width,height);
        let symbols = default();
        Self {logger,camera,symbols}
    }

    pub fn symbols(&self) -> Ref<Vec<SymbolId>> {
        self.symbols.borrow()
    }
}



// =============
// === Views ===
// =============

#[derive(Clone,Debug)]
pub struct Views {
    logger   : Logger,
    pub main : View,
    all      : Rc<RefCell<Vec<WeakView>>>,
    width    : f32,
    height   : f32,
}

impl CloneRef for Views {
    fn clone_ref(&self) -> Self {
        let logger = self.logger.clone_ref();
        let main   = self.main.clone_ref();
        let all    = self.all.clone_ref();
        let width  = self.width;  // FIXME
        let height = self.height; // FIXME
        Self {logger,main,all,width,height}
    }
}

impl Views {
    pub fn mk(logger:&Logger, width:f32, height:f32) -> Self {
        let logger = logger.sub("views");
        let main   = View::new(&logger,width,height);
        let all    = Rc::new(RefCell::new(vec![main.downgrade()]));
        Self {logger,main,all,width,height}
    }

    pub fn new(&self) -> View {
        let view = View::new(&self.logger,self.width,self.height);
        self.all.borrow_mut().push(view.downgrade());
        view
    }

    pub fn all(&self) -> Ref<Vec<WeakView>> {
        self.all.borrow()
    }
}



// =================
// === SceneData ===
// =================

#[derive(Clone,Debug)]
pub struct SceneData {
    pub display_object : display::object::Node,
    pub dom            : Dom,
    pub context        : Context,
    symbols            : SymbolRegistry,
    pub variables      : UniformScope,
    pub mouse          : Mouse,
    pub uniforms       : Uniforms,
    pub shapes         : ShapeRegistry,
    pub stats          : Stats,
    pub dirty          : Dirty,
    pub logger         : Logger,
    pub callbacks      : Callbacks,
    pub renderer       : Renderer,
    pub views          : Views,
}

impl CloneRef for SceneData {
    fn clone_ref(&self) -> Self {
        let display_object = self.display_object.clone_ref();
        let dom            = self.dom.clone_ref();
        let context        = self.context.clone_ref();
        let symbols        = self.symbols.clone_ref();
        let dirty          = self.dirty.clone_ref();
        let views          = self.views.clone_ref();
        let logger         = self.logger.clone_ref();
        let variables      = self.variables.clone_ref();
        let renderer       = self.renderer.clone_ref();
        let stats          = self.stats.clone_ref();
        let uniforms       = self.uniforms.clone_ref();
        let mouse          = self.mouse.clone_ref();
        let callbacks      = self.callbacks.clone_ref();
        let shapes         = self.shapes.clone_ref();
        Self {display_object,dom,context,symbols,dirty,views,logger,variables,renderer,stats
             ,uniforms,callbacks,mouse,shapes}
    }
}

impl SceneData {
    /// Create new instance with the provided on-dirty callback.
    pub fn new<OnMut:Fn()+Clone+'static>
    (parent_dom:&HtmlElement, logger:Logger, stats:&Stats, on_mut:OnMut) -> Self {
        logger.trace("Initializing.");

        let dom = Dom::new(&logger);
        parent_dom.append_child(&dom.root).unwrap();
        dom.recompute_shape_with_reflow();

        let display_object = display::object::Node::new(&logger);
        let context        = web::get_webgl2_context(&dom.layers.canvas);
        let sub_logger     = logger.sub("shape_dirty");
        let shape_dirty    = ShapeDirty::new(sub_logger,Box::new(on_mut.clone()));
        let sub_logger     = logger.sub("symbols_dirty");
        let dirty_flag     = SymbolRegistryDirty::new(sub_logger,Box::new(on_mut));
        let on_change      = enclose!((dirty_flag) move || dirty_flag.set());
        let variables      = UniformScope::new(logger.sub("global_variables"),&context);
        let symbols        = SymbolRegistry::mk(&variables,&stats,&context,&logger,on_change);
        let screen_shape   = dom.shape().current();
        let width          = screen_shape.width();
        let height         = screen_shape.height();
        let symbols_dirty  = dirty_flag;
        let views          = Views::mk(&logger,width,height);
        let stats          = stats.clone();
        let mouse          = Mouse::new(&dom.shape(),&variables);
        let shapes         = ShapeRegistry::default();
        let uniforms       = Uniforms::new(&variables);
        let dirty          = Dirty {symbols:symbols_dirty,shape:shape_dirty};
        let renderer       = Renderer::new(&logger,&dom,&context,&variables);
        let on_zoom_cb     = enclose!((uniforms) move |zoom:&f32| uniforms.zoom.set(*zoom));
        let on_resize_cb   = enclose!((dirty) move |_:&web::dom::ShapeData| dirty.shape.set());
        let on_zoom        = views.main.camera.add_zoom_update_callback(on_zoom_cb);
        let on_resize      = dom.root.on_resize(on_resize_cb);
        let callbacks      = Callbacks {on_zoom,on_resize};

        uniforms.zoom.set(dom.shape().pixel_ratio());
        Self {renderer,display_object,dom,context,symbols,views,dirty,logger,variables
             ,stats,uniforms,mouse,callbacks,shapes}
    }

    /// Bind FRP graph to mouse js events.
    #[deprecated(note="Please use `scene.mouse.frp` instead")]
    pub fn bind_frp_to_mouse_events(&self, frp:&enso_frp::Mouse) -> MouseFrpCallbackHandles {
        mouse::bind_frp_to_mouse(&self.dom.shape(),frp,&self.mouse.mouse_manager)
    }

    pub fn camera(&self) -> &Camera2d {
        &self.views.main.camera
    }

    pub fn new_symbol(&self) -> Symbol {
        let symbol = self.symbols.new();
        self.views.main.add(&symbol);
        symbol
    }

    pub fn symbols(&self) -> &SymbolRegistry {
        &self.symbols
    }

    fn handle_mouse_events(&self) {
        let target = Target::from_internal(self.mouse.hover_ids.get());
        if target != self.mouse.target.get() {
            self.mouse.target.set(target);
            match target {
                Target::Background => {}
                Target::Symbol {symbol_id, instance_id} => {
                    let symbol = self.symbols.index(symbol_id as usize);
                    symbol.dispatch_event(&DynEvent::new(()));
                    // println!("{:?}",target);
                    // TODO: finish events sending, including OnOver and OnOut.
                }
            }
        }
    }

    fn update_shape(&self) {
        if self.dirty.shape.check_all() {
            let screen = self.dom.shape().current();
            self.resize_canvas(&self.dom.shape());
            for view in &*self.views.all.borrow() {
                view.upgrade().for_each(|v| v.camera.set_screen(screen.width(), screen.height()))
            }
//            self.camera.set_screen(screen.width(), screen.height());
            self.renderer.reload_composer();
            self.dirty.shape.unset_all();
        }
    }

    fn update_symbols(&self) {
        if self.dirty.symbols.check_all() {
            self.symbols.update();
            self.dirty.symbols.unset_all();
        }
    }

    fn update_camera(&self) {
        // Updating camera for DOM layers. Please note that DOM layers cannot use multi-camera
        // setups now, so we are using here the main camera only.
        let camera  = self.camera();
        let changed = camera.update();
        if changed {
            self.symbols.set_camera(camera);
            self.dom.layers.front.update_view_projection(camera);
            self.dom.layers.back.update_view_projection(camera);
        }

        // Updating all other cameras (the main camera was already updated, so it will be skipped).
        for view in &*self.views.all() {
            view.upgrade().for_each(|v| v.camera.update());
        }
    }

    /// Resize the underlying canvas. This function should rather not be called
    /// directly. If you want to change the canvas size, modify the `shape` and
    /// set the dirty flag.
    fn resize_canvas(&self, shape:&web::dom::Shape) {
        let screen = shape.current();
        let canvas = shape.current().device_pixels();
        group!(self.logger,"Resized to {screen.width()}px x {screen.height()}px.", {
            self.dom.layers.canvas.set_attribute("width",  &canvas.width().to_string()).unwrap();
            self.dom.layers.canvas.set_attribute("height", &canvas.height().to_string()).unwrap();
            self.context.viewport(0,0,canvas.width() as i32, canvas.height() as i32);
        });
    }
}

impl<'t> From<&'t SceneData> for &'t display::object::Node {
    fn from(scene:&'t SceneData) -> Self {
        &scene.display_object
    }
}



// =============
// === Scene ===
// =============

#[derive(Clone,Debug)]
pub struct Scene {
    no_mut_access : SceneData
}

impl CloneRef for Scene {
    fn clone_ref(&self) -> Self {
        let no_mut_access = self.no_mut_access.clone_ref();
        Self {no_mut_access}
    }
}

impl Scene {
    pub fn new<OnMut:Fn()+Clone+'static>
    (parent_dom:&HtmlElement, logger:Logger, stats:&Stats, on_mut:OnMut) -> Self {
        let no_mut_access = SceneData::new(parent_dom,logger,stats,on_mut);
        let this = Self {no_mut_access};
        this.no_mut_access.shapes.rc.borrow_mut().scene = Some(this.clone_ref()); // FIXME ugly
        this
    }
}

impl AsRef<SceneData> for Scene {
    fn as_ref(&self) -> &SceneData {
        &self.no_mut_access
    }
}

impl std::borrow::Borrow<SceneData> for Scene {
    fn borrow(&self) -> &SceneData {
        &self.no_mut_access
    }
}

impl Deref for Scene {
    type Target = SceneData;
    fn deref(&self) -> &Self::Target {
        &self.no_mut_access
    }
}

impl Scene {
    pub fn update(&self) {
        group!(self.logger, "Updating.", {
            self.display_object.update_with(self);
            self.update_shape();
            self.update_symbols();
            self.update_camera();
            self.handle_mouse_events();
        })
    }
}

impl<'t> From<&'t Scene> for &'t display::object::Node {
    fn from(scene:&'t Scene) -> Self {
        &scene.display_object
    }
}
