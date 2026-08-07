use crate::renderer::shader::create_program_from_source;
use glow::HasContext;
use std::rc::Rc;

const FRAGMENT_SHADER_BLACK_SCREEN: &str = include_str!("./shaders/FragShaderBlackScreen.frag");
const VERTEX_SHADER_BLACK_SCREEN: &str = include_str!("./shaders/VertShaderBlackScreen.vert");

struct Shader {
  program: glow::Program,

  alpha: Option<glow::UniformLocation>,
}

fn create_shader_from_source(
  gl: &glow::Context,
  vertex_src: &str,
  fragment_src: &str,
) -> anyhow::Result<Shader> {
  let program = create_program_from_source(gl, vertex_src, fragment_src)?;

  unsafe {
    Ok(Shader {
      program,
      alpha: gl.get_uniform_location(program, "u_alpha"),
    })
  }
}

pub struct BlackScreenRenderer {
  gl: Rc<glow::Context>,
  shader: Shader,
  vao: glow::VertexArray,
  vbo: glow::Buffer,
}

impl BlackScreenRenderer {
  pub fn new(gl: Rc<glow::Context>) -> anyhow::Result<Self> {
    let shader = create_shader_from_source(
      &gl,
      VERTEX_SHADER_BLACK_SCREEN,
      FRAGMENT_SHADER_BLACK_SCREEN,
    )?;

    let (vao, vbo) = unsafe {
      let vertices: [f32; 12] = [
        -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0, 1.0,
      ];

      let vao = gl.create_vertex_array().map_err(anyhow::Error::msg)?;
      let vbo = gl.create_buffer().map_err(anyhow::Error::msg)?;

      gl.bind_vertex_array(Some(vao));
      gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));

      gl.buffer_data_u8_slice(
        glow::ARRAY_BUFFER,
        bytemuck::cast_slice(&vertices),
        glow::STATIC_DRAW,
      );

      gl.enable_vertex_attrib_array(0);
      gl.vertex_attrib_pointer_f32(
        0,
        2,
        glow::FLOAT,
        false,
        2 * std::mem::size_of::<f32>() as i32,
        0,
      );

      gl.bind_vertex_array(None);
      gl.bind_buffer(glow::ARRAY_BUFFER, None);

      (vao, vbo)
    };

    Ok(Self {
      gl,
      vao,
      vbo,
      shader,
    })
  }

  pub fn draw(&self, alpha: f32) {
    if alpha <= 0.0 {
      return;
    }

    unsafe {
      self.gl.enable(glow::BLEND);
      self
        .gl
        .blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);

      self.gl.disable(glow::DEPTH_TEST);

      self.gl.use_program(Some(self.shader.program));
      self
        .gl
        .uniform_1_f32(self.shader.alpha.as_ref(), alpha);

      self.gl.bind_vertex_array(Some(self.vao));
      self.gl.draw_arrays(glow::TRIANGLES, 0, 6);

      self.gl.bind_vertex_array(None);
      self.gl.use_program(None);

      self.gl.enable(glow::DEPTH_TEST);
      self.gl.disable(glow::BLEND);
    }
  }
}

impl Drop for BlackScreenRenderer {
  fn drop(&mut self) {
    unsafe {
      self.gl.delete_buffer(self.vbo);
      self.gl.delete_program(self.shader.program);
      self.gl.delete_vertex_array(self.vao);
    }
  }
}
