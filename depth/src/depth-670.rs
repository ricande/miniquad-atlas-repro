use miniquad::{native::gl::*, *};

#[repr(C)]
struct Uniforms {
    tint: [f32; 4],
    depth: f32,
}

struct Stage {
    context: Box<dyn RenderingBackend>,
    bindings: Bindings,
    write_pipeline: Pipeline,
    no_write_pipeline: Pipeline,
    pass: RenderPass,
}

impl Stage {
    fn new() -> Self {
        let mut context = window::new_rendering_backend();
        unsafe {
            for (label, parameter) in [("renderer", 0x1F01), ("version", GL_VERSION)] {
                let value = std::ffi::CStr::from_ptr(glGetString(parameter).cast());
                println!("GL {label}: {}", value.to_string_lossy());
            }
        }
        let shader = context.new_shader(
            ShaderSource::Glsl {
                vertex: "#version 100\nattribute vec2 position; uniform float depth; void main() { gl_Position = vec4(position, depth, 1.0); }",
                fragment: "#version 100\nprecision mediump float; uniform vec4 tint; void main() { gl_FragColor = tint; }",
            },
            ShaderMeta {
                images: vec![],
                uniforms: UniformBlockLayout { uniforms: vec![
                    UniformDesc::new("tint", UniformType::Float4),
                    UniformDesc::new("depth", UniformType::Float1),
                ] },
            },
        ).unwrap();
        let mut pipeline = |depth_write| context.new_pipeline(
            &[BufferLayout::default()],
            &[VertexAttribute::new("position", VertexFormat::Float2)],
            shader,
            PipelineParams {
                depth_test: Comparison::LessOrEqual,
                depth_write,
                color_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::Value(BlendValue::SourceAlpha),
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                )),
                ..Default::default()
            },
        );
        let write_pipeline = pipeline(true);
        let no_write_pipeline = pipeline(false);
        let vertices: [[f32; 2]; 4] = [[-0.8, -0.8], [0.8, -0.8], [0.8, 0.8], [-0.8, 0.8]];
        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
        let bindings = Bindings {
            vertex_buffers: vec![context.new_buffer(BufferType::VertexBuffer, BufferUsage::Immutable, BufferSource::slice(&vertices))],
            index_buffer: context.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(&indices)),
            images: vec![],
        };
        let color = context.new_render_texture(TextureParams {
            width: 64, height: 64, format: TextureFormat::RGBA8, ..Default::default()
        });
        let depth = context.new_render_texture(TextureParams {
            width: 64, height: 64, format: TextureFormat::Depth, ..Default::default()
        });
        let pass = context.new_render_pass(color, Some(depth));
        Self { context, bindings, write_pipeline, no_write_pipeline, pass }
    }

    fn sample(&mut self, label: &str, writes: bool, workaround: bool, depth: f32) -> [u8; 4] {
        unsafe { glDepthMask(GL_TRUE as u8); }
        self.context.begin_pass(Some(self.pass), PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        self.context.apply_pipeline(&self.write_pipeline);
        self.context.apply_bindings(&self.bindings);
        self.context.apply_uniforms(UniformsSource::table(&Uniforms { tint: [0.0, 1.0, 0.0, 1.0], depth: -0.5 }));
        self.context.draw(0, 6, 1);
        self.context.apply_pipeline(if writes { &self.write_pipeline } else { &self.no_write_pipeline });
        if workaround { unsafe { glDepthMask(GL_FALSE as u8); } }
        let mut enabled = 0;
        let mut mask = 0;
        unsafe {
            glGetIntegerv(GL_DEPTH_TEST, &mut enabled);
            glGetIntegerv(0x0B72, &mut mask);
        }
        self.context.apply_uniforms(UniformsSource::table(&Uniforms { tint: [1.0, 0.0, 0.0, 0.5], depth }));
        self.context.draw(0, 6, 1);
        let mut pixel = [0u8; 4];
        unsafe {
            glReadPixels(32, 32, 1, 1, GL_RGBA, GL_UNSIGNED_BYTE, pixel.as_mut_ptr().cast());
            let error = glGetError();
            assert_eq!(error, GL_NO_ERROR, "GL error in {label}: {error}");
            glDepthMask(GL_TRUE as u8);
        }
        self.context.end_render_pass();
        println!("{label}: depth_test={enabled}, depth_mask={mask}, rgba={pixel:?}");
        pixel
    }
}

impl EventHandler for Stage {
    fn update(&mut self) {}

    fn draw(&mut self) {
        let control = self.sample("behind_write_true", true, false, 0.5);
        let actual = self.sample("behind_write_false", false, false, 0.5);
        let workaround = self.sample("behind_manual_mask_false", true, true, 0.5);
        let front = self.sample("front_write_true", true, false, -0.75);
        let green = |pixel: [u8; 4]| pixel[0] < 3 && pixel[1] > 252 && pixel[2] < 3;
        let blended = |pixel: [u8; 4]| (125..=130).contains(&pixel[0]) && (125..=130).contains(&pixel[1]) && pixel[2] < 3;
        assert!(green(control), "depth-buffer control failed");
        assert!(green(workaround), "reported workaround failed");
        assert!(blended(front), "visible transparent geometry control failed");
        if blended(actual) {
            println!("RESULT: issue 670 reproduced; hidden quad contributes color only when depth_write=false; controls pass");
            std::process::exit(1);
        }
        assert!(green(actual), "unexpected pixel result");
        println!("RESULT: issue 670 not reproduced; hidden quad correctly occluded");
        std::process::exit(0);
    }
}

fn main() {
    let mut configuration = conf::Conf { window_title: "Miniquad issue 670 verification".to_owned(), window_width: 160, window_height: 160, ..Default::default() };
    configuration.platform.apple_gfx_api = conf::AppleGfxApi::OpenGl;
    miniquad::start(configuration, || Box::new(Stage::new()));
}
