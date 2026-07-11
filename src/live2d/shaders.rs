use glium::{Display, Program, glutin::surface::WindowSurface};

static VERTEX_SHADER_SRC: &str = include_str!("./shaders/VertShaderSrc.vert");
static VERTEX_SHADER_SRC_SETUP_MASK: &str =
  include_str!("./shaders/VertShaderSrcSetupMask.vert");
static VERTEX_SHADER_SRC_MASKED: &str = include_str!("./shaders/VertShaderSrcMasked.vert");
static FRAGMENT_SHADER_SRC_SETUP_MASK: &str =
  include_str!("./shaders/FragShaderSrcSetupMask.frag");
static FRAGMENT_SHADER_SRC: &str = include_str!("./shaders/FragShaderSrc.frag");
static FRAGMENT_SHADER_SRC_MASK: &str = include_str!("./shaders/FragShaderSrcMask.frag");
static FRAGMENT_SHADER_SRC_MASK_INVERTED: &str =
  include_str!("./shaders/FragShaderSrcMaskInverted.frag");

pub struct GlobalShaders {
  pub setup: Program,
  pub normal: Program,
  pub masked: Program,
  pub inverted_mask: Program
}

pub fn load_global_shaders(display: &Display<WindowSurface>) -> anyhow::Result<GlobalShaders> {
  let shaders = GlobalShaders {
    setup: glium::Program::from_source(
      display,
      VERTEX_SHADER_SRC_SETUP_MASK,
      FRAGMENT_SHADER_SRC_SETUP_MASK,
      None,
    )?,
    normal: glium::Program::from_source(display, VERTEX_SHADER_SRC, FRAGMENT_SHADER_SRC, None)?,
    masked: glium::Program::from_source(
      display,
      VERTEX_SHADER_SRC_MASKED,
      FRAGMENT_SHADER_SRC_MASK,
      None,
    )?,
    inverted_mask:glium::Program::from_source(
      display,
      VERTEX_SHADER_SRC_MASKED,
      FRAGMENT_SHADER_SRC_MASK_INVERTED,
      None,
    )?,
  };

  Ok(shaders)
}
