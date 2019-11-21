use basegl_backend_webgl::{Context, compile_shader, link_program, Program, Shader};

use web_sys::{WebGlRenderingContext, WebGlBuffer, WebGlTexture};
use crate::prelude::*;

pub mod font;

use font::FontRenderInfo;
use crate::text::font::MsdfTexture;
use js_sys::Float32Array;
use crate::display::workspace::WorkspaceData;

#[derive(Debug)]
pub struct Color {
    pub r : f32,
    pub g : f32,
    pub b : f32,
    pub a : f32
}

#[derive(Debug)]
struct TextShaders {
    vert_shader : Shader,
    frag_shader : Shader,
    program     : Program
}

#[derive(Debug)]
pub struct TextComponent {
    pub text             : String,
    pub x                : f32,
    pub y                : f32,
    pub size             : f32,
    pub color            : Color,
    pub background_color : Color,

    workspace        : Rc<WorkspaceData>,
    gl_shaders       : TextShaders,
    gl_vertex_buf    : WebGlBuffer,
    gl_tex_coord_buf : WebGlBuffer,
    gl_msdf_texture  : WebGlTexture
}

impl TextComponent {
    pub fn new(
        workspace        : Rc<WorkspaceData>,
        text             : String,
        x                : f32,
        y                : f32,
        size             : f32,
        font             : &mut FontRenderInfo,
        color            : Color,
        background_color : Color,
    ) -> TextComponent {
        let gl_shaders = Self::create_shaders(&workspace.context);
        let gl_vertex_buf = Self::create_vertex_buf(
            &workspace.context,
            text.as_str(),
            x,
            y,
            size,
        );
        let gl_tex_coord_buf = Self::create_tex_coord_buf(
            &workspace.context,
            text.as_str(),
            font,
        );
        let gl_msdf_texture = Self::create_msdf_texture(
            &workspace.context,
            &gl_shaders.program,
            font
        );

        let component = TextComponent {
            text, x, y, size, workspace, color, background_color,
            gl_shaders, gl_vertex_buf, gl_tex_coord_buf, gl_msdf_texture
        };
        component.setup_uniforms();
        component
    }

    fn create_shaders(gl_context : &Context) -> TextShaders {
        gl_context.get_extension("OES_standard_derivatives")
            .unwrap().unwrap();
        let vert_shader = compile_shader(
            &gl_context,
            WebGlRenderingContext::VERTEX_SHADER,
            include_str!("msdf_vert.glsl")
        ).unwrap();
        let frag_shader = compile_shader(
            &gl_context,
            WebGlRenderingContext::FRAGMENT_SHADER,
            include_str!("msdf_frag.glsl")
        ).unwrap();

        let program = link_program(&gl_context, &vert_shader, &frag_shader)
            .unwrap();

        TextShaders { vert_shader, frag_shader, program}
    }

    fn create_vertex_buf(
        gl_context : &Context,
        text : &str,
        x : f32,
        y : f32,
        size : f32
    ) -> WebGlBuffer {
        let y_max = y  + size;
        let x_step = size;
        let vertices = (0..text.len()).map(|i| {
            let ix      = x + (i as f32) * x_step;
            let ix_max  = ix + x_step;
            vec![ix, y, ix, y_max, ix_max, y, ix_max,
                y, ix, y_max, ix_max, y_max]
        }).flatten().collect::<Box<[f32]>>();

        let buffer = gl_context.create_buffer().unwrap();
        gl_context.bind_buffer(
            WebGlRenderingContext::ARRAY_BUFFER,
            Some(&buffer)
        );
        unsafe {
            let float_32_array = Float32Array::view(&vertices);
            gl_context.buffer_data_with_array_buffer_view(
                WebGlRenderingContext::ARRAY_BUFFER,
                &float_32_array,
                WebGlRenderingContext::STATIC_DRAW,
            );
        }
        buffer
    }

    fn create_tex_coord_buf(
        gl_context : &Context,
        text : &str,
        font : &mut FontRenderInfo,
    ) -> WebGlBuffer {
        for ch in text.chars() {
            font.get_char_info(ch);
        }
        let vertices = text.chars().map(|c| {
            let msdf_rows = font.msdf_texture.rows() as f32;
            let info = font.get_char_info(c);
            let y_min = info.msdf_texture_rows.start as f32 / msdf_rows;
            let y_max = info.msdf_texture_rows.end as f32 / msdf_rows;
            vec![0.0, y_min, 0.0, y_max, 1.0, y_min,
                1.0, y_min, 0.0, y_max, 1.0, y_max]
        }).flatten().collect::<Box<[f32]>>();

        let buffer = gl_context.create_buffer().unwrap();
        gl_context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&buffer));
        unsafe {
            let float_32_array = Float32Array::view(&vertices);
            gl_context.buffer_data_with_array_buffer_view(
                WebGlRenderingContext::ARRAY_BUFFER,
                &float_32_array,
                WebGlRenderingContext::STATIC_DRAW,
            );
        }
        buffer
    }

    fn create_msdf_texture(
        gl_context : &Context,
        gl_program : &Program,
        font : &FontRenderInfo
    ) -> WebGlTexture {
        let msdf_texture = gl_context.create_texture().unwrap();
        gl_context.bind_texture(Context::TEXTURE_2D, Some(&msdf_texture));

        gl_context.tex_parameteri(
            Context::TEXTURE_2D,
            Context::TEXTURE_WRAP_S,
            Context::CLAMP_TO_EDGE as i32
        );
        gl_context.tex_parameteri(
            Context::TEXTURE_2D,
            Context::TEXTURE_WRAP_T,
            Context::CLAMP_TO_EDGE as i32
        );
        gl_context.tex_parameteri(
            Context::TEXTURE_2D,
            Context::TEXTURE_MIN_FILTER,
            Context::LINEAR as i32
        );

        gl_context.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            Context::TEXTURE_2D,
            0,
            Context::RGB as i32,
            MsdfTexture::WIDTH as i32,
            font.msdf_texture.rows() as i32,
            0,
            Context::RGB,
            Context::UNSIGNED_BYTE,
            Some(font.msdf_texture.data.as_slice())
        ).unwrap();

        let msdf_loc = gl_context.get_uniform_location(gl_program, "msdf");
        let msdf_size_loc =
            gl_context.get_uniform_location(gl_program, "msdfSize");

        gl_context.use_program(Some(gl_program));
        gl_context.uniform1i(msdf_loc.as_ref(), 0);
        gl_context.uniform2f(
            msdf_size_loc.as_ref(),
            MsdfTexture::WIDTH as f32,
            font.msdf_texture.rows() as f32
        );

        msdf_texture
    }

    fn setup_uniforms(&self) {
        let gl = &self.workspace.context;
        let program = &self.gl_shaders.program;
        let bg_color_location = gl.get_uniform_location(program, "bgColor");
        let fg_color_location = gl.get_uniform_location(program, "fgColor");
        let px_range_location = gl.get_uniform_location(program, "pxRange");

        gl.use_program(Some(program));
        gl.uniform4f(
            bg_color_location.as_ref(),
            self.background_color.r,
            self.background_color.g,
            self.background_color.b,
            self.background_color.a
        );
        gl.uniform4f(
            fg_color_location.as_ref(),
            self.color.r,
            self.color.g,
            self.color.b,
            self.color.a,
        );
        gl.uniform1f(
            px_range_location.as_ref(),
            FontRenderInfo::MSDF_PARAMS.range as f32
        );
    }

    pub fn display(&self) {
        let gl = &self.workspace.context;
        let program = &self.gl_shaders.program;

        gl.use_program(Some(&self.gl_shaders.program));

        let position_location = gl.get_attrib_location(program, "position");
        gl.enable_vertex_attrib_array(position_location as u32);
        gl.bind_buffer(
            WebGlRenderingContext::ARRAY_BUFFER,
            Some(&self.gl_vertex_buf)
        );
        gl.vertex_attrib_pointer_with_i32(
            position_location as u32,
            2,
            WebGlRenderingContext::FLOAT,
            false,
            0,
            0
        );

        gl.bind_texture(Context::TEXTURE_2D, Some(&self.gl_msdf_texture));

        let tex_coord_location = gl.get_attrib_location(program, "texCoord");
        assert!(tex_coord_location >= 0);
        gl.enable_vertex_attrib_array(tex_coord_location as u32);
        gl.bind_buffer(
            WebGlRenderingContext::ARRAY_BUFFER,
            Some(&self.gl_tex_coord_buf)
        );
        gl.vertex_attrib_pointer_with_i32(
            tex_coord_location as u32,
            2,
            WebGlRenderingContext::FLOAT,
            false,
            0,
            0
        );

//        gl.clear_color(0.0, 0.0, 0.0, 1.0);
//        gl.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);

        gl.draw_arrays(
            WebGlRenderingContext::TRIANGLES,
            0,
            (self.text.chars().count()*6) as i32,
        );
    }
}
//
//pub fn print_line(
//    context : &Context,
//    text : &str,
//
//    logger : &Logger
//)
//{
//
//    context.use_program(Some(&program));
//
//    // when
//
//    context.bind_texture(Context::TEXTURE_2D, msdf_texture.as_ref());
//    context.tex_parameteri(Context::TEXTURE_2D, Context::TEXTURE_WRAP_S, Context::CLAMP_TO_EDGE as i32);
//    context.tex_parameteri(Context::TEXTURE_2D, Context::TEXTURE_WRAP_T, Context::CLAMP_TO_EDGE as i32);
//    context.tex_parameteri(Context::TEXTURE_2D, Context::TEXTURE_MIN_FILTER, Context::LINEAR as i32);
////    logger.trace(|| format!("{:?}", res.as_ref().expect_err("No error?")));
//    context.uniform1i(msdf_location.as_ref(), 0);
//    context.uniform1i(msdf_size_location.as_ref(), 16);
//
//
//    let y_max = y + size;
//    let vertices= (0..text.len()).map[
//        -1.0, -1.0, 0.0,
//        -1.0,  1.0, 0.0,
//         1.0,  1.0, 0.0,
//         1.0,  1.0, 0.0,
//         1.0, -1.0, 0.0,
//        -1.0, -1.0, 0.0
//    ];
//
//    let buffer = context.create_buffer().unwrap();
//    context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&buffer));
//
//    // Note that `Float32Array::view` is somewhat dangerous (hence the
//    // `unsafe`!). This is creating a raw view into our module's
//    // `WebAssembly.Memory` buffer, but if we allocate more pages for ourself
//    // (aka do a memory allocation in Rust) it'll cause the buffer to change,
//    // causing the `Float32Array` to be invalid.
//    //
//    // As a result, after `Float32Array::view` we have to be very careful not to
//    // do any memory allocations before it's dropped.
//    unsafe {
//        let vert_array = js_sys::Float32Array::view(&vertices);
//
//        context.buffer_data_with_array_buffer_view(
//            WebGlRenderingContext::ARRAY_BUFFER,
//            &vert_array,
//            WebGlRenderingContext::STATIC_DRAW,
//        );
//    }
//
//    context.vertex_attrib_pointer_with_i32(0, 3, WebGlRenderingContext::FLOAT, false, 0, 0);
//    context.enable_vertex_attrib_array(0);
//
//    context.clear_color(0.0, 0.0, 0.0, 1.0);
//    context.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);
//
//    context.draw_arrays(
//        WebGlRenderingContext::TRIANGLES,
//        0,
//        (vertices.len() / 3) as i32,
//    );
//}