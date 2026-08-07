use anyhow::Context;
use chumsky::Parser;
use dear_imgui_glow::GlowRenderer;
use dear_imgui_rs::*;
use core::renderer::RenderContext;
use std::{
  fs::{self, File},
  path::PathBuf,
  rc::Rc,
};
use winit::{
  event::KeyEvent,
  keyboard::{KeyCode, PhysicalKey},
};

use crate::layout::{EditorContext, Layout};

pub struct App {
  ctx: Rc<RenderContext>,
  texture_id: TextureId,
  model: core::live2d::Model,
  enummap: core::live2d::animator::EnumMap,
  mvp: glam::Mat4,
  model_renderer: core::renderer::ModelRenderer,
  blackscreen_renderer: core::renderer::BlackScreenRenderer,
  texture_target: core::renderer::TextureTarget,
  animator: core::live2d::animator::Animator,
  dialog_mgr: core::dialog::DialogManager,
  dialog_player: Option<core::dialog::DialogPlayer>,
  motion_mgr: core::live2d::animator::MotionManager,
  layout: Layout,
}

impl App {
  pub fn new(
    model_path: PathBuf,
    renderer: &mut GlowRenderer,
    imgui_context: &dear_imgui_rs::Context,
  ) -> anyhow::Result<Self> {
    let gl = renderer.gl_context().unwrap().clone();

    let model_name = model_path
      .file_name()
      .ok_or_else(|| anyhow::anyhow!("Fail to get model name from '{}'", model_path.display()))?;

    let model_file =
      File::open(model_path.join(format!("{}.model3.json", model_name.display()))).context(
        format!("Failed to read .model3.json in '{}'", model_path.display()),
      )?;

    let model3 = cubism::json::model::Model3::from_reader(model_file).context(format!(
      "Failed to parse '{}.model3.json'",
      model_name.display()
    ))?;
    let mut model = core::live2d::Model::new(gl.clone(), &model_path, &model3)?;
    model.save_parameters();

    let mut enummap_path = model_path.join(model_name);
    enummap_path.set_extension("map");
    let enummap = core::live2d::animator::load_enum_map(&enummap_path)?;

    let ctx = Rc::from(core::renderer::RenderContext::from_gl(gl));

    let model_renderer = core::renderer::ModelRenderer::new(
      ctx.clone(),
      core::live2d::config::MODEL_WIDTH,
      core::live2d::config::MODEL_HEIGHT,
    )
    .context("Failed to create Live2D Renderer")?;
    let texture_target = core::renderer::TextureTarget::new(
      ctx.clone(),
      core::live2d::config::MODEL_WIDTH,
      core::live2d::config::MODEL_HEIGHT,
    )?;
    let blackscreen_renderer = core::renderer::BlackScreenRenderer::new(ctx.clone())?;

    let texture_id = TextureId::new(1000);
    renderer
      .texture_map_mut()
      .set(texture_id, texture_target.tex());

    let motion_mgr = core::live2d::animator::MotionManager::new(&model_path, &model3)
      .context("Failed to read motions")?;

    let mut dialog_path = model_path.join(model_name);
    dialog_path.set_extension("dialog");

    let dialog_src = fs::read_to_string(&dialog_path)
      .context(format!("Failed to read {}", dialog_path.display()))?;

    let dialog_tokens = core::dialog::dialog_block_lexer()
      .parse(&dialog_src)
      .into_result()
      .map_err(|err| anyhow::anyhow!("Dialog Block Lexer {:#?}", err))?;

    let dialog_mgr = core::dialog::DialogManager::new_from_tokens(dialog_tokens)
      .context("Failed to create DialogManager")?;

    Ok(Self {
      ctx,
      texture_id,
      model,
      enummap,
      mvp: glam::Mat4::from_scale(glam::vec3(2.0, 2.0, 1.0)),
      model_renderer,
      blackscreen_renderer,
      texture_target,
      motion_mgr,
      animator: core::live2d::animator::Animator::new(),
      layout: Layout::new(imgui_context, &dialog_mgr),
      dialog_mgr,
      dialog_player: None,
    })
  }

  pub fn update(&mut self, deltatime: f32) {
    if let Some(player) = &mut self.dialog_player {
      player.update(
        &mut self.animator,
        &self.dialog_mgr,
        &self.enummap,
        &self.motion_mgr,
      );
    }

    self.animator.update(deltatime, &mut self.model);
  }

  pub fn draw(&mut self, ui: &mut Ui) {
    self.texture_target.draw(|| {
      self.model_renderer.draw(&self.model, &self.mvp);
      self.blackscreen_renderer.draw(self.animator.blackscreen_alpha());
    });

    let ctx = EditorContext {
      model: &mut self.model,
      animator: &mut self.animator,
      enummap: &mut self.enummap,
      dialog_mgr: &mut self.dialog_mgr,
      dialog_player: &mut self.dialog_player,
      motion_mgr: &self.motion_mgr,
    };
    self.layout.draw(ui, ctx);

    ui.window("Preview").build(|| {
      // Put this section in comment if you wanna work with both of your hands
      let available = ui.content_region_avail();

      if available[0] <= 0.0 || available[1] <= 0.0 {
        return;
      }

      let mut draw_size = available;

      if available[0] / available[1] > 1.0 {
        draw_size[0] = available[1];
      } else {
        draw_size[1] = available[0];
      }

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
  }

  pub fn resize(&mut self, width: u32, height: u32) {}

  pub fn keyboard(&mut self, event: KeyEvent) {
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
    }
  }
}
