#![allow(missing_docs)]

//! NOTE
//! This file is under a heavy development. It contains commented lines of code and some code may
//! be of poor quality. Expect drastic changes.

use ensogl::prelude::*;
use ensogl::traits::*;

use ensogl::data::color::*;
use ensogl::display;
use ensogl::display::Sprite;
use ensogl::display::navigation::navigator::Navigator;
use ensogl::display::shape::*;
use ensogl::display::shape::primitive::system::ShapeSystemDefinition;
use ensogl::display::shape::Var;
use ensogl::display::world::*;
use ensogl::system::web;
use graph::component::node;
use graph::component::node::Node;
use graph::component::node::WeakNode;
use graph::component::cursor;
use graph::component::cursor::Cursor;
use nalgebra::Vector2;
use shapely::shared;
use std::any::TypeId;
use wasm_bindgen::prelude::*;
use ensogl::control::io::mouse::MouseManager;
use enso_frp::{frp, Position};
use enso_frp::Mouse;
use ensogl::control::io::mouse;
use enso_frp::core::node::class::EventEmitterPoly;
use ensogl_system_web::StyleSetter;
use ensogl::display::layout::alignment;
use wasm_bindgen::JsCast;
use ensogl::display::scene;
use ensogl::display::scene::{Scene, MouseTarget};
use ensogl::gui::component::StrongRef;
use ensogl::gui::component::WeakRef;


#[wasm_bindgen]
#[allow(dead_code)]
pub fn run_example_shapes() {
    web::forward_panic_hook_to_console();
    web::set_stdout();
    web::set_stack_trace_limit();
    init(&World::new(&web::get_html_element_by_id("root").unwrap()));
}

fn mouse_pointer() -> AnyShape {
    let radius  = 10.px();
    let side    = &radius * 2.0;
    let width   = Var::<Distance<Pixels>>::from("input_selection_size.x");
    let height  = Var::<Distance<Pixels>>::from("input_selection_size.y");
    let pointer = Rect((&side + width.abs(),&side + height.abs()))
        .corners_radius(radius)
        .translate((-&width/2.0, -&height/2.0))
        .translate(("input_position.x","input_position.y"))
        .fill(Srgba::new(0.0,0.0,0.0,0.3));
    pointer.into()
}


use ensogl::control::event_loop::RawAnimationLoop;
use ensogl::control::event_loop::AnimationLoop;
use ensogl::control::event_loop::TimeInfo;
use ensogl::control::event_loop::FixedFrameRateSampler;
use ensogl::animation::physics::inertia::DynInertiaSimulator;
use ensogl::data::OptVec;
use ensogl::display::object::Id;
use im_rc as im;


#[derive(Clone,CloneRef,Debug,Default)]
pub struct NodeSet {
    data : Rc<RefCell<HashMap<Id,Node>>>
}

impl NodeSet {
    pub fn borrow(&self) -> Ref<HashMap<Id,Node>> {
        self.data.borrow()
    }

    pub fn take(&self) -> HashMap<Id,Node> {
        mem::take(&mut *self.data.borrow_mut())
    }

    pub fn insert(&self, node:Node) {
        self.data.borrow_mut().insert(node.id(),node);
    }

    pub fn remove(&self, node:&Node) {
        self.data.borrow_mut().remove(&node.id());
    }

    pub fn contains(&self, node:&Node) -> bool {
        self.get(node.id()).is_some()
    }

    pub fn get(&self, id:Id) -> Option<Node> {
        self.data.borrow().get(&id).map(|t| t.clone_ref())
    }
}



#[derive(Clone,CloneRef,Debug,Default)]
pub struct WeakNodeSet {
    data : Rc<RefCell<HashMap<Id,WeakNode>>>
}

impl WeakNodeSet {
    pub fn borrow(&self) -> Ref<HashMap<Id,WeakNode>> {
        self.data.borrow()
    }

    pub fn take(&self) -> HashMap<Id,WeakNode> {
        mem::take(&mut *self.data.borrow_mut())
    }

    pub fn for_each_taken<F:Fn(Node)>(&self,f:F) {
        self.take().into_iter().for_each(|(_,node)| { node.upgrade().for_each(|n| f(n)) })
    }

    pub fn insert(&self, node:&Node) {
        self.data.borrow_mut().insert(node.id(),node.downgrade());
    }

    pub fn contains(&self, node:&Node) -> bool {
        self.get(node.id()).is_some()
    }

    pub fn get(&self, id:Id) -> Option<Node> {
        self.data.borrow().get(&id).and_then(|t| t.upgrade())
    }
}


#[derive(Clone,CloneRef,Debug,Default,Shrinkwrap)]
pub struct WeakNodeSelectionSet {
    data : WeakNodeSet
}

impl WeakNodeSelectionSet {
    pub fn deselect_all(&self) {
        self.for_each_taken(|node| node.events.deselect.event.emit(()));
    }
}


fn init(world: &World) {
    let scene  = world.scene();
    let camera = scene.camera();
    let screen = camera.screen();
    let navigator = Navigator::new(&scene,&camera);


    let cursor = Cursor::new();

    world.add_child(&cursor);


    web::body().set_style_or_panic("cursor","none");

    let mouse = &scene.mouse.frp;

    let node_set = NodeSet::default();

    let selected_nodes = WeakNodeSelectionSet::default();

    let selected_nodes2 = selected_nodes.clone_ref();

    frp! {
        mouse_down_position    = mouse.position.sample        (&mouse.on_down);
        selection_zero         = source::<Position>           ();
        selection_size_down    = mouse.position.map2          (&mouse_down_position,|m,n|{m-n});
        selection_size_if_down = selection_size_down.gate     (&mouse.is_down);
        selection_size_on_down = selection_zero.sample        (&mouse.on_down);
        selection_size         = selection_size_if_down.merge (&selection_size_on_down);


        mouse_down_target      = mouse.on_down.map            (enclose!((scene) move |_| scene.mouse.target.get()));


        node_mouse_down = source::<Option<WeakNode>> ();

        add_node = source::<()> ();
        remove_selected_nodes = source::<()> ();

        foo = add_node.map2(&mouse.position, enclose!((node_set,node_mouse_down,world) move |_,pos| {
            let node = Node::new();
            let weak_node = node.downgrade();
            let ttt = node.view.events.mouse_down.map("foo",enclose!((node_mouse_down) move |_| {
                node_mouse_down.event.emit(Some(weak_node.clone_ref()))
            }));
//
            world.add_child(&node);
            node.mod_position(|t| {
                t.x += pos.x as f32;
                t.y += pos.y as f32;
            });

            node_set.insert(node);

        }));

        bar = remove_selected_nodes.map(enclose!((selected_nodes2) move |_| {
            selected_nodes2.for_each_taken(|node| {
                node_set.remove(&node);
                // mem::take(&mut *node_set.data.borrow_mut());
                println!("REMOVE, {:?}", node_set.data.borrow().len());
            })
        }));

//        nodes_update = nodes.map2(&new_node, |node_set,new_node| {
//            new_node.for_each_ref(|node| {
//                node_set.vec.borrow_mut().insert(node.clone_ref());
//            })
//        });


        baz = node_mouse_down.map(move |opt_node| {
            opt_node.for_each_ref(|weak_node| {
                weak_node.upgrade().map(|node| {
                    let is_selected = selected_nodes2.contains(&node);
                    selected_nodes2.deselect_all();
                    node.events.select.event.emit(());
                    selected_nodes2.insert(&node);
                    println!("is_selected {}", is_selected);
                })
            })
        })


    }



    mouse.on_down.map("cursor_press", enclose!((cursor) move |p| {
        cursor.events.press.event.emit(());
    }));

    mouse.on_up.map("cursor_release", enclose!((cursor) move |p| {
        cursor.events.release.event.emit(());
    }));



    mouse.position.map("cursor_position", enclose!((cursor) move |p| {
        cursor.set_position(Vector2::new(p.x as f32,p.y as f32));
    }));

    selection_size.map("cursor_size", enclose!((cursor) move |p| {
        cursor.set_selection_size(Vector2::new(p.x as f32,p.y as f32));
    }));



    let selected_nodes2 = selected_nodes.clone_ref();
    mouse_down_target.map("mouse_down_target", enclose!((scene) move |target| {
        match target {
            display::scene::Target::Background => {
                selected_nodes2.deselect_all();
            }
            display::scene::Target::Symbol {symbol_id, instance_id} => {
                scene.shapes.get_mouse_target(&(*instance_id as usize)).for_each(|target| {
                    target.mouse_down().for_each(|t| t.event.emit(()));
                })
            }
        }
        println!("SELECTING {:?}", target);
    }));


    let add_node_ref = add_node.clone_ref();
    let remove_selected_nodes_ref = remove_selected_nodes.clone_ref();
    let world2 = world.clone_ref();
    let c: Closure<dyn Fn(JsValue)> = Closure::wrap(Box::new(move |val| {
        let val = val.unchecked_into::<web_sys::KeyboardEvent>();
        let key = val.key();
        if      key == "n"         { add_node_ref.event.emit(()) }
        else if key == "Backspace" { remove_selected_nodes_ref.event.emit(()) }
        else if key == "p" {
            selected_nodes.for_each_taken(|node| {
                world2.scene().remove_child(&node);
                println!("REMOVE CH");
            })
        }
    }));
    web::document().add_event_listener_with_callback("keydown",c.as_ref().unchecked_ref()).unwrap();
    c.forget();




    // FIRST NODE!
//    add_node.event.emit(());




    let mut iter:i32 = 0;
    let mut time:i32 = 0;
    let mut was_rendered = false;
    let mut loader_hidden = false;

    let world_clone = world.clone_ref();
    world.on_frame(move |_| {
        let _keep_alive = &world_clone;
        let _keep_alive = &navigator;
        if was_rendered && !loader_hidden {
            web::get_element_by_id("loader").map(|t| {
                t.parent_node().map(|p| {
                    p.remove_child(&t).unwrap()
                })
            }).ok();
            loader_hidden = true;
        }
        was_rendered = true;
    }).forget();
}



// ================
// === FRP Test ===
// ================

//#[allow(unused_variables)]
//pub fn frp_test (callback: Box<dyn Fn(f32,f32)>) -> MouseManager {
//    let document        = web::document();
//    let mouse_manager   = MouseManager::new(&document);
//    let mouse           = Mouse::new();
//
//    frp! {
//        mouse_down_position    = mouse.position.sample       (&mouse.on_down);
//        mouse_position_if_down = mouse.position.gate         (&mouse.is_down);
//        final_position_ref     = recursive::<Position>       ();
//        pos_diff_on_down       = mouse_down_position.map2    (&final_position_ref,|m,f|{m-f});
//        final_position         = mouse_position_if_down.map2 (&pos_diff_on_down  ,|m,f|{m-f});
//        debug                  = final_position.sample       (&mouse.position);
//    }
//    final_position_ref.initialize(&final_position);
//
//    // final_position.event.display_graphviz();
//
////    trace("X" , &debug.event);
//
////    final_position.map("foo",move|p| {callback(p.x as f32,-p.y as f32)});
//
//    let target = mouse.position.event.clone_ref();
//    let handle = mouse_manager.on_move.add(move |event:&mouse::OnMove| {
//        target.emit(Position::new(event.client_x(),event.client_y()));
//    });
//    handle.forget();
//
//    let target = mouse.on_down.event.clone_ref();
//    let handle = mouse_manager.on_down.add(move |event:&mouse::OnDown| {
//        target.emit(());
//    });
//    handle.forget();
//
//    let target = mouse.on_up.event.clone_ref();
//    let handle = mouse_manager.on_up.add(move |event:&mouse::OnUp| {
//        target.emit(());
//    });
//    handle.forget();
//
//    mouse_manager
//}
