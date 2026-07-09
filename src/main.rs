use dark_city_engine::{Game, input::{self, get_input_state}};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{console, window, HtmlCanvasElement};
use std::rc::Rc;
use std::cell::RefCell;

#[wasm_bindgen(start)]
pub async fn main_wasm() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    console::log_1(&"Dark City Engine Initializing...".into());

    // Check WebGPU support
    let nav = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("navigator"));
    let has_gpu = if let Ok(n) = nav {
        js_sys::Reflect::get(&n, &JsValue::from_str("gpu")).is_ok()
    } else { false };

    if !has_gpu {
        let msg = "WebGPU not available. Use Chrome 113+.";
        console::error_1(&JsValue::from_str(msg));
        show_error(msg);
        return Ok(());
    }

    console::log_1(&"WebGPU detected!".into());
    let _ = input::InputState::setup_event_listeners();

    let window = window().unwrap();
    let document = window.document().unwrap();
    let canvas = document.get_element_by_id("dark_city_canvas")
        .unwrap().dyn_into::<HtmlCanvasElement>().unwrap();

    console::log_1(&"Creating Game...".into());
    let mut game = match Game::new(canvas.clone()).await {
        Ok(g) => g,
        Err(e) => { show_error(&format!("Engine error: {}", e)); return Ok(()); }
    };
    console::log_1(&"Game ready!".into());

    let canvas2 = canvas.clone();
    let click_h = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |_| {
        if canvas2.pointer_lock_element().is_none() {
            let _ = canvas2.request_pointer_lock();
        }
    });
    canvas.add_event_listener_with_callback("click", click_h.as_ref().unchecked_ref())?;
    click_h.forget();

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
            return;
        }
        request_animation_frame(f.borrow().as_ref().unwrap());
    }));
    request_animation_frame(g.borrow().as_ref().unwrap());
    Ok(())
}

fn show_error(msg: &str) {
    if let Some(doc) = window().and_then(|w| w.document()) {
        if let Some(el) = doc.get_element_by_id("dark_city_canvas") {
            if let Ok(c) = el.dyn_into::<HtmlCanvasElement>() {
                if let Ok(Some(ctx)) = c.get_context("2d") {
                    let ctx: web_sys::CanvasRenderingContext2d = ctx.unchecked_into();
                    ctx.set_fill_style(&JsValue::from_str("#ff4444"));
                    ctx.set_font("14px monospace");
                    ctx.fill_text(msg, 10.0, 30.0).ok();
                }
            }
        }
    }
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    window().unwrap().request_animation_frame(f.as_ref().unchecked_ref()).unwrap();
}
