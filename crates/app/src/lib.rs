use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use diorama_core::schema::DioramaScene;
use diorama_renderer::state::RendererState;

thread_local! {
    static ACTIVE_RENDERER: RefCell<Option<Rc<RefCell<RendererState>>>> = const { RefCell::new(None) };
}

/// Replace only the scene mesh; keep the GPU device, controls and render loop.
#[wasm_bindgen]
pub fn load_scene(json: &str) -> Result<(), JsValue> {
    let scene: DioramaScene = serde_json::from_str(json)
        .map_err(|err| JsValue::from_str(&format!("Invalid scene: {err}")))?;
    ACTIVE_RENDERER.with(|active| {
        let active = active.borrow();
        let renderer = active.as_ref().ok_or_else(|| JsValue::from_str("Renderer is not ready"))?;
        renderer.borrow_mut().set_scene(&scene);
        Ok(())
    })
}

const DEFAULT_SCENE_JSON: &str = include_str!("../../../assets/default_scene.json");

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

    let scene: DioramaScene = serde_json::from_str(DEFAULT_SCENE_JSON)
        .expect("Failed to parse default scene JSON");

    console_log!("Initializing WebGPU renderer...");
    let (width, height) = diorama_core::viewport::render_size(
        canvas.client_width() as f64, canvas.client_height() as f64, window.device_pixel_ratio());
    canvas.set_width(width);
    canvas.set_height(height);
    let state = RendererState::new(canvas.clone(), &scene).await;
    let (sw, sh) = state.surface_size();
    console_log!("Renderer initialized! surface={}x{}, indices={}", sw, sh, state.mesh_index_count());

    let state = Rc::new(RefCell::new(state));

    ACTIVE_RENDERER.with(|active| *active.borrow_mut() = Some(state.clone()));

    // Pointer events support mouse, touch, and pen with the same gesture state.
    let gesture = Rc::new(RefCell::new(diorama_core::gesture::Gesture::default()));
    {
        let gesture = gesture.clone();
        let target = canvas.clone();
        let closure = Closure::<dyn FnMut(_)>::new(move |e: web_sys::PointerEvent| {
            if e.button() != 0 { return; }
            e.prevent_default();
            let _ = target.focus();
            if target.set_pointer_capture(e.pointer_id()).is_ok() {
                gesture.borrow_mut().start(e.pointer_id(), e.client_x(), e.client_y());
            }
        });
        canvas.add_event_listener_with_callback("pointerdown", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }
    {
        let gesture = gesture.clone();
        let state = state.clone();
        let closure = Closure::<dyn FnMut(_)>::new(move |e: web_sys::PointerEvent| {
            use diorama_core::gesture::Motion;
            let motion = gesture.borrow_mut().update(e.pointer_id(), e.client_x(), e.client_y());
            let mut state = state.borrow_mut();
            match motion {
                Some(Motion::Orbit(dx, dy)) => state.camera.rotate(dx, dy),
                Some(Motion::TwoFinger { .. }) => unreachable!("two-finger motion is sampled in the render loop"),
                None => {}
            }
        });
        canvas.add_event_listener_with_callback("pointermove", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }
    for event in ["pointerup", "pointercancel", "lostpointercapture"] {
        let gesture = gesture.clone();
        let closure = Closure::<dyn FnMut(_)>::new(move |e: web_sys::PointerEvent| {
            gesture.borrow_mut().end(e.pointer_id());
        });
        canvas.add_event_listener_with_callback(event, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    // Keyboard controls are scoped to the focused canvas, so forms keep typing.
    let keys = Rc::new(RefCell::new([false; 4]));
    for (event, pressed) in [("keydown", true), ("keyup", false)] {
        let keys = keys.clone();
        let closure = Closure::<dyn FnMut(_)>::new(move |e: web_sys::KeyboardEvent| {
            if pressed && (e.ctrl_key() || e.meta_key() || e.alt_key()) { return; }
            let index = match e.key().to_ascii_lowercase().as_str() {
                "w" => 0, "a" => 1, "s" => 2, "d" => 3, _ => return,
            };
            e.prevent_default();
            keys.borrow_mut()[index] = pressed;
        });
        canvas.add_event_listener_with_callback(event, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }
    for target in [canvas.as_ref() as &web_sys::EventTarget, window.as_ref()] {
        let keys = keys.clone();
        let closure = Closure::<dyn FnMut()>::new(move || *keys.borrow_mut() = [false; 4]);
        target.add_event_listener_with_callback("blur", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    // Trackpad scrolling pans; trackpad pinch and Ctrl+wheel retain zoom.
    {
        let state = state.clone();
        let target = canvas.clone();
        let closure = Closure::<dyn FnMut(_)>::new(move |e: web_sys::WheelEvent| {
            e.prevent_default();
            let height = target.client_height().max(1) as f32;
            let unit = match e.delta_mode() { 1 => 16.0, 2 => height, _ => 1.0 };
            let dx = e.delta_x() as f32 * unit;
            let dy = e.delta_y() as f32 * unit;
            if e.ctrl_key() || e.meta_key() {
                state.borrow_mut().camera.zoom(dy);
            } else {
                state.borrow_mut().camera.pan(dx, dy, height);
            }
        });
        let options = web_sys::AddEventListenerOptions::new();
        options.set_passive(false);
        canvas.add_event_listener_with_callback_and_add_event_listener_options(
            "wheel", closure.as_ref().unchecked_ref(), &options,
        )?;
        closure.forget();
    }

    // --- Render loop ---
    let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();

    let state_loop = state.clone();
    let mut last_frame = js_sys::Date::now();
    *g.borrow_mut() = Some(Closure::new(move || {
        let now = js_sys::Date::now();
        let dt = ((now - last_frame) / 1000.0).clamp(0.0, 0.05) as f32;
        last_frame = now;
        if let Some(diorama_core::gesture::Motion::TwoFinger { dx, dy, ratio, rotation }) =
            gesture.borrow_mut().take_two_finger_motion()
        {
            let mut renderer = state_loop.borrow_mut();
            // Direct manipulation: the scene follows the fingers.
            renderer.camera.pan(-dx, -dy, canvas.client_height().max(1) as f32);
            renderer.camera.azimuth -= rotation;
            renderer.camera.distance = (renderer.camera.distance * ratio)
                .clamp(5.0, renderer.camera.max_distance);
        }
        let held = *keys.borrow();
        let dx = (held[3] as i32 - held[1] as i32) as f32;
        let dy = (held[2] as i32 - held[0] as i32) as f32;
        let length = (dx * dx + dy * dy).sqrt().max(1.0);
        let height = canvas.client_height().max(1) as f32;
        state_loop.borrow_mut().camera.pan(
            dx / length * height * 0.5 * dt,
            dy / length * height * 0.5 * dt, height);
        // Layout changes and moving between displays do not always fire resize.
        // Reallocate GPU targets only when the bounded backing size changes.
        let (width, height) = diorama_core::viewport::render_size(
            canvas.client_width() as f64, canvas.client_height() as f64, window.device_pixel_ratio());
        if state_loop.borrow().surface_size() != (width, height) {
            canvas.set_width(width);
            canvas.set_height(height);
            state_loop.borrow_mut().resize(width, height);
        }
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
