use miniquad::{native::gl::*, *};

macro_rules! report {
    ($($argument:tt)*) => {{
        let text = format!($($argument)*);
        #[cfg(target_arch = "wasm32")]
        {
            let message = std::ffi::CString::new(text).unwrap();
            unsafe { miniquad::native::wasm::console_log(message.as_ptr()); }
        }
        #[cfg(not(target_arch = "wasm32"))]
        println!("{}", text);
    }};
}

const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

struct Stage {
    context: Box<dyn RenderingBackend>,
    shader: ShaderId,
    bindings: Bindings,
    pass: RenderPass,
    finished: bool,
    failures: usize,
    checks: usize,
}

impl Stage {
    fn new() -> Self {
        let mut context = window::new_rendering_backend();
        unsafe {
            report!("GL renderer: {}", std::ffi::CStr::from_ptr(glGetString(0x1F01).cast()).to_string_lossy());
            report!("GL version: {}", std::ffi::CStr::from_ptr(glGetString(GL_VERSION).cast()).to_string_lossy());
        }
        let shader = context.new_shader(
            ShaderSource::Glsl {
                vertex: "#version 100\nattribute vec2 position; uniform vec4 depth; void main() { gl_Position = vec4(position, depth.x + position.x * depth.y, 1.0); }",
                fragment: "#version 100\nprecision mediump float; uniform vec4 tint; void main() { gl_FragColor = tint; }",
            },
            ShaderMeta {
                images: vec![],
                uniforms: UniformBlockLayout { uniforms: vec![
                    UniformDesc::new("tint", UniformType::Float4),
                    UniformDesc::new("depth", UniformType::Float4),
                ] },
            },
        ).unwrap();
        let vertices: [[f32; 2]; 4] = [[-0.8, -0.8], [0.8, -0.8], [0.8, 0.8], [-0.8, 0.8]];
        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
        let vertex = context.new_buffer(BufferType::VertexBuffer, BufferUsage::Immutable, BufferSource::slice(&vertices));
        let index = context.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(&indices));
        let color = context.new_render_texture(TextureParams { width: 64, height: 64, format: TextureFormat::RGBA8, ..Default::default() });
        let depth = if cfg!(feature = "webgl2") {
            None
        } else {
            Some(context.new_render_texture(TextureParams { width: 64, height: 64, format: TextureFormat::Depth, ..Default::default() }))
        };
        let pass = context.new_render_pass(color, depth);
        context.begin_pass(Some(pass), PassAction::Nothing);
        unsafe {
            if cfg!(feature = "webgl2") {
                let mut renderbuffer = 0;
                glGenRenderbuffers(1, &mut renderbuffer);
                glBindRenderbuffer(GL_RENDERBUFFER, renderbuffer);
                glRenderbufferStorageMultisample(GL_RENDERBUFFER, 0, GL_DEPTH_COMPONENT16, 64, 64);
                glFramebufferRenderbuffer(GL_FRAMEBUFFER, GL_DEPTH_ATTACHMENT, GL_RENDERBUFFER, renderbuffer);
                glBindRenderbuffer(GL_RENDERBUFFER, 0);
            }
            assert_eq!(glCheckFramebufferStatus(GL_FRAMEBUFFER), GL_FRAMEBUFFER_COMPLETE);
            assert_eq!(glGetError(), GL_NO_ERROR, "framebuffer setup error");
        }
        context.end_render_pass();
        Self {
            context, shader, pass,
            bindings: Bindings { vertex_buffers: vec![vertex], index_buffer: index, images: vec![] },
            finished: false, failures: 0, checks: 0,
        }
    }

    fn pipeline(&mut self, comparison: Comparison, writes: bool, offset: Option<(f32, f32)>) -> Pipeline {
        self.context.new_pipeline(
            &[BufferLayout::default()],
            &[VertexAttribute::new("position", VertexFormat::Float2)],
            self.shader,
            PipelineParams { depth_test: comparison, depth_write: writes, depth_write_offset: offset, ..Default::default() },
        )
    }

    fn begin(&mut self) {
        unsafe {
            glDepthMask(GL_TRUE as u8);
            glDisable(GL_POLYGON_OFFSET_FILL);
            glPolygonOffset(0.0, 0.0);
        }
        self.context.begin_pass(Some(self.pass), PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
    }

    fn paint(&mut self, pipeline: Pipeline, tint: [f32; 4], depth: f32, slope: f32) {
        self.context.apply_pipeline(&pipeline);
        self.context.apply_bindings(&self.bindings);
        self.context.apply_uniforms(UniformsSource::table(&[tint, [depth, slope, 0.0, 0.0]]));
        self.context.draw(0, 6, 1);
    }

    fn check(&mut self, label: &str, passed: bool) {
        self.checks += 1;
        if !passed { self.failures += 1; }
        report!("{} {label}", if passed { "PASS" } else { "FAIL" });
    }

    fn state(&mut self, label: &str, parameter: u32, expected: i32) {
        let mut actual = 0;
        unsafe { glGetIntegerv(parameter, &mut actual); }
        self.check(&format!("{label}: actual={actual}, expected={expected}"), actual == expected);
    }

    fn pixel(&mut self, label: &str, expected: [f32; 4]) {
        let mut actual = [0u8; 4];
        unsafe { glReadPixels(31, 32, 1, 1, GL_RGBA, GL_UNSIGNED_BYTE, actual.as_mut_ptr().cast()); }
        let expected = expected.map(|channel| (channel * 255.0) as u8);
        self.check(&format!("{label}: actual={actual:?}, expected={expected:?}"), actual == expected);
    }

    fn end(&mut self) {
        let error = unsafe { glGetError() };
        self.check(&format!("GL error={error}"), error == GL_NO_ERROR);
        self.context.end_render_pass();
    }

    fn run(&mut self) {
        let write = self.pipeline(Comparison::Less, true, None);
        let readonly = self.pipeline(Comparison::Less, false, None);
        self.begin();
        self.paint(write, GREEN, -0.5, 0.0);
        self.paint(readonly, RED, 0.5, 0.0);
        self.pixel("issue 670: read-only rear quad remains occluded", GREEN);
        self.end();
        for comparison in [Comparison::Never, Comparison::Less, Comparison::LessOrEqual,
            Comparison::Greater, Comparison::GreaterOrEqual, Comparison::Equal,
            Comparison::NotEqual, Comparison::Always] {
            for writes in [false, true] {
                self.begin();
                self.paint(write, GREEN, 0.0, 0.0);
                let candidate = self.pipeline(comparison, writes, None);
                let depth = match comparison {
                    Comparison::Less => -0.5,
                    Comparison::Greater | Comparison::NotEqual | Comparison::Always => 0.5,
                    _ => 0.0,
                };
                self.paint(candidate, RED, depth, 0.0);
                let label = format!("{comparison:?}, writes={writes}");
                self.pixel(&label, if comparison == Comparison::Never { GREEN } else { RED });
                self.state(&format!("{label} mask"), 0x0B72, writes as i32);
                self.state(&format!("{label} test"), GL_DEPTH_TEST, (writes || comparison != Comparison::Always) as i32);
                self.end();
            }
        }

        self.begin();
        self.paint(readonly, GREEN, -0.5, 0.0);
        self.paint(write, RED, 0.0, 0.0);
        self.pixel("read-only geometry did not write depth; writing re-enabled", RED);
        self.paint(write, BLUE, 0.5, 0.0);
        self.pixel("re-enabled writing stored depth", RED);
        self.end();

        for writes in [false, true] {
            self.begin();
            self.paint(write, GREEN, -0.5, 0.0);
            let always = self.pipeline(Comparison::Always, writes, None);
            self.paint(always, RED, 0.5, 0.0);
            self.paint(write, BLUE, 0.0, 0.0);
            self.pixel(&format!("Always writes={writes} depth storage"), if writes { BLUE } else { RED });
            self.end();
        }

        self.begin();
        self.paint(write, GREEN, -0.5, 0.0);
        self.context.apply_pipeline(&readonly);
        self.context.clear(None, Some(1.0), None);
        self.state("depth clear restores read-only mask", 0x0B72, 0);
        self.context.apply_bindings(&self.bindings);
        self.context.apply_uniforms(UniformsSource::table(&[RED, [0.0f32; 4]]));
        self.context.draw(0, 6, 1);
        self.pixel("depth clear allows drawing without reapplying pipeline", RED);
        self.paint(write, BLUE, 0.5, 0.0);
        self.pixel("draw after clear did not write depth", BLUE);
        self.end();

        self.begin();
        self.paint(write, GREEN, -0.5, 0.0);
        unsafe { glDepthMask(GL_FALSE as u8); }
        self.context.clear(None, Some(1.0), None);
        self.state("depth clear preserves directly disabled mask", 0x0B72, 0);
        self.paint(write, RED, 0.0, 0.0);
        self.pixel("depth clear works with directly disabled mask", RED);
        self.end();

        self.begin();
        self.paint(write, GREEN, -0.5, 0.0);
        self.context.apply_pipeline(&readonly);
        self.context.clear(Some((0.0, 0.0, 1.0, 1.0)), None, None);
        self.state("color-only clear preserves mask", 0x0B72, 0);
        self.paint(write, RED, 0.0, 0.0);
        self.pixel("color-only clear leaves depth intact", BLUE);
        self.end();

        self.begin();
        self.paint(write, GREEN, -0.5, 0.0);
        self.context.apply_pipeline(&readonly);
        self.context.end_render_pass();
        self.context.begin_pass(Some(self.pass), PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        self.state("begin_pass clear preserves mask", 0x0B72, 0);
        self.paint(write, RED, 0.0, 0.0);
        self.pixel("begin_pass clears previous depth while read-only", RED);
        self.end();

        let negative = self.pipeline(Comparison::Less, true, Some((-4.0, -4.0)));
        let positive = self.pipeline(Comparison::LessOrEqual, true, Some((4.0, 4.0)));
        let no_offset = self.pipeline(Comparison::LessOrEqual, true, None);
        self.begin();
        self.paint(write, GREEN, 0.0, 0.0);
        self.paint(negative, RED, 0.0, 0.0);
        self.pixel("negative offset", RED);
        self.context.clear(Some((0.0, 0.0, 0.0, 1.0)), Some(1.0), None);
        self.paint(write, GREEN, 0.0, 0.0);
        self.state("Some to None disables offset", GL_POLYGON_OFFSET_FILL, 0);
        self.paint(positive, RED, 0.0, 0.0);
        self.pixel("positive offset after negative", GREEN);
        self.state("positive factor replaces negative", 0x8038, 4);
        self.state("positive units replace negative", 0x2A00, 4);
        self.paint(no_offset, RED, 0.0, 0.0);
        self.state("positive to None disables offset", GL_POLYGON_OFFSET_FILL, 0);
        self.pixel("None after positive permits equal depth", RED);
        self.end();

        self.begin();
        self.paint(write, GREEN, 0.0, 0.0);
        let negative_readonly = self.pipeline(Comparison::Less, false, Some((-4.0, -4.0)));
        let positive_readonly = self.pipeline(Comparison::LessOrEqual, false, Some((4.0, 4.0)));
        let equal_readonly = self.pipeline(Comparison::LessOrEqual, false, None);
        self.paint(negative_readonly, RED, 0.0, 0.0);
        self.pixel("negative offset with depth writing disabled", RED);
        self.state("combined negative offset write mask", 0x0B72, 0);
        self.state("combined negative offset enabled", GL_POLYGON_OFFSET_FILL, 1);
        self.paint(positive_readonly, BLUE, 0.0, 0.0);
        self.pixel("direct negative to positive offset with writing disabled", RED);
        self.state("combined positive factor", 0x8038, 4);
        self.state("combined positive units", 0x2A00, 4);
        self.paint(equal_readonly, BLUE, 0.0, 0.0);
        self.pixel("read-only offset did not change stored depth", BLUE);
        self.state("combined None disables offset", GL_POLYGON_OFFSET_FILL, 0);
        self.end();

        for factor in [-4.0, 4.0] {
            self.begin();
            self.paint(write, GREEN, 0.0, 0.25);
            let comparison = if factor < 0.0 { Comparison::Less } else { Comparison::LessOrEqual };
            let slope_pipeline = self.pipeline(comparison, true, Some((factor, 0.0)));
            self.paint(slope_pipeline, RED, 0.0, 0.25);
            self.pixel(&format!("slope factor {factor}, units zero"), if factor < 0.0 { RED } else { GREEN });
            self.end();
        }
        report!("RESULT: {} checks, {} failures", self.checks, self.failures);
        self.finished = true;
    }
}

impl EventHandler for Stage {
    fn update(&mut self) {}
    fn draw(&mut self) { if !self.finished { self.run(); } }
}

fn main() {
    let mut configuration = conf::Conf {
        window_title: "Combined depth regression tests".to_owned(),
        window_width: 160, window_height: 160, ..Default::default()
    };
    configuration.platform.apple_gfx_api = conf::AppleGfxApi::OpenGl;
    configuration.platform.webgl_version = if cfg!(feature = "webgl2") { conf::WebGLVersion::WebGL2 } else { conf::WebGLVersion::WebGL1 };
    miniquad::start(configuration, || Box::new(Stage::new()));
}
