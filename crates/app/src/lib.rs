use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use diorama_renderer::state::RendererState;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format!($($t)*)))
}

#[wasm_bindgen]
pub async fn init_diorama() -> Result<(), JsValue> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document
        .get_element_by_id("diorama-canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()?;

    console_log!("Initializing WebGPU renderer...");
    let state = RendererState::new(canvas.clone()).await;
    let (sw, sh) = state.surface_size();
    console_log!("Renderer initialized! surface={}x{}, indices={}", sw, sh, state.mesh_index_count());

    let state = Rc::new(RefCell::new(state));

    // --- Mouse drag for orbit ---
    let dragging = Rc::new(RefCell::new(false));
    let last_pos = Rc::new(RefCell::new((0i32, 0i32)));

    {
        let dragging = dragging.clone();
        let last_pos = last_pos.clone();
        let closure = Closure::<dyn FnMut(_)>::new(move |e: web_sys::MouseEvent| {
            *dragging.borrow_mut() = true;
            *last_pos.borrow_mut() = (e.client_x(), e.client_y());
        });
        canvas.add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let dragging = dragging.clone();
        let last_pos = last_pos.clone();
        let state = state.clone();
        let closure = Closure::<dyn FnMut(_)>::new(move |e: web_sys::MouseEvent| {
            if !*dragging.borrow() {
                return;
            }
            let (lx, ly) = *last_pos.borrow();
            let dx = e.client_x() - lx;
            let dy = e.client_y() - ly;
            *last_pos.borrow_mut() = (e.client_x(), e.client_y());
            state.borrow_mut().camera.rotate(dx as f32, -dy as f32);
        });
        canvas.add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let dragging = dragging.clone();
        let closure = Closure::<dyn FnMut(_)>::new(move |_e: web_sys::MouseEvent| {
            *dragging.borrow_mut() = false;
        });
        canvas.add_event_listener_with_callback("mouseup", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    // --- Scroll wheel for zoom ---
    {
        let state = state.clone();
        let closure = Closure::<dyn FnMut(_)>::new(move |e: web_sys::WheelEvent| {
            e.prevent_default();
            state.borrow_mut().camera.zoom(e.delta_y() as f32);
        });
        canvas.add_event_listener_with_callback("wheel", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    // --- Render loop ---
    let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();

    let state_loop = state.clone();
    *g.borrow_mut() = Some(Closure::new(move || {
        state_loop.borrow().render();
        request_animation_frame(f.borrow().as_ref().unwrap());
    }));
    request_animation_frame(g.borrow().as_ref().unwrap());

    Ok(())
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    web_sys::window()
        .unwrap()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .unwrap();
}
