#[cfg(target_arch = "wasm32")]
use dark_city_engine::{Game, input::{self, InputState, get_input_state}};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::{console, window};
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;
#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub async fn main_wasm() -> Result<(), JsValue> {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();

    console::log_1(&"Dark City Engine Initializing...".into());

    // Set up input event listeners
    input::InputState::setup_event_listeners()?;

    let window = window().expect("should have a window document");
    let document = window.document().expect("should have a document on window");
    let canvas = document.get_element_by_id("dark_city_canvas")
        .expect("should have dark_city_canvas on the document")
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| JsValue::from_str("Can't get canvas element"))?;

    let mut game = Game::new(canvas).await.map_err(|e| JsValue::from_str(&e))?;

    // Enable pointer lock for first-person camera on click
    {
        let canvas_clone = canvas.clone();
        let click_handler = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |_event: web_sys::MouseEvent| {
            if canvas_clone.pointer_lock_element().is_none() {
                let _ = canvas_clone.request_pointer_lock();
            }
        });
        canvas.add_event_listener_with_callback("click", click_handler.as_ref().unchecked_ref())?;
        click_handler.forget();
    }

    let pointer_lock_change = Closure::<dyn FnMut()>::new(move || {
        console::log_1(&"Pointer lock changed".into());
    });
    document.add_event_listener_with_callback("pointerlockchange", pointer_lock_change.as_ref().unchecked_ref())?;
    pointer_lock_change.forget();

    let f = Rc::new(RefCell::new(None));
    let g = f.clone();

    *g.borrow_mut() = Some(Closure::new(move || {
        let input_state = get_input_state();

        // WASD camera movement
        let speed = 5.0 * game.time().delta;
        {
            let camera = game.renderer_mut().camera_mut();
            if let Some(camera) = camera {
                let forward = (camera.target - camera.eye).normalize();
                let right = forward.cross(camera.up).normalize();

                if input_state.is_key_down("w") || input_state.is_key_down("W") {
                    camera.eye += forward * speed;
                    camera.target += forward * speed;
                }
                if input_state.is_key_down("s") || input_state.is_key_down("S") {
                    camera.eye -= forward * speed;
                    camera.target -= forward * speed;
                }
                if input_state.is_key_down("a") || input_state.is_key_down("A") {
                    camera.eye -= right * speed;
                    camera.target -= right * speed;
                }
                if input_state.is_key_down("d") || input_state.is_key_down("D") {
                    camera.eye += right * speed;
                    camera.target += right * speed;
                }

                // Mouse look (only when pointer is locked)
                if canvas.pointer_lock_element().is_some() {
                    let sensitivity = 0.002;
                    let yaw = -input_state.mouse_dx * sensitivity;
                    let pitch = -input_state.mouse_dy * sensitivity;
                    let forward_dir = (camera.target - camera.eye).normalize();
                    let right_dir = forward_dir.cross(camera.up).normalize();
                    let up_dir = right_dir.cross(forward_dir).normalize();

                    // Orbit around target for first-person feel
                    let radius = (camera.target - camera.eye).length();
                    let current_forward = (camera.target - camera.eye).normalize();
                    let new_forward = current_forward + right_dir * yaw + up_dir * pitch;
                    let new_forward = new_forward.normalize();
                    camera.eye = camera.target - new_forward * radius;
                }

                game.renderer_mut().update_camera_buffer();
            }
        }

        if let Err(e) = game.tick() {
            console::error_1(&JsValue::from_str(&format!("Game tick error: {}", e)));
            return;
        }
        request_animation_frame(f.borrow().as_ref().unwrap());
    }));

    request_animation_frame(g.borrow().as_ref().unwrap());

    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    window()
        .expect("window")
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` okay");
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {}
