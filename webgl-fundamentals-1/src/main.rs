#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate stdweb_derive;

mod webgl_rendering_context;

use stdweb::unstable::TryInto;
use stdweb::web::{document, IParentNode, TypedArray};

use stdweb::web::html_element::CanvasElement;
use webgl_rendering_context::{GLenum, WebGLProgram, WebGLRenderingContext as gl, WebGLShader};

fn create_shader(context: &gl, type_: GLenum, source: &'static str) -> Option<WebGLShader> {
    let shader = context.create_shader(type_).unwrap();
    context.shader_source(&shader, source);
    context.compile_shader(&shader);

    let success: bool = context
        .get_shader_parameter(&shader, gl::COMPILE_STATUS)
        .try_into()
        .unwrap();
    if success {
        return Some(shader);
    }

    console!(error, context.get_shader_info_log(&shader));
    context.delete_shader(Some(&shader));
    None
}

fn create_program(
    context: &gl,
    vertex_shader: &WebGLShader,
    fragment_shader: &WebGLShader,
) -> Option<WebGLProgram> {
    let program = context.create_program().unwrap();
    context.attach_shader(&program, vertex_shader);
    context.attach_shader(&program, fragment_shader);
    context.link_program(&program);

    let success: bool = context
        .get_program_parameter(&program, gl::LINK_STATUS)
        .try_into()
        .unwrap();
    if success {
        return Some(program);
    }

    console!(error, context.get_program_info_log(&program));
    context.delete_program(Some(&program));
    None
}

fn canvas_client_width(canvas: &CanvasElement) -> u32 {
    js! (
        return @{canvas}.clientWidth;
    )
    .try_into()
    .unwrap()
}

fn canvas_client_height(canvas: &CanvasElement) -> u32 {
    js! (
        return @{canvas}.clientHeight;
    )
    .try_into()
    .unwrap()
}

fn resize_canvas_to_display_size(canvas: &mut CanvasElement) {
    let width = canvas_client_width(canvas);
    let height = canvas_client_height(canvas);
    if canvas.width() != width || canvas.height() != height {
        canvas.set_width(width);
        canvas.set_height(height);
    }
}

fn main() {
    stdweb::initialize();

    let vert_code = r#"
        // an attribute will receive data from a buffer
        attribute vec4 a_position;

        // all shaders have a main function
        void main() {
            // gl_Position is a special variable a vertex shader
            // is responsible for setting
            gl_Position = a_position;
        }"#;

    let frag_code = r#"
        // fragment shaders don't have a default precision so we need
        // to pick one. mediump is a good default
        precision mediump float;

        void main() {
            // gl_FragColor is a special variable a fragment shader
            // is responsible for setting
            gl_FragColor = vec4(1, 0, 0.5, 1); // return redish-purple
        }"#;

    // Get A WebGL context
    let mut canvas: CanvasElement = document()
        .query_selector("#canvas")
        .unwrap()
        .unwrap()
        .try_into()
        .unwrap();
    let context: gl = canvas.get_context().unwrap();

    // create GLSL shaders, upload the GLSL source, compile the shaders
    let vertex_shader = create_shader(&context, gl::VERTEX_SHADER, vert_code).unwrap();
    let fragment_shader = create_shader(&context, gl::FRAGMENT_SHADER, frag_code).unwrap();

    // Link the two shaders into a program
    let program = create_program(&context, &vertex_shader, &fragment_shader).unwrap();

    // look up where the vertex data needs to go.
    let position_attribute_location = context.get_attrib_location(&program, "a_position") as u32;

    // Create a buffer and put three 2d clip space points in it
    let position_buffer = context.create_buffer().unwrap();

    // Bind it to ARRAY_BUFFER (think of it as ARRAY_BUFFER = positionBuffer)
    context.bind_buffer(gl::ARRAY_BUFFER, Some(&position_buffer));

    let positions = TypedArray::<f32>::from(
        &[
            0.0, 0.0, // 1st point
            0.0, 0.5, // 2nd point
            0.7, 0.0, // 3rd point
        ][..],
    )
    .buffer();

    context.buffer_data_1(gl::ARRAY_BUFFER, Some(&positions), gl::STATIC_DRAW);

    // code above this line is initialization code.
    // code below this line is rendering code.

    resize_canvas_to_display_size(&mut canvas);

    // Tell WebGL how to convert from clip space to pixels
    let (w, h) = (canvas.width(), canvas.height());
    context.viewport(0, 0, w as i32, h as i32);

    // Clear the canvas
    context.clear_color(0.0, 0.0, 0.0, 0.0);
    context.clear(gl::COLOR_BUFFER_BIT);

    // Tell it to use our program (pair of shaders)
    context.use_program(Some(&program));

    // Turn on the attribute
    context.enable_vertex_attrib_array(position_attribute_location);

    // Bind the position buffer.
    context.bind_buffer(gl::ARRAY_BUFFER, Some(&position_buffer));

    // Tell the attribute how to get data out of positionBuffer (ARRAY_BUFFER)
    let size = 2; // 2 components per iteration
    let type_ = gl::FLOAT; // the data is 32bit floats
    let normalize = false; // don't normalize the data
    let stride = 0; // 0 = move forward size * sizeof(type) each iteration to get the next position
    let offset = 0; // start at the beginning of the buffer
    context.vertex_attrib_pointer(
        position_attribute_location,
        size,
        type_,
        normalize,
        stride,
        offset,
    );

    // draw
    let primitive_type = gl::TRIANGLES;
    let offset = 0;
    let count = 3;
    context.draw_arrays(primitive_type, offset, count);

    stdweb::event_loop();
}
