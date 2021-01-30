
use std::collections::HashMap;
use std::mem;

use alacritty_terminal::term::SizeInfo;
use alacritty_terminal::graphics::Graphics;
use alacritty_terminal::grid::GraphicsRow;

use crate::gl;
use crate::gl::types::*;
use crate::renderer;

/// Shader sources for rect rendering program.
static IMG_SHADER_F: &str = include_str!("../../res/img.f.glsl");
static IMG_SHADER_V: &str = include_str!("../../res/img.v.glsl");

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct ImgCoords {
    // Normalized screen coordinates.
    x: f32,
    y: f32,

    // Normalized texture coordinates.
    tx: f32,
    ty: f32,
}

impl ImgCoords {
    fn new(x: f32, y: f32, tx: f32, ty: f32) -> Self {
        ImgCoords {x, y, tx, ty}
    }
}

#[derive(Debug)]
pub struct ImageRenderer {
    // GL buffer objects.
    vao: GLuint,
    vbo: GLuint,

    program: ImgShaderProgram,

    // mapping between the image id and the texture id
    pub texture_map: HashMap<usize, u32>,
}

impl ImageRenderer {
    /// Done
    pub fn new() -> Result<Self, renderer::Error> {
        let mut vao: GLuint = 0;
        let mut vbo: GLuint = 0;
        let program = ImgShaderProgram::new()?;

        unsafe {
            // Allocate buffers.
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);

            gl::BindVertexArray(vao);

            // VBO binding is not part of VAO itself, but VBO binding is stored in attributes.
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);

            let mut attribute_offset = 0;

            // Position.
            gl::VertexAttribPointer(
                0,
                2,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<ImgCoords>() as i32,
                attribute_offset as *const _,
            );
            gl::EnableVertexAttribArray(0);
            attribute_offset += mem::size_of::<f32>() * 2;

            // Image Texture Coords.
            gl::VertexAttribPointer(
                1,
                2,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<ImgCoords>() as i32,
                attribute_offset as *const _,
            );
            gl::EnableVertexAttribArray(1);

            // Reset buffer bindings.
            gl::BindVertexArray(0);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        }

        Ok(Self { vao, vbo, program, texture_map: HashMap::new() })
    }

    pub fn draw(&mut self, size_info: &SizeInfo, graphics_row: &GraphicsRow, line: usize, column: usize, image_id: u32) {

        // Prepare rect rendering state.
        unsafe {
            // Remove padding from viewport.
            gl::Viewport(0, 0, size_info.width() as i32, size_info.height() as i32);
            gl::BlendFuncSeparate(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA, gl::SRC_ALPHA, gl::ONE);
        }

        let vertices = self.gen_vertices(size_info, graphics_row, line, column);

        unsafe {
            gl::UseProgram(self.program.id);

            // Bind VAO to enable vertex attribute slots.
            gl::BindVertexArray(self.vao);

            // Bind VBO only once for buffer data upload only.
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);

            gl::ActiveTexture(gl::TEXTURE0);

            gl::BindTexture(gl::TEXTURE_2D, image_id);
        }

        unsafe {
            // Upload accumulated vertices.
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * mem::size_of::<ImgCoords>()) as isize,
                vertices.as_ptr() as *const _,
                gl::STREAM_DRAW,
            );

            // Draw all vertices as list of triangles.
            gl::DrawArrays(gl::TRIANGLES, 0, vertices.len() as i32);

            // Disable program.
            gl::UseProgram(0);

            // Reset buffer bindings to nothing.
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::BindVertexArray(0);
        }

        // Activate regular state again.
        unsafe {
            // Reset blending strategy.
            gl::BlendFunc(gl::SRC1_COLOR, gl::ONE_MINUS_SRC1_COLOR);

            // Restore viewport with padding.
            let padding_x = size_info.padding_x() as i32;
            let padding_y = size_info.padding_y() as i32;
            let width = size_info.width() as i32;
            let height = size_info.height() as i32;
            gl::Viewport(padding_x, padding_y, width - 2 * padding_x, height - 2 * padding_y);
        }
    }

    // Generate co-ordinates of triangles required by OpenGL
    fn gen_vertices(&self, size_info: &SizeInfo, image_row: &GraphicsRow, line: usize, column: usize) -> Vec<ImgCoords> {
        let mut vertices: Vec<ImgCoords> = Vec::with_capacity(6);
        let cell_height = size_info.cell_height();
        let cell_width = size_info.cell_width();
        let screen_width = size_info.width();
        let screen_height = size_info.height();
        let image_width = image_row.raw.width as f32;
        let image_height = image_row.raw.height as f32;
        let top_left_x = column as f32 * cell_width;
        let top_left_y = line as f32 * cell_height;
        let img_top_left_x = (column - image_row.start_column.0) as f32 * cell_width;
        let img_top_left_y = image_row.offset_y as f32 * cell_height;
        let normalized_x = 2.0 * top_left_x / screen_width - 1.0;
        let normalized_y = 2.0 * top_left_y / screen_height - 1.0;
        let normalized_img_x = img_top_left_x / image_width;
        let normalized_img_y = img_top_left_y / image_height;
        let top_left_vertex = ImgCoords::new(normalized_x, normalized_y, normalized_img_x, normalized_img_y);

        let mut img_cell_width = (column + 1) as f32 * cell_width - image_width;
        if img_cell_width < 0.0 {
            img_cell_width = cell_width;
        } else {
            img_cell_width = cell_width - img_cell_width;
        }
        let mut img_cell_height = (image_row.offset_y + 1) as f32 * cell_height - image_height;
        if img_cell_height < 0.0 {
            img_cell_height = cell_height;
        } else {
            img_cell_height = cell_height - img_cell_height;
        }

        let normalized_right_x = 2.0 * (top_left_x + img_cell_width) / screen_width - 1.0;
        let normalized_bottom_y = 2.0 * (top_left_y + img_cell_height) / screen_height - 1.0;
        let normalized_image_right_x = normalized_img_x + img_cell_width / image_width;
        let normalized_image_bottom_y = normalized_img_y + img_cell_height / image_height;
        let bottom_left_vertex = ImgCoords::new(normalized_x, normalized_bottom_y, normalized_img_x, normalized_image_bottom_y);
        let top_right_vertex = ImgCoords::new(normalized_right_x, normalized_y, normalized_image_right_x, normalized_img_y);
        let bottom_right_vertex = ImgCoords::new(normalized_right_x, normalized_bottom_y, normalized_image_right_x, normalized_image_bottom_y);
        vertices.push(top_left_vertex);
        vertices.push(bottom_left_vertex);
        vertices.push(bottom_right_vertex);
        vertices.push(top_left_vertex);
        vertices.push(bottom_right_vertex);
        vertices.push(top_right_vertex);
        vertices
    }

    pub fn get_tex_id(&self, id: usize) -> Option<&u32> {
        self.texture_map.get(&id)
    }

    pub fn add_img(&mut self, image: &Graphics) -> u32 {
        let mut id = 0;
        let width = image.width as i32;
        let height = image.height as i32;
        unsafe {
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
            gl::GenTextures(1, &mut id);
            gl::BindTexture(gl::TEXTURE_2D, id);

            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGB as i32,
                width,
                height,
                0,
                gl::RGB,
                gl::UNSIGNED_BYTE,
                image.rgb.as_ptr() as *const _,
            );

            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
        let counter_id = image.id;
        self.texture_map.entry(counter_id).or_insert(id);
        id
    }
}

/// Image drawing program.
#[derive(Debug)]
pub struct ImgShaderProgram {
    /// Program id.
    id: GLuint,
}

impl ImgShaderProgram {
    pub fn new() -> Result<Self, renderer::ShaderCreationError> {
        let vertex_shader = renderer::create_shader(gl::VERTEX_SHADER, IMG_SHADER_V)?;
        let fragment_shader = renderer::create_shader(gl::FRAGMENT_SHADER, IMG_SHADER_F)?;
        let program = renderer::create_program(vertex_shader, fragment_shader)?;

        unsafe {
            gl::DeleteShader(fragment_shader);
            gl::DeleteShader(vertex_shader);
            gl::UseProgram(program);
        }

        let shader = Self { id: program };

        unsafe { gl::UseProgram(0) }

        Ok(shader)
    }
}

impl Drop for ImgShaderProgram {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}
