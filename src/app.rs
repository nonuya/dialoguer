use crate::live2d::{
  self,
  animator::{Animator, Value},
};
use glutin::display::GlDisplay;
use log::debug;
use winit::event::KeyEvent;
use std::{path::PathBuf, rc::Rc};

pub struct App {
  gl: Rc<glow::Context>,
  renderer: live2d::Renderer,
  model: live2d::Model,
  mvp: glam::Mat4,
  animator: Animator,
}

impl App {
  pub fn new(display: &impl GlDisplay) -> anyhow::Result<Self> {
    let gl = Rc::new(unsafe {
      glow::Context::from_loader_function_cstr(|symbol| display.get_proc_address(symbol))
    });

    let renderer = live2d::Renderer::new(gl.clone())?;
    let model = live2d::Model::new(gl.clone(), PathBuf::from("assets/models/iav_014_2"))?;

    let motion = cubism::motion::Motion::from_motion3_json(
      "assets/models/iav_014_2/motions/Wait04.anim.motion3.json",
    )?;
    let mut animator = Animator::new();
    animator.play_motion(motion, false);

    Ok(Self {
      gl,
      renderer,
      model,
      mvp: glam::Mat4::IDENTITY,
      animator,
    })
  }

  pub fn update(&mut self, deltatime: f32) {
    self.animator.update(deltatime, &mut self.model);
  }

  pub fn draw(&self) {
    self.renderer.draw(&self.model, &self.mvp);
  }

  pub fn resize(&mut self, width: u32, height: u32) {
    self.renderer.resize(width, height);
  }

  pub fn keyboard(&mut self, event: KeyEvent) {
    // self.animator.set_parameter("Param9".to_string(), Value::smooth(0.0, 1.0));
  }
}
