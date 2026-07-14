use anyhow::Context;
use glow::HasContext;

const VERTEX_SHADER_SRC_SETUP_MASK: &str = include_str!("./shaders/VertShaderSrcSetupMask.vert");
const FRAGMENT_SHADER_SRC_SETUP_MASK: &str = include_str!("./shaders/FragShaderSrcSetupMask.frag");
const VERTEX_SHADER_SRC: &str = include_str!("./shaders/VertShaderSrc.vert");
const FRAGMENT_SHADER_SRC: &str = include_str!("./shaders/FragShaderSrc.frag");
const VERTEX_SHADER_SRC_MASKED: &str = include_str!("./shaders/VertShaderSrcMasked.vert");
const FRAGMENT_SHADER_SRC_MASK: &str = include_str!("./shaders/FragShaderSrcMask.frag");
const FRAGMENT_SHADER_SRC_MASK_INVERTED: &str =
  include_str!("./shaders/FragShaderSrcMaskInverted.frag");

pub struct GlobalShaders {
  pub(in crate::live2d) setup: Shader,
  pub(in crate::live2d) normal: Shader,
  pub(in crate::live2d) masked: Shader,
  pub(in crate::live2d) inverted_mask: Shader,
}

impl GlobalShaders {
  pub fn new(gl: &glow::Context) -> anyhow::Result<Self> {
    let setup = create_shader_from_source(
      gl,
      VERTEX_SHADER_SRC_SETUP_MASK,
      FRAGMENT_SHADER_SRC_SETUP_MASK,
    )?;
    let normal = create_shader_from_source(gl, VERTEX_SHADER_SRC, FRAGMENT_SHADER_SRC)?;
    let masked = create_shader_from_source(gl, VERTEX_SHADER_SRC_MASKED, FRAGMENT_SHADER_SRC_MASK)?;
    let inverted_mask = create_shader_from_source(
      gl,
      VERTEX_SHADER_SRC_MASKED,
      FRAGMENT_SHADER_SRC_MASK_INVERTED,
    )?;

    Ok(Self {
      setup,
      normal,
      masked,
      inverted_mask,
    })
  }
}

fn create_shader_from_source(
  gl: &glow::Context,
  vertex_src: &str,
  fragment_src: &str,
) -> anyhow::Result<Shader> {
  let program = create_program_from_source(gl, vertex_src, fragment_src)?;

  unsafe {
    Ok(
      Shader {
        program,
        matrix: gl.get_uniform_location(program, "u_matrix"),
        texture0: gl.get_uniform_location(program, "s_texture0"),
        texture1: gl.get_uniform_location(program, "s_texture1"),
        channel_flag: gl.get_uniform_location(program, "u_channelFlag"),
        base_color: gl.get_uniform_location(program, "u_baseColor"),
        multiply_color: gl.get_uniform_location(program, "u_multiplyColor"),
        screen_color: gl.get_uniform_location(program, "u_screenColor"),
        clip_matrix: gl.get_uniform_location(program, "u_clipMatrix"),
      }) }
}

fn create_program_from_source(
  gl: &glow::Context,
  vertex_src: &str,
  fragment_src: &str,
) -> anyhow::Result<glow::Program> {
  unsafe {
    let program = gl
      .create_program()
      .map_err(anyhow::Error::msg)
      .context("Failed to create OpenGL program.")?;

    let vertex_shader = gl
      .create_shader(glow::VERTEX_SHADER)
      .map_err(anyhow::Error::msg)
      .context("Failed to create vertex shader.")?;

    gl.shader_source(vertex_shader, vertex_src);
    gl.compile_shader(vertex_shader);

    if !gl.get_shader_compile_status(vertex_shader) {
      let log = gl.get_shader_info_log(vertex_shader);
      gl.delete_shader(vertex_shader);
      gl.delete_program(program);
      anyhow::bail!("Vertex shader compilation failed:\n{}", log);
    }

    let fragment_shader = gl
      .create_shader(glow::FRAGMENT_SHADER)
      .map_err(anyhow::Error::msg)
      .context("Failed to create fragment shader")?;

    gl.shader_source(fragment_shader, fragment_src);
    gl.compile_shader(fragment_shader);

    if !gl.get_shader_compile_status(fragment_shader) {
      let log = gl.get_shader_info_log(fragment_shader);
      gl.delete_shader(vertex_shader);
      gl.delete_shader(fragment_shader);
      gl.delete_program(program);
      anyhow::bail!("Fragment shader compilation failed:\n{}", log);
    }

    gl.attach_shader(program, vertex_shader);
    gl.attach_shader(program, fragment_shader);

    gl.link_program(program);

    if !gl.get_program_link_status(program) {
      let log = gl.get_program_info_log(program);

      gl.detach_shader(program, vertex_shader);
      gl.detach_shader(program, fragment_shader);

      gl.delete_shader(vertex_shader);
      gl.delete_shader(fragment_shader);
      gl.delete_program(program);

      anyhow::bail!("Program linking failed:\n{}", log);
    }

    // Ya no son necesarios después del link
    gl.detach_shader(program, vertex_shader);
    gl.detach_shader(program, fragment_shader);

    gl.delete_shader(vertex_shader);
    gl.delete_shader(fragment_shader);

    Ok(program)
  }
}

pub struct Shader {
  pub program: glow::Program,

  pub texture0: Option<glow::UniformLocation>,
  pub texture1: Option<glow::UniformLocation>,
  pub channel_flag: Option<glow::UniformLocation>,
  pub base_color: Option<glow::UniformLocation>,
  pub multiply_color: Option<glow::UniformLocation>,
  pub screen_color: Option<glow::UniformLocation>,
  pub clip_matrix: Option<glow::UniformLocation>,
  pub matrix: Option<glow::UniformLocation>,
}
