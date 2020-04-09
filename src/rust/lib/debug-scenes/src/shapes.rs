#![allow(missing_docs)]

//! NOTE
//! This file is under a heavy development. It contains commented lines of code and some code may
//! be of poor quality. Expect drastic changes.

use ensogl::prelude::*;

use ensogl::display::navigation::navigator::Navigator;
use ensogl::display::world::*;
use ensogl::system::web;
use graph_editor::GraphEditor;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use ensogl::display::object::ObjectOps;


#[wasm_bindgen]
#[allow(dead_code)]
pub fn run_example_shapes() {
    web::forward_panic_hook_to_console();
    web::set_stdout();
    web::set_stack_trace_limit();
    init(&World::new(&web::get_html_element_by_id("root").unwrap()));
}


fn init(world: &World) {
    let scene  = world.scene();
    let camera = scene.camera();
    let navigator = Navigator::new(&scene,&camera);

    let graph_editor = GraphEditor::new(world);
    world.add_child(&graph_editor);

    let mut _iter:i32 = 0;
    let mut _time:i32 = 0;
    let mut was_rendered = false;
    let mut loader_hidden = false;

    let add_node_ref = graph_editor.frp.add_node_under_cursor.clone_ref();
    let remove_selected_nodes_ref = graph_editor.frp.remove_selected_nodes.clone_ref();
    let selected_nodes2 = graph_editor.selected_nodes.clone_ref();
    let world2 = world.clone_ref();
    let c: Closure<dyn Fn(JsValue)> = Closure::wrap(Box::new(move |val| {
        let val = val.unchecked_into::<web_sys::KeyboardEvent>();
        let key = val.key();
        if      key == "n"         { add_node_ref.emit(()) }
        else if key == "Backspace" {
            remove_selected_nodes_ref.emit(())
        }
        else if key == "p" {
            selected_nodes2.for_each_taken(|node| {
                world2.scene().remove_child(&node);
            })
        }
    }));
    web::document().add_event_listener_with_callback("keydown",c.as_ref().unchecked_ref()).unwrap();
    c.forget();

    let world_clone = world.clone_ref();
    world.on_frame(move |_| {
        let _keep_alive = &world_clone;
        let _keep_alive = &navigator;
        let _keep_alive = &graph_editor;
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
