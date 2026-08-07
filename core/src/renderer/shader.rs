use anyhow::Context;
use glow::HasContext;

pub fn create_program_from_source(
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
