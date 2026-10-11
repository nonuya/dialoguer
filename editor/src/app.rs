use std::{path::PathBuf, rc::Rc};

use crate::{
  project::{ProjectAssets, ProjectManifest}, runtime::EditorRuntime, ui::{UiState, draw_editor},
};
use winit::event::KeyEvent;

pub struct App {
  assets: ProjectAssets,
  runtime: EditorRuntime,
  ui_state: UiState,
  // ui: EditorUI,
}

// EditorState for ImGUI mutable
// tal vez así pueda usar funciones en vez de estructuras, así evitaría el &mut self que me jode bastante

impl App {
  pub fn new(
    model_path: PathBuf,
    gl: Rc<glow::Context>,
    imgui_context: &dear_imgui_rs::Context,
    imgui_renderer: &mut dear_imgui_glow::GlowRenderer,
  ) -> anyhow::Result<Self> {
    let manifest = ProjectManifest::from_path(&model_path)?;
    let assets = ProjectAssets::load(&manifest)?;
    let runtime = EditorRuntime::new(&manifest, &assets, gl)?;
    let ui_state = UiState::new(&assets, &runtime, imgui_context, imgui_renderer);

    // let ui = EditorUI::new(&core, imgui_context, imgui_renderer);

    Ok(Self {
      assets,
      runtime,
      ui_state,
    })
  }

  pub fn update(&mut self, deltatime: f32) {
    // self.core.update(deltatime);

    /*

    self
      .animator
      .update(deltatime, &mut self.model, &self.enummap);

    if !self.layout.is_main_layout() {
      self
        .layout
        .mut_enum_parameter_state()
        .update_model(deltatime, &mut self.model);
      self.model.update_parameters();
    }*/
  }

  pub fn draw(&mut self, ui: &dear_imgui_rs::Ui) {
    draw_editor(ui, &self.assets, &mut self.ui_state);

    // self.core.draw_model();
    // let mut layout = self.layout.get_mut();
    // layout.draw(ui);
    /*
    self.texture_target.draw(|| {
      self
        .model_renderer
        .draw(&self.model, self.animator.get_matrix());
      self
        .blackscreen_renderer
        .draw(self.animator.blackscreen_alpha());
    });

    let ctx = EditorContext {
      model: &mut self.model,
      animator: &mut self.animator,
      enummap: &mut self.enummap,
      dialog_mgr: &mut self.dialog_mgr,
      dialog_player: &mut self.dialog_player,
      motion_mgr: &self.motion_mgr,
      enummap_path: &self.enummap_path,
      dialog_path: &self.dialog_path,
    };
    self.layout.draw(ui, ctx);

    ui.window("Preview").build(|| {
      // Put this section in comment if you wanna work with both of your hands
      let available = ui.content_region_avail();

      if available[0] <= 0.0 || available[1] <= 0.0 {
        return;
      }

      let layout = self.model_renderer.layout();

      let aspect_ratio = layout.width / layout.height;

      let draw_size = if available[0] / available[1] > aspect_ratio {
        [available[1] * aspect_ratio, available[1]]
      } else {
        [available[0], available[0] / aspect_ratio]
      };

      let cursor = ui.cursor_pos();

      ui.set_cursor_pos([
        cursor[0] + (available[0] - draw_size[0]) * 0.5,
        cursor[1] + (available[1] - draw_size[1]) * 0.5,
      ]);

      Image::new(ui, self.texture_id, draw_size)
        .uv0([0.0, 1.0])
        .uv1([1.0, 0.0])
        .build();
    });
    */
  }

  pub fn resize(&mut self, width: u32, height: u32) {}

  pub fn keyboard(&mut self, event: KeyEvent) {
    /*
    if let Some(dialog_player) = &mut self.dialog_player {
      if !event.state.is_pressed() {
        match event.physical_key {
          PhysicalKey::Code(KeyCode::KeyI) => {
            dialog_player.play();
          }
          PhysicalKey::Code(KeyCode::Space) => {
            dialog_player.next();
          }
          PhysicalKey::Code(KeyCode::Digit1) => dialog_player.handle_input(0),
          PhysicalKey::Code(KeyCode::Digit2) => dialog_player.handle_input(1),
          PhysicalKey::Code(KeyCode::Digit3) => dialog_player.handle_input(2),
          PhysicalKey::Code(KeyCode::Digit4) => dialog_player.handle_input(3),
          PhysicalKey::Code(KeyCode::Digit5) => dialog_player.handle_input(4),
          PhysicalKey::Code(KeyCode::Digit6) => dialog_player.handle_input(5),
          _ => {}
        }
      }
    }*/
  }
}
