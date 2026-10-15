use miniquad::{native::gl::*, *};

fn report(text: &str) {
    let message = std::ffi::CString::new(text).unwrap();
    unsafe { miniquad::native::wasm::console_log(message.as_ptr()); }
}

struct Stage {
    context: Box<dyn RenderingBackend>,
    pipeline: Pipeline,
    bindings: Bindings,
    first: TextureId,
    second: TextureId,
    finished: bool,
}

impl Stage {
    fn new() -> Self {
        let mut context = window::new_rendering_backend();
        let shader = context.new_shader(
            ShaderSource::Glsl {
                vertex: "#version 100\nattribute vec2 position; varying vec2 uv; void main(){gl_Position=vec4(position,0.0,1.0);uv=position*0.5+0.5;}",
                fragment: "#version 100\nprecision mediump float; varying vec2 uv; uniform sampler2D image; void main(){gl_FragColor=texture2D(image,uv);}",
            },
            ShaderMeta { images: vec!["image".to_owned()], uniforms: UniformBlockLayout { uniforms: vec![] } },
        ).unwrap();
        let pipeline = context.new_pipeline(&[BufferLayout::default()], &[VertexAttribute::new("position", VertexFormat::Float2)], shader, PipelineParams::default());
        let vertices = [[-1.0f32,-1.0], [1.0,-1.0], [1.0,1.0], [-1.0,1.0]];
        let indices = [0u16,1,2,0,2,3];
        let vertex = context.new_buffer(BufferType::VertexBuffer, BufferUsage::Immutable, BufferSource::slice(&vertices));
        let index = context.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(&indices));
        let first = context.new_texture_from_rgba8(1, 1, &[255,0,0,255]);
        let second = context.new_texture_from_rgba8(1, 1, &[0,0,255,255]);
        Self { context, pipeline, first, second, finished: false, bindings: Bindings { vertex_buffers: vec![vertex], index_buffer: index, images: vec![first] } }
    }
}

impl EventHandler for Stage {
    fn update(&mut self) {}

    fn draw(&mut self) {
        if self.finished { return; }
        self.finished = true;
        self.context.begin_default_pass(PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        self.context.apply_pipeline(&self.pipeline);
        self.context.apply_bindings(&self.bindings);
        self.context.draw(0, 6, 1);
        let mut before = [0u8;4];
        unsafe { glReadPixels(16,16,1,1,GL_RGBA,GL_UNSIGNED_BYTE,before.as_mut_ptr().cast()); }
        assert_eq!(before, [255,0,0,255]);
        assert_eq!(unsafe { glGetError() }, GL_NO_ERROR);
        report("CONTROL: live first texture draws red without GL error");
        if !cfg!(feature="live-control") {
            report("PHASE: delete first texture using Miniquad API");
            self.context.delete_texture(self.first);
        }
        if cfg!(feature="invalid-handle") {
            report("PHASE: deliberately rebind a deleted application handle after binding a live one");
            self.bindings.images[0] = self.second;
            self.context.apply_bindings(&self.bindings);
            self.bindings.images[0] = self.first;
            self.context.apply_bindings(&self.bindings);
        } else {
            report("PHASE: update only the other live texture; application never reuses deleted handle");
            self.context.texture_update(self.second, &[0,255,0,255]);
        }
        report("PHASE: verify that live-texture rendering still works afterwards");
        self.context.texture_update(self.second, &[0,255,0,255]);
        self.bindings.images[0] = self.second;
        self.context.apply_bindings(&self.bindings);
        self.context.draw(0,6,1);
        let mut pixel = [0u8;4];
        unsafe { glReadPixels(16,16,1,1,GL_RGBA,GL_UNSIGNED_BYTE,pixel.as_mut_ptr().cast()); }
        let error = unsafe { glGetError() };
        report(&format!("RESULT: rgba={pixel:?}; error={error}"));
        self.context.end_render_pass();
    }
}

fn main() {
    let mut configuration = conf::Conf { window_title: "Texture lifetime verification".to_owned(), window_width: 64, window_height: 64, ..Default::default() };
    configuration.platform.webgl_version = if cfg!(feature="webgl2") { conf::WebGLVersion::WebGL2 } else { conf::WebGLVersion::WebGL1 };
    miniquad::start(configuration, || Box::new(Stage::new()));
}
