use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use std::cell::RefCell;
use std::rc::Rc;
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response, console};
mod nes;

#[derive(Debug, Serialize, Deserialize)]
pub struct Branch {
    pub name: String,
    pub commit: Commit,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Commit {
    pub sha: String,
    pub commit: CommitDetails,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitDetails {
    pub author: Signature,
    pub committer: Signature,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Signature {
    pub name: String,
    pub email: String,
}

async fn load_rom() ->Result<Vec<u8>, JsValue> {
    let mut opts = RequestInit::new();
    opts.method("GET");

    let url = "nestest.nes";

    let request = Request::new_with_str_and_init(&url, &opts)?;

    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;

    // `resp_value` is a `Response` object.
    assert!(resp_value.is_instance_of::<Response>());
    let resp: Response = resp_value.dyn_into().unwrap();
    let array_buffer = JsFuture::from(resp.array_buffer()?).await?;
    let blob = js_sys::Uint8Array::new(&array_buffer).to_vec();

    return Ok(blob);
}

// When the `wee_alloc` feature is enabled, this uses `wee_alloc` as the global
// allocator.
//
// If you don't want to use `wee_alloc`, you can safely delete this.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    window()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}

struct Scene {
    count: u32
}

#[inline]
fn put_pixel(data: &mut wasm_bindgen::Clamped<Vec<u8>>, x: usize, y: usize, color: u32) {
    let pos: usize = ((y * 256) + x) * 4;
    data[pos + 0] = 0xFF;
    data[pos + 1] = 0x00;
    data[pos + 2] = 0x00;
    data[pos + 3] = 0xFF;
}

fn drawloop(scene: &mut Scene, data: &mut wasm_bindgen::Clamped<Vec<u8>>, context: &web_sys::CanvasRenderingContext2d) {
    for y in 0..224 {
        for x in 0..256 {
            put_pixel(data, x, y, 0xFF0000);
        }
    }
    let buffer = web_sys::ImageData::new_with_u8_clamped_array_and_sh(wasm_bindgen::Clamped(data), 256, 240).unwrap();
    // context.put_image_data_with_dirty_x_and_dirty_y_and_dirty_width_and_dirty_height(buffer, 0.0, 0.0, 0.0, 0.0, 256.0, 224.0).unwrap();
    context.put_image_data(&buffer, 0.0, 0.0).unwrap();

    scene.count = (scene.count + 1) % 0xFFFF;

    console::log_2(&"count = ".into(), &scene.count.into());
}

// This is like the `main` function, except for JavaScript.
#[wasm_bindgen(start)]
pub async fn main_js() -> Result<(), JsValue> {
    // This provides better error messages in debug mode.
    // It's disabled in release mode so it doesn't bloat up the file size.
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    let document = web_sys::window().unwrap().document().unwrap();
    let htmlbody = document.body().expect("document should have a body");

    // Manufacture the element we're gonna append
    let canvas = document.create_element("canvas")?;

    htmlbody.append_child(&canvas)?;

    let canvas: web_sys::HtmlCanvasElement = canvas
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| ())
        .unwrap();

    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    let romdata = load_rom().await?;
    let f = Rc::new(RefCell::new(None));
    let g = f.clone();
    let mut data = context.get_image_data(0.0, 0.0, 256.0, 240.0).unwrap().data().clone();
    let nes_rom = nes::rom::load_nes(&romdata);
    let mut cpu = nes::cpu::new_cpu();
    let mut ppu = nes::ppu::new_ppu(&nes_rom.character_rom.data);
    let mut mem = nes::memory::new_memory(&nes_rom.program_rom.data);
    let mut scene = Scene{count: 0};
    let mut ppu_cycle = 2;

    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        let mut vmem = nes::vmem::new_vmem(&mut mem, &mut ppu);
        nes::cpu::run(&mut cpu, &mut vmem);
        ppu_cycle = ppu_cycle - 1;
        if ppu_cycle <= 0 {
            nes::ppu::run(&mut vmem.ppu);
            ppu_cycle = 2;
        }
        
        if nes::ppu::is_draw_timing(vmem.ppu) {
            nes::ppu::draw_to_canvas(&mut data, &mut vmem.ppu);
            drawloop(&mut scene, &mut data, &context);
        }

        // Schedule ourself for another requestAnimationFrame callback.
        request_animation_frame(f.borrow().as_ref().unwrap());
    }) as Box<dyn FnMut()>));

    request_animation_frame(g.borrow().as_ref().unwrap());
    Ok(())
}
