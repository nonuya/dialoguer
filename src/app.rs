use std::{path::PathBuf, rc::Rc};
use glutin::display::GlDisplay;
use crate::live2d;

pub struct App {
  gl: Rc<glow::Context>,
  renderer: live2d::Renderer,
  model: live2d::Model,
  mvp: glam::Mat4,
}

impl App {
  pub fn new(display: &impl GlDisplay, width: u32, height: u32) -> anyhow::Result<Self> {
    let gl = Rc::new(unsafe {
      glow::Context::from_loader_function_cstr(|symbol| display.get_proc_address(symbol))
    });

    let renderer = live2d::Renderer::new(gl.clone())?;
    let model = live2d::Model::new(gl.clone(), PathBuf::from("assets/models/iav_013_2"))?;
    Ok(Self {
      gl,
      renderer,
      model,
      mvp: glam::Mat4::IDENTITY,
    })
  }

  pub fn draw(&self) {
    self.renderer.draw(&self.model, &self.mvp);
  }

  pub fn resize(&mut self, width: u32, height: u32) {
  }
}
