use miniquad::{native::gl::*, *};

struct Stage {
    context: Box<dyn RenderingBackend>,
    bindings: Bindings,
    shader: ShaderId,
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
                vertex: "#version 100\nattribute vec2 position; void main() { gl_Position = vec4(position, 0.0, 1.0); }",
                fragment: "#version 100\nprecision mediump float; uniform vec4 tint; void main() { gl_FragColor = tint; }",
            },
            ShaderMeta {
                images: vec![],
                uniforms: UniformBlockLayout {
                    uniforms: vec![UniformDesc::new("tint", UniformType::Float4)],
                },
            },
        ).unwrap();
        let vertices: [[f32; 2]; 4] = [
            [-0.8, -0.8], [0.8, -0.8], [0.8, 0.8], [-0.8, 0.8],
        ];
        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
        let vertex_buffer = context.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&vertices),
        );
        let index_buffer = context.new_buffer(
            BufferType::IndexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&indices),
        );
        let bindings = Bindings {
            vertex_buffers: vec![vertex_buffer],
            index_buffer,
            images: vec![],
        };
        let color = context.new_render_texture(TextureParams {
            width: 64,
            height: 64,
            format: TextureFormat::RGBA8,
            ..Default::default()
        });
        let depth = context.new_render_texture(TextureParams {
            width: 64,
            height: 64,
            format: TextureFormat::Depth,
            ..Default::default()
        });
        let pass = context.new_render_pass(color, Some(depth));
        Self { context, bindings, shader, pass }
    }

    fn pipeline(&mut self, comparison: Comparison, offset: Option<(f32, f32)>) -> Pipeline {
        self.context.new_pipeline(
            &[BufferLayout::default()],
            &[VertexAttribute::new("position", VertexFormat::Float2)],
            self.shader,
            PipelineParams {
                depth_test: comparison,
                depth_write: true,
                depth_write_offset: offset,
                ..Default::default()
            },
        )
    }

    fn sample(
        &mut self,
        label: &str,
        comparison: Comparison,
        api_offset: Option<(f32, f32)>,
        manual_offset: Option<(f32, f32)>,
    ) -> [u8; 4] {
        let base = self.pipeline(Comparison::Less, None);
        let overlay = self.pipeline(comparison, api_offset);
        unsafe {
            glDisable(GL_POLYGON_OFFSET_FILL);
            glPolygonOffset(0.0, 0.0);
            glDepthMask(GL_TRUE as u8);
        }
        self.context.begin_pass(Some(self.pass), PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        self.context.apply_pipeline(&base);
        self.context.apply_bindings(&self.bindings);
        self.context.apply_uniforms(UniformsSource::table(&[0.0f32, 1.0, 0.0, 1.0]));
        self.context.draw(0, 6, 1);
        self.context.apply_pipeline(&overlay);
        if let Some((factor, units)) = manual_offset {
            unsafe {
                glEnable(GL_POLYGON_OFFSET_FILL);
                glPolygonOffset(factor, units);
            }
        }
        let mut enabled = 0;
        let mut factor = 0;
        let mut units = 0;
        let mut depth_test = 0;
        unsafe {
            glGetIntegerv(GL_POLYGON_OFFSET_FILL, &mut enabled);
            glGetIntegerv(0x8038, &mut factor);
            glGetIntegerv(0x2A00, &mut units);
            glGetIntegerv(GL_DEPTH_TEST, &mut depth_test);
        }
        assert_eq!(depth_test, 1, "depth test must remain enabled in {label}");
        self.context.apply_uniforms(UniformsSource::table(&[1.0f32, 0.0, 0.0, 1.0]));
        self.context.draw(0, 6, 1);
        let mut pixel = [0u8; 4];
        unsafe {
            glReadPixels(31, 32, 1, 1, GL_RGBA, GL_UNSIGNED_BYTE, pixel.as_mut_ptr().cast());
            assert_eq!(glGetError(), GL_NO_ERROR, "GL error in {label}");
            glDisable(GL_POLYGON_OFFSET_FILL);
            glPolygonOffset(0.0, 0.0);
        }
        self.context.end_render_pass();
        self.context.delete_pipeline(base);
        self.context.delete_pipeline(overlay);
        println!("{label}: offset_enabled={enabled}, factor={factor}, units={units}, depth_test={depth_test}, rgba={pixel:?}");
        pixel
    }
}

impl EventHandler for Stage {
    fn update(&mut self) {}

    fn draw(&mut self) {
        let less_none = self.sample("less_no_offset", Comparison::Less, None, None);
        let less_api = self.sample("less_api_negative", Comparison::Less, Some((-4.0, -4.0)), None);
        let less_manual = self.sample("less_manual_negative", Comparison::Less, None, Some((-4.0, -4.0)));
        let equal_none = self.sample("lequal_no_offset", Comparison::LessOrEqual, None, None);
        let equal_api = self.sample("lequal_api_positive", Comparison::LessOrEqual, Some((4.0, 4.0)), None);
        let equal_manual = self.sample("lequal_manual_positive", Comparison::LessOrEqual, None, Some((4.0, 4.0)));
        let green = [0, 255, 0, 255];
        let red = [255, 0, 0, 255];
        assert_eq!(less_none, green, "equal depth must fail Less");
        assert_eq!(equal_none, red, "equal depth must pass LessOrEqual");
        assert_eq!(less_manual, red, "manual negative offset must bring overlay forward");
        assert_eq!(equal_manual, green, "manual positive offset must push overlay back");
        if less_api == green && equal_api == red {
            println!("RESULT: issue 597 reproduced; both API offsets ignored; all four controls pass");
            std::process::exit(1);
        }
        assert_eq!(less_api, red, "unexpected negative API offset result");
        assert_eq!(equal_api, green, "unexpected positive API offset result");
        println!("RESULT: issue 597 not reproduced; both API offsets work");
        std::process::exit(0);
    }
}

fn main() {
    let mut configuration = conf::Conf {
        window_title: "Miniquad issue 597 verification".to_owned(),
        window_width: 160,
        window_height: 160,
        ..Default::default()
    };
    configuration.platform.apple_gfx_api = conf::AppleGfxApi::OpenGl;
    miniquad::start(configuration, || Box::new(Stage::new()));
}
