use macroquad::prelude::*;

fn configuration() -> Conf {
    Conf {
        window_title: "Macroquad depth patch integration".to_owned(),
        window_width: 160,
        window_height: 160,
        ..Default::default()
    }
}

fn check_pixel(label: &str, image: &Image, expected: [u8; 3]) {
    let offset = ((image.height as usize / 2) * image.width as usize + image.width as usize / 2) * 4;
    let actual = &image.bytes[offset..offset + 3];
    assert!(actual.iter().zip(expected).all(|(actual, expected)| actual.abs_diff(expected) <= 2),
        "{label}: actual={actual:?}, expected={expected:?}");
    println!("PASS {label}: rgb={actual:?}");
}

#[macroquad::main(configuration)]
async fn main() {
    let scene = render_target_ex(64, 64, RenderTargetParams { depth: true, sample_count: 1 });
    let canvas = render_target(64, 64);
    let camera_3d = Camera3D {
        position: vec3(0.0, 0.0, 5.0),
        target: Vec3::ZERO,
        up: Vec3::Y,
        render_target: Some(scene.clone()),
        ..Default::default()
    };
    let mut camera_2d = Camera2D::from_display_rect(Rect::new(0.0, 0.0, 64.0, 64.0));
    camera_2d.render_target = Some(scene.clone());
    let mut canvas_camera = Camera2D::from_display_rect(Rect::new(0.0, 0.0, 64.0, 64.0));
    canvas_camera.render_target = Some(canvas.clone());
    let red = Color::new(1.0, 0.0, 0.0, 1.0);
    let green = Color::new(0.0, 1.0, 0.0, 1.0);
    let blue = Color::new(0.0, 0.0, 1.0, 1.0);
    let yellow = Color::new(1.0, 1.0, 0.0, 1.0);
    let mut checks = 0;
    for frame in 0..10 {
        set_camera(&camera_3d);
        clear_background(BLACK);
        draw_cube(vec3(0.0, 0.0, 1.0), vec3(2.0, 2.0, 0.2), None, green);
        draw_cube(Vec3::ZERO, vec3(3.0, 3.0, 0.2), None, red);
        set_default_camera();
        check_pixel("3D opaque occlusion", &scene.texture.get_texture_data(), [0, 255, 0]);
        checks += 1;

        set_camera(&camera_2d);
        draw_rectangle(0.0, 0.0, 64.0, 64.0, yellow);
        set_default_camera();
        check_pixel("2D overlay ignores previous depth", &scene.texture.get_texture_data(), [255, 255, 0]);
        checks += 1;

        set_camera(&camera_3d);
        clear_background(BLACK);
        draw_cube(Vec3::ZERO, vec3(3.0, 3.0, 0.2), None, blue);
        set_default_camera();
        check_pixel("3D clear after 2D restores visibility", &scene.texture.get_texture_data(), [0, 0, 255]);
        checks += 1;

        set_camera(&canvas_camera);
        clear_background(BLACK);
        draw_texture(&scene.texture, 0.0, 0.0, WHITE);
        set_default_camera();
        check_pixel("render target sampled into second target", &canvas.texture.get_texture_data(), [0, 0, 255]);
        checks += 1;

        set_camera(&canvas_camera);
        clear_background(blue);
        draw_rectangle(0.0, 0.0, 64.0, 64.0, Color::new(1.0, 0.0, 0.0, 0.5));
        set_default_camera();
        check_pixel("2D alpha blending", &canvas.texture.get_texture_data(), [128, 0, 127]);
        checks += 1;

        set_camera(&canvas_camera);
        clear_background(BLACK);
        draw_text("Depth", 2.0, 32.0, 22.0, WHITE);
        set_default_camera();
        let text = canvas.texture.get_texture_data();
        let white = text.bytes.chunks_exact(4).filter(|pixel| pixel[0] > 200 && pixel[1] > 200 && pixel[2] > 200).count();
        assert!(white > 20, "text rendering: white={white}");
        println!("PASS text rendering: white={white}");
        checks += 1;

        clear_background(yellow);
        let screen = get_screen_data();
        check_pixel("default framebuffer clear", &screen, [255, 255, 0]);
        checks += 1;
        let error = unsafe { miniquad::native::gl::glGetError() };
        assert_eq!(error, 0, "GL error in frame {frame}");
        next_frame().await;
    }
    println!("RESULT: Macroquad integration {checks} checks, 0 failures, 10 frames");
}
