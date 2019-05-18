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

use rand::Rng;

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

// Fill the buffer with the values that define a rectangle.
fn set_rectangle(context: &gl, x: u32, y: u32, width: u32, height: u32) {
    let x1 = x as f32;
    let x2 = x1 + width as f32;
    let y1 = y as f32;
    let y2 = y1 + height as f32;

    let positions = TypedArray::<f32>::from(
        &[
            // 1st triangle
            x1, y1, // 1st point
            x2, y1, // 2nd point
            x1, y2, // 3rd point
            // 2nd triangle
            x1, y2, // 1st point
            x2, y1, // 2nd point
            x2, y2, // 3rd point
        ][..],
    )
    .buffer();

    context.buffer_data_1(gl::ARRAY_BUFFER, Some(&positions), gl::STATIC_DRAW);
}

fn main() {
    stdweb::initialize();

    let vert_code = r#"
        attribute vec2 a_position;
        uniform vec2 u_resolution;

        void main() {
            // convert the position from pixels to 0.0 to 1.0
            vec2 zeroToOne = a_position / u_resolution;

            // convert from 0->1 to 0->2
            vec2 zeroToTwo = zeroToOne * 2.0;

            // convert from 0->2 to -1->+1 (clipspace)
            vec2 clipSpace = zeroToTwo - 1.0;

            gl_Position = vec4(clipSpace * vec2(1, -1), 0, 1);
        }"#;

    let frag_code = r#"
        precision mediump float;
        uniform vec4 u_color;

        void main() {
            gl_FragColor = u_color;
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
    // look up uniform locations
    let resolution_uniform_location = context
        .get_uniform_location(&program, "u_resolution")
        .unwrap();
    let color_uniform_location = context.get_uniform_location(&program, "u_color").unwrap();

    // Create a buffer and put three 2d clip space points in it
    let position_buffer = context.create_buffer().unwrap();

    // Bind it to ARRAY_BUFFER (think of it as ARRAY_BUFFER = positionBuffer)
    context.bind_buffer(gl::ARRAY_BUFFER, Some(&position_buffer));

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

    // set the resolution
    context.uniform2f(
        Some(&resolution_uniform_location),
        canvas.width() as f32,
        canvas.height() as f32,
    );

    // random generator
    let mut rng = rand::thread_rng();

    // draw 50 random rectangles in random colors
    for _ in 0..50 {
        // Setup a random rectangle
        // This will write to positionBuffer because
        // its the last thing we bound on the ARRAY_BUFFER
        // bind point
        set_rectangle(
            &context,
            rng.gen_range(0, 300),
            rng.gen_range(0, 300),
            rng.gen_range(0, 300),
            rng.gen_range(0, 300),
        );

        // Set a random color.
        context.uniform4f(
            Some(&color_uniform_location),
            rng.gen::<f32>(),
            rng.gen::<f32>(),
            rng.gen::<f32>(),
            1.0,
        );

        let primitive_type = gl::TRIANGLES;
        let offset = 0;
        let count = 6;
        context.draw_arrays(primitive_type, offset, count);
    }

    stdweb::event_loop();
}
