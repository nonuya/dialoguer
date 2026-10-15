use core::dialog::{DialogManager, DialogPlayer, PlayerCommand, PlayerEffect};
use std::{path::PathBuf, rc::Rc};

use crate::{
  project::{ProjectAssets, ProjectManifest},
  runtime::EditorRuntime,
  ui::{UiCommand, UiState, draw_editor, handle_ui_command},
};
use winit::{
  event::KeyEvent,
  keyboard::{KeyCode, PhysicalKey},
};

pub struct App {
  assets: ProjectAssets,
  runtime: EditorRuntime,
  ui_state: UiState,
}

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

    Ok(Self {
      assets,
      runtime,
      ui_state,
    })
  }

  pub fn update(&mut self, deltatime: f32) {
    if self
      .runtime
      .dialog_player
      .as_ref()
      .is_some_and(|p| p.is_waiting() && !self.runtime.animator.is_timer_playing())
    {
      self.handle_player_command(PlayerCommand::Tick(deltatime));
    }

    if self
      .runtime
      .dialog_player
      .as_ref()
      .is_some_and(|p| p.is_next())
    {
      self.handle_player_command(PlayerCommand::Continue);
    }

    self
      .runtime
      .animator
      .update(deltatime, &mut self.runtime.model, &self.assets.model_def);
  }

  fn handle_player_command(&mut self, command: PlayerCommand) {
    let Some(player) = self.runtime.dialog_player.as_mut() else {
      return;
    };

    let output = player.handle(
      command,
      &self.assets.model_def,
      &self.assets.dialog_mgr,
      &self.assets.motion_mgr,
      &self.runtime.animator,
    );

    if !matches!(output.status, core::dialog::PlayerHandleStatus::Idle) {
      self.ui_state.current_container_index = Some(output.current_index);
      self.ui_state.current_dialog_name = Some(player.get_current_dialog_name());
    }

    if let core::dialog::PlayerHandleStatus::ShowChoices(choices) = output.status {
      for (i, c) in choices.iter().enumerate() {
        println!("{}) {c}", i + 1);
      }
    }

    for effect in output.effects {
      match effect {
        PlayerEffect::SetMainChoicer(id) => {
          println!("SetMainChoicer")
        }
        PlayerEffect::RemoveEnum { name, is_deferred } => {
          if is_deferred {
            self.runtime.animator.deferred_remove_enum(name);
          } else {
            self
              .runtime
              .animator
              .get_mut_parameter_state()
              .remove_enum(&name, &self.assets.model_def);
          }
        }
        PlayerEffect::Text(who, text) => {
          println!("{who}: {text}");
        }
        PlayerEffect::SetView(view) => {
          self.runtime.animator.set_target_view(view);
        }
        PlayerEffect::SetParameterChange {
          changes,
          is_deferred,
        } => {
          for c in changes {
            if is_deferred {
              self.runtime.animator.deferred_set_parameter_change(c);
            } else {
              self
                .runtime
                .animator
                .get_mut_parameter_state()
                .set_parameter_change(c);
            }
          }
        }
        PlayerEffect::PlayAnimation(motion, looped) => {
          self.runtime.animator.play_motion(motion, looped);
        }
      }
    }
  }

  pub fn draw(&mut self, ui: &dear_imgui_rs::Ui) {
    let output = draw_editor(ui, &self.assets, &mut self.ui_state);

    for cmd in output.commands {
      match cmd {
        UiCommand::PlayDialog { skip } => {
          if let Some(node) = self.ui_state.graph_state.get_selected_node() {
            match self.ui_state.graph_state.export_to_dialog(node.id()) {
              Ok(entries) => {
                let dialog_mgr = DialogManager::from_entries(entries);
                let name = node.name();
                match dialog_mgr.build(name) {
                  Some(entry_point) => {
                    self.assets.dialog_mgr = dialog_mgr;
                    self.ui_state.is_dialog_playing = true;
                    self.runtime.dialog_player =
                      Some(DialogPlayer::new(name.as_str().into(), entry_point));

                    self.handle_player_command(PlayerCommand::Start(skip));
                  }
                  None => {
                    log::warn!("[Export] Entry point Node {name} doesnt exists")
                  }
                }
              }
              Err(e) => log::warn!("[Export] Failed to Play: {e}"),
            }
          }
        }
        UiCommand::StopDialog => {
          self.ui_state.is_dialog_playing = false;
          self.ui_state.current_container_index = None;
          self.runtime.dialog_player = None;
          self.runtime.animator.clear_motion();
        }
        other => {
          handle_ui_command(other, &mut self.ui_state);
        }
      }
    }

    self.runtime.draw_model();
  }

  pub fn resize(&mut self, width: u32, height: u32) {}

  pub fn keyboard(&mut self, event: KeyEvent) {
    if !event.state.is_pressed() {
      match event.physical_key {
        PhysicalKey::Code(KeyCode::Space) => {
          self.handle_player_command(PlayerCommand::Continue);
        }
        PhysicalKey::Code(KeyCode::Digit1) => self.handle_player_command(PlayerCommand::Chose(0)),
        PhysicalKey::Code(KeyCode::Digit2) => self.handle_player_command(PlayerCommand::Chose(1)),
        PhysicalKey::Code(KeyCode::Digit3) => self.handle_player_command(PlayerCommand::Chose(2)),
        PhysicalKey::Code(KeyCode::Digit4) => self.handle_player_command(PlayerCommand::Chose(3)),
        PhysicalKey::Code(KeyCode::Digit5) => self.handle_player_command(PlayerCommand::Chose(4)),
        PhysicalKey::Code(KeyCode::Digit6) => self.handle_player_command(PlayerCommand::Chose(5)),
        _ => {}
      }
    }
  }
}
