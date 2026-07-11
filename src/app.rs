use std::path::PathBuf;

use glium::{Display, Frame, glutin::surface::WindowSurface};

use crate::live2d::{
  self,
  renderer::{draw_masks, draw_model},
};

pub struct App {
  model: live2d::model::Model,
  shaders: live2d::shaders::GlobalShaders,
}

impl App {
  pub fn new(display: &Display<WindowSurface>) -> anyhow::Result<Self> {
    let shaders = live2d::shaders::load_global_shaders(display)?;
    let model = live2d::model::Model::new(display, PathBuf::from("assets/models/iav_024_2"))?;

    Ok(Self { model, shaders })
  }

  pub fn draw(&self, display: &Display<WindowSurface>) {
    draw_masks(display, &self.model, &self.shaders);
    let mut frame = display.draw();
    draw_model(&mut frame, &self.model, &self.shaders);
    //draw_model_test(&mut frame, display, &self.model);
    frame.finish().unwrap();
  }
}
