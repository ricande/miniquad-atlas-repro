//! Reproduction of macroquad#1006 (text drawn as black rectangles), caused
//! by miniquad 0.4.11's `delete_texture` not clearing its texture-binding
//! cache. See README.md.
//!
//! Every frame, text is drawn into a render target that is read back, then
//! into the window, with font sizes that keep adding glyphs to macroquad's
//! font atlas, so the atlas grows (macroquad deletes its texture and creates
//! a bigger one) at varying points in the frame. Whether a growth lands right
//! after the render-target pass – where the atlas is the last texture bound –
//! depends on the order in which the atlas re-packs its glyphs (a `HashMap`),
//! so the failure shows up in some runs and not in others.
//!
//! `cargo run --release [frames] [--rect]`: exits with 1 and "RESULT: black
//! text" when the render target's text has no white pixels left. `--rect`
//! ends each batch with a plain rectangle (the workaround).

use macroquad::prelude::*;

const W: u32 = 1080;
const H: u32 = 1920;

fn white_pixels(image: &Image) -> usize {
    image
        .bytes
        .chunks(4)
        .filter(|p| p[0] > 200 && p[1] > 200 && p[2] > 200)
        .count()
}

/// (texture bound on the active unit, active unit), read from GL itself.
fn gl_binding() -> (i32, i32) {
    let (mut texture, mut unit) = (0, 0);
    unsafe {
        miniquad::native::gl::glGetIntegerv(0x8069, &mut texture);
        miniquad::native::gl::glGetIntegerv(0x84E0, &mut unit);
    }
    (texture, unit - 0x84C0)
}

fn window_conf() -> Conf {
    Conf {
        window_title: "macroquad text stress".to_owned(),
        window_width: 540,
        window_height: 960,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let frames: u32 = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(300);
    let end_with_rect = std::env::args().any(|a| a == "--rect");
    let renderer = unsafe {
        let p = miniquad::native::gl::glGetString(0x1F01);
        std::ffi::CStr::from_ptr(p as *const std::ffi::c_char).to_string_lossy().into_owned()
    };
    println!("GL renderer: {renderer}");
    let target = render_target(W, H);
    let mut camera = Camera2D::from_display_rect(Rect::new(0.0, 0.0, W as f32, H as f32));
    camera.render_target = Some(target.clone());
    for frame in 1..=frames {
        // A new size now and then, as when text is shrunk to fit.
        let size = 20.0 + (frame % 97) as f32 * 1.37;

        set_camera(&camera);
        clear_background(BLACK);
        draw_text("Your game here", 40.0, 300.0, size * 2.0, WHITE);
        draw_text("Level 1   Score 42", 40.0, 600.0, 56.0, WHITE);
        if end_with_rect {
            draw_rectangle(0.0, 0.0, 1.0, 1.0, Color::new(0.0, 0.0, 0.0, 0.0));
        }
        set_default_camera();
        let after_target = gl_binding();
        let image = target.texture.get_texture_data();

        clear_background(DARKBLUE);
        draw_text("Your game here", 20.0, 150.0, size, WHITE);
        draw_text(&format!("frame {frame}"), 20.0, 300.0, 28.0, WHITE);
        if end_with_rect {
            draw_rectangle(0.0, 0.0, 1.0, 1.0, Color::new(0.0, 0.0, 0.0, 0.0));
        }

        let white = white_pixels(&image);
        if std::env::var_os("DEBUG").is_some() {
            println!("frame {frame}: bound after the target pass {after_target:?}, white {white}");
        }
        if white == 0 {
            println!("RESULT: black text in the render target from frame {frame}");
            std::process::exit(1);
        }
        next_frame().await;
    }
    println!("RESULT: text fine for {frames} frames");
}
