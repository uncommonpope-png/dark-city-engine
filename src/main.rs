use dark_city_engine::{Game, input::{self, get_input_state}};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{console, window, HtmlCanvasElement, CanvasRenderingContext2d};
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

    // Draw 2D fallback immediately so user sees SOMETHING
    if let Ok(Some(ctx)) = canvas.get_context("2d") {
        let ctx: CanvasRenderingContext2d = ctx.unchecked_into();
        ctx.set_fill_style(&JsValue::from_str("#0a0a1a"));
        ctx.fill_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
        ctx.set_fill_style(&JsValue::from_str("#ff4488"));
        ctx.set_font("24px monospace");
        ctx.fill_text("Dark City Engine - 2D Fallback", 20.0, 40.0).ok();
        // Draw some buildings as colored rectangles
        let colors = ["#ff4488","#22ddff","#ffdd44","#44ff88","#8844ff","#ff8800","#4488ff","#00ff66"];
        let w = canvas.width() as f64;
        let h = canvas.height() as f64;
        for x in 0..8 {
            for z in 0..8 {
                let bx = 50.0 + x as f64 * 45.0;
                let bz = h - 100.0 - z as f64 * 40.0;
                let bh = 20.0 + ((x * 7 + z * 13) % 6) as f64 * 20.0;
                let col = colors[(x * 3 + z * 7) % 8];
                ctx.set_fill_style(&JsValue::from_str(col));
                ctx.fill_rect(bx, bz - bh, 30.0, bh).ok();
            }
        }
        console::log_1(&"2D fallback drawn!".into());
    }

    console::log_1(&"Creating WebGPU renderer...".into());
    let mut game = match Game::new(canvas.clone()).await {
        Ok(g) => g,
        Err(e) => {
            console::error_1(&JsValue::from_str(&format!("Engine init failed: {}", e)));
            return Ok(());
        }
    };
    console::log_1(&"WebGPU renderer ready!".into());

    // Pointer lock
    let canvas2 = canvas.clone();
    let click_h = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |_| {
        if canvas2.pointer_lock_element().is_none() {
            let _ = canvas2.request_pointer_lock();
        }
    });
    canvas.add_event_listener_with_callback("click", click_h.as_ref().unchecked_ref())?;
    click_h.forget();

    // Main loop - try WebGPU first, fall back to 2D
    let f = Rc::new(RefCell::new(None));
    let g = f.clone();
    *g.borrow_mut() = Some(Closure::new(move || {
        let inp = get_input_state();
        let speed = 5.0 * game.time().delta;

        // WebGPU render
        if let Err(e) = game.tick() {
            console::error_1(&JsValue::from_str(&format!("WebGPU tick failed: {}", e)));
        }

        request_animation_frame(f.borrow().as_ref().unwrap());
    }));
    request_animation_frame(g.borrow().as_ref().unwrap());
    Ok(())
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    window().unwrap().request_animation_frame(f.as_ref().unchecked_ref()).unwrap();
}
