use std::path::PathBuf;

use glium::{Display, glutin::surface::WindowSurface};

use crate::live2d;

pub struct App {
  model: live2d::Model,
  shaders: live2d::GlobalShaders,
}

impl App {
  pub fn new(display: &Display<WindowSurface>) -> anyhow::Result<Self> {
    let shaders = live2d::load_global_shaders(display)?;
    let model = live2d::Model::new(display, PathBuf::from("assets/models/iav_024_2"))?;

    Ok(Self { model, shaders })
  }

  pub fn draw(&self, display: &Display<WindowSurface>) {
    live2d::draw_masks(display, &self.model, &self.shaders);

    let mut frame = display.draw();
    live2d::draw_model(&mut frame, &self.model, &self.shaders);
    frame.finish().unwrap();
  }
}
