use dark_city_engine::{Game, input::{self, get_input_state}};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{console, window, HtmlCanvasElement};
use std::rc::Rc;
use std::cell::RefCell;

#[wasm_bindgen(start)]
pub async fn main_wasm() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    console::log_1(&"Dark City Engine starting...".into());

    let _ = input::InputState::setup_event_listeners();

    let window = window().unwrap();
    let document = window.document().unwrap();
    let canvas = document.get_element_by_id("dark_city_canvas")
        .unwrap().dyn_into::<HtmlCanvasElement>().unwrap();

    console::log_1(&"Creating renderer...".into());
    let mut game = match Game::new(canvas.clone()).await {
        Ok(g) => g,
        Err(e) => {
            console::error_1(&JsValue::from_str(&format!("Engine init failed: {}", e)));
            return Ok(());
        }
    };
    console::log_1(&"Engine ready!".into());

    // Pointer lock on click
    let canvas2 = canvas.clone();
    let click_h = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |_| {
        if canvas2.pointer_lock_element().is_none() {
            let _ = canvas2.request_pointer_lock();
        }
    });
    canvas.add_event_listener_with_callback("click", click_h.as_ref().unchecked_ref())?;
    click_h.forget();

    // Main loop
    let f = Rc::new(RefCell::new(None));
    let g = f.clone();
    *g.borrow_mut() = Some(Closure::new(move || {
        let inp = get_input_state();
        let speed = 5.0 * game.time().delta;
        if let Some(cam) = game.renderer_mut().camera_mut() {
            let fwd = (cam.target - cam.eye).normalize();
            let right = fwd.cross(cam.up).normalize();
            if inp.is_key_down("w") { cam.eye += fwd * speed; cam.target += fwd * speed; }
            if inp.is_key_down("s") { cam.eye -= fwd * speed; cam.target -= fwd * speed; }
            if inp.is_key_down("a") { cam.eye -= right * speed; cam.target -= right * speed; }
            if inp.is_key_down("d") { cam.eye += right * speed; cam.target += right * speed; }
            if canvas.pointer_lock_element().is_some() {
                let sens = 0.002;
                let yaw = -inp.mouse_dx * sens;
                let pitch = -inp.mouse_dy * sens;
                let dir = (cam.target - cam.eye).normalize();
                let r = dir.cross(cam.up).normalize();
                let u = r.cross(dir).normalize();
                let rad = (cam.target - cam.eye).length();
                let nd = (dir + r * yaw + u * pitch).normalize();
                cam.eye = cam.target - nd * rad;
            }
            game.renderer_mut().update_camera_buffer();
        }
        if let Err(e) = game.tick() {
            console::error_1(&JsValue::from_str(&format!("Tick: {}", e)));
        }
        request_animation_frame(f.borrow().as_ref().unwrap());
    }));
    request_animation_frame(g.borrow().as_ref().unwrap());
    Ok(())
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    window().unwrap().request_animation_frame(f.as_ref().unchecked_ref()).unwrap();
}
