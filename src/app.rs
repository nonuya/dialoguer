use std::{collections::HashMap, fs::File, path::PathBuf, rc::Rc};

use glutin::display::GlDisplay;
use log::debug;
use winit::event::KeyEvent;

use crate::scene::Scene;

// TODO: Add Speed for Animations

pub struct App {
  gl: Rc<glow::Context>,
  scene: Scene,
}

impl App {
  pub fn new(display: &impl GlDisplay) -> anyhow::Result<Self> {
    let gl = Rc::new(unsafe {
      glow::Context::from_loader_function_cstr(|symbol| display.get_proc_address(symbol))
    });

    let model_path = PathBuf::from("assets/models/iav_013_2");
    debug!("Loading Scene for model '{}'", model_path.display());
    let scene = Scene::load_from_model_path(gl.clone(), model_path)?;
    debug!("SCENE LOADED");
    
    Ok(Self {
      gl,
      scene,
    })
  }

  pub fn update(&mut self, deltatime: f32) {
    self.scene.update(deltatime);
  }

  pub fn draw(&self) {
    self.scene.draw();
  }

  pub fn resize(&mut self, width: u32, height: u32) {
    self.scene.resize(width, height);
  }

  pub fn keyboard(&mut self, event: KeyEvent) {
    self.scene.keyboard(event);
  }
}
