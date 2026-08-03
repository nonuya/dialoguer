use std::{path::PathBuf, rc::Rc};

use dear_imgui_glow::GlowRenderer;
use dear_imgui_rs::*;
use glutin::display::GlDisplay;
use log::debug;
use winit::event::KeyEvent;

// TODO: Add Speed for Animations

pub struct App {}

impl App {
  pub fn new() -> anyhow::Result<Self> {
    Ok(Self {})
  }

  pub fn update(&mut self, deltatime: f32) {}

  pub fn draw(&self, ui: &mut Ui) {
    // Main window content
    ui.window("Hello, Dear ImGui Glow!")
      .size([400.0, 300.0], Condition::FirstUseEver)
      .build(|| {
        ui.text("Welcome to Dear ImGui with Glow backend!");
        ui.separator();

        ui.text(&format!(
          "Application average {:.3} ms/frame ({:.1} FPS)",
          1000.0 / ui.io().framerate(),
          ui.io().framerate()
        ));

        // Toggle software cursor (ImGui-drawn cursor)
        ui.text("Modern texture management features:");
        ui.bullet_text("RENDERER_HAS_TEXTURES backend flag");
        ui.bullet_text("Complete ImTextureData system");
        ui.bullet_text("Texture registration and updates");
      });
  }

  pub fn resize(&mut self, width: u32, height: u32) {}

  pub fn keyboard(&mut self, event: KeyEvent) {}
}
