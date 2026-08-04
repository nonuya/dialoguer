use std::{fs::{self, File}, path::{Path, PathBuf}, rc::Rc};

use anyhow::Context;
use chumsky::Parser;
use glam::vec3;
use log::{debug, warn};
use winit::{event::KeyEvent, keyboard::{KeyCode, PhysicalKey}};

use crate::{dialog, live2d::{self, Model, Renderer, config}};

pub struct Scene {
  model: live2d::Model,
  animator: live2d::animator::Animator,
  motion_mgr: live2d::animator::MotionManager,
  dialog_mgr: dialog::manager::DialogManager,
  dialog_player: dialog::manager::DialogPlayer,
  dialog_path: PathBuf,
  enummap_path: PathBuf,
  enummap: live2d::animator::EnumMap,
  renderer: live2d::Renderer,
  mpv: glam::Mat4,
}

fn load_and_build_dialogs(dialog_path: &Path) -> anyhow::Result<(dialog::manager::DialogPlayer, dialog::manager::DialogManager)> {
  let dialog_src =
    fs::read_to_string(&dialog_path)
    .context(format!("Failed to read {}", dialog_path.display()))?;

  let dialog_tokens = dialog::parser::dialog_block_lexer()
    .parse(&dialog_src)
    .into_result()
    .map_err(|err| anyhow::anyhow!("Dialog Block Lexer {:#?}", err))?;
  let dialog_mgr =
    dialog::manager::DialogManager::new(dialog_tokens)
    .context("Failed to create DialogManager")?;

  let mut dialog_player = dialog::manager::DialogPlayer::new(dialog_mgr.build_idle());
  dialog_player.play();

  Ok((dialog_player, dialog_mgr))
}

impl Scene {
  pub fn load_from_model_path(gl: Rc<glow::Context>,  model_path: PathBuf) -> anyhow::Result<Self> {
    let renderer =
      live2d::Renderer::new(gl.clone(), config::MODEL_WIDTH, config::MODEL_HEIGHT)
      .context("Failed to create Live2D Renderer")?;

    let model_name = model_path
      .file_name()
      .ok_or_else(|| anyhow::anyhow!("Fail to get model name from '{}'", model_path.display()))?;
    
    let model_file =
      File::open(model_path.join(format!("{}.model3.json", model_name.display())))
      .context(format!("Failed to read .model3.json in '{}'", model_path.display()))?;

    let model3 =
      cubism::json::model::Model3::from_reader(model_file)
      .context(format!("Failed to parse '{}.model3.json'", model_name.display()))?;
    let model = live2d::Model::new(gl.clone(), &model_path, &model3)?;

    let mut enummap_path = model_path.join(model_name);
    enummap_path.set_extension("map");
    let enummap = live2d::animator::load_enum_map(&enummap_path)?;

    let motion_mgr =
      live2d::animator::MotionManager::new(&model_path, &model3)
      .context("Failed to read motions")?;

    let mut dialog_path = model_path.join(model_name);
    dialog_path.set_extension("dialog");
    let (dialog_player, dialog_mgr) = load_and_build_dialogs(&dialog_path)?;

    Ok(Self {
      model,
      renderer,
      motion_mgr,
      enummap_path,
      dialog_path,
      dialog_mgr,
      dialog_player,
      enummap,
      mpv: glam::Mat4::from_scale(vec3(2.0, 2.0, 1.0)),
      animator: live2d::animator::Animator::new(),
    })
  }

  fn reload_dialogs(&mut self) {
    debug!("Realoding dialogs and enummap...");
    match (
      load_and_build_dialogs(&self.dialog_path),
      live2d::animator::load_enum_map(&self.enummap_path)
      ) {
      (Ok((dialog_player, dialog_mgr)), Ok(enummap)) => {
        self.dialog_player = dialog_player;
        self.dialog_mgr = dialog_mgr;
        self.enummap = enummap;
        self.animator.stop_timer();
      },
      (Err(e), _) | (_, Err(e)) => {
            warn!("Failed to reload dialogs: {e}");
      }}
  }

  pub fn update(&mut self, deltatime: f32) {
    self.dialog_player.update(
      &mut self.animator,
      &self.dialog_mgr,
      &self.enummap,
      &self.motion_mgr
    );

    self.animator.update(deltatime, &mut self.model);
  }

  pub fn keyboard(&mut self, event: KeyEvent) {
    if !event.state.is_pressed() {
      match event.physical_key {
        PhysicalKey::Code(KeyCode::KeyI) => {
          self.dialog_player.play();
        },
        PhysicalKey::Code(KeyCode::Space) => {
          self.dialog_player.next();
        },
        PhysicalKey::Code(KeyCode::KeyR) => {
          self.reload_dialogs();
        },
        PhysicalKey::Code(KeyCode::Digit1) => self.dialog_player.handle_input(0),
        PhysicalKey::Code(KeyCode::Digit2) => self.dialog_player.handle_input(1),
        PhysicalKey::Code(KeyCode::Digit3) => self.dialog_player.handle_input(2),
        PhysicalKey::Code(KeyCode::Digit4) => self.dialog_player.handle_input(3),
        PhysicalKey::Code(KeyCode::Digit5) => self.dialog_player.handle_input(4),
        PhysicalKey::Code(KeyCode::Digit6) => self.dialog_player.handle_input(5),
        _ => {}
      }
    }
  }

  pub fn draw(&self) {
    self.renderer.draw(&self.model, &self.mpv);
  }

  pub fn resize(&mut self, width: u32, height: u32) {
    self.renderer.resize(width, height);
  }

  pub fn renderer(&self) -> &Renderer {
    &self.renderer
  }

  pub fn mut_model(&mut self) -> &mut Model {
    &mut self.model
  }
}
