use super::debug;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;
use web_sys::WebGlProgram;
use web_sys::WebGlRenderingContext as GL;
use web_sys::WebGlShader;
use yew::services::{RenderService, Task};
use yew::{html, Component, ComponentLink, Html, NodeRef, Properties, ShouldRender};

pub struct Model {
    canvas: Option<HtmlCanvasElement>,
    gl: Option<GL>,
    link: ComponentLink<Self>,
    props: Props,
    canvas_ref: NodeRef,
    render_loop: Option<Box<dyn Task>>,
    shader_program: Option<WebGlProgram>,
}

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub cursor: (i32, i32),
}

pub enum Msg {
    Render(f64),
}

impl Component for Model {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Model {
            canvas: None,
            gl: None,
            link,
            props,
            canvas_ref: NodeRef::default(),
            render_loop: None,
            shader_program: None,
        }
    }

    // TODO: Fix WebGL warnings
    fn rendered(&mut self, first_render: bool) {
        let canvas = self
            .canvas_ref
            .cast::<HtmlCanvasElement>()
            .expect("Failed to get canvas element");

        let gl: GL = canvas
            .get_context("webgl")
            .expect("Failed to get WebGL context")
            .expect("Failed to get js_sys Object")
            .dyn_into()
            .expect("Failed to cast to WebGL render context");

        self.canvas = Some(canvas);
        self.gl = Some(gl);

        if first_render {
            let shader_program =
                self.compile_program(include_str!("./basic.vert"), include_str!("./basic.frag"));
            self.shader_program = Some(shader_program);

            let render_frame = self.link.callback(Msg::Render);
            let handle = RenderService::request_animation_frame(render_frame);

            self.render_loop = Some(Box::new(handle));
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Render(timestamp) => {
                self.render_gl(timestamp);
                true
            }
        }
    }

    fn view(&self) -> Html {
        html! {
            <canvas ref=self.canvas_ref.clone()/>
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if self.props != props {
            self.props = props;
            true
        } else {
            false
        }
    }
}

impl Model {
    fn render_gl(&mut self, timestamp: f64) {
        let gl = self.gl.as_ref().expect("GL Context not initialized!");

        let vertices: Vec<f32> = vec![
            -1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, 1.0,
        ];

        let vertex_buffer = gl.create_buffer().expect("Failed to create GL buffer");
        let verts = js_sys::Float32Array::from(vertices.as_slice());

        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&vertex_buffer));
        gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &verts, GL::STATIC_DRAW);

        let shader_program = self
            .shader_program
            .as_ref()
            .expect("Shader program uninitialised!");

        gl.use_program(Some(shader_program));

        let position = gl.get_attrib_location(shader_program, "a_position") as u32;
        gl.vertex_attrib_pointer_with_i32(position, 2, GL::FLOAT, false, 0, 0);
        gl.enable_vertex_attrib_array(position);

        let time_uni = gl.get_uniform_location(shader_program, "time");
        gl.uniform1f(time_uni.as_ref(), ((timestamp as f64) / 1000.0f64) as f32);

        let range = self
            .canvas
            .as_ref()
            .expect("Canvas not initialised")
            .get_bounding_client_rect();

        let relative_x = ((self.props.cursor.0 as f64) - range.x()) as f32;
        let relative_y = ((self.props.cursor.1 as f64) - range.y()) as f32;

        let cursor_uni = gl.get_uniform_location(shader_program, "cursor");
        gl.uniform2f(cursor_uni.as_ref(), relative_x, relative_y);

        let resolution_uni = gl.get_uniform_location(shader_program, "resolution");
        // I think this might be the default for canvas? It's doesn't match the html element size,
        // but it's the range of gl_FragCoord inside the shader.
        gl.uniform2f(resolution_uni.as_ref(), 300.0, 150.0);

        let element_size_uni = gl.get_uniform_location(shader_program, "element_size");
        gl.uniform2f(
            element_size_uni.as_ref(),
            range.width() as f32,
            range.height() as f32,
        );

        gl.draw_arrays(GL::TRIANGLES, 0, 6);

        let render_frame = self.link.callback(Msg::Render);
        let handle = RenderService::request_animation_frame(render_frame);

        // A reference to the new handle must be retained for the next render to run.
        self.render_loop = Some(Box::new(handle));
    }

    fn compile_program(&mut self, vert_code: &str, frag_code: &str) -> WebGlProgram {
        let gl = self.gl.as_ref().expect("GL Context not initialized!");
        let shader_program = gl
            .create_program()
            .expect("Failed to create shader program");

        let vert_shader = compile_shader(gl, vert_code, GL::VERTEX_SHADER);

        let frag_shader = compile_shader(gl, frag_code, GL::FRAGMENT_SHADER);

        gl.attach_shader(&shader_program, &vert_shader);
        gl.attach_shader(&shader_program, &frag_shader);
        gl.link_program(&shader_program);

        shader_program
    }
}

fn compile_shader(gl: &GL, code: &str, shader_type: u32) -> WebGlShader {
    let shader = gl
        .create_shader(shader_type)
        .expect("Failed to create shader");
    gl.shader_source(&shader, code);
    gl.compile_shader(&shader);

    let error_log = gl.get_shader_info_log(&shader);
    if let Some(msg) = error_log {
        debug::log(&msg);
    }

    shader
}
