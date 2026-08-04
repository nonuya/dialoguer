use anyhow::Context;
use dear_imgui_glow::GlowRenderer;
use dear_imgui_rs::*;
use std::{fs::File, path::PathBuf, rc::Rc};
use winit::event::KeyEvent;

use crate::{layout::{EditorContext, Layout}, timeline::Timeline};

pub struct App {
  gl: Rc<glow::Context>,
  texture_id: TextureId,
  model: live2d::Model,
  enummap: live2d::animator::EnumMap,
  mvp: glam::Mat4,
  model_renderer: live2d::Renderer,
  animator: live2d::animator::Animator,
  layout: Layout,
}

impl App {
  pub fn new(model_path: PathBuf, renderer: &mut GlowRenderer) -> anyhow::Result<Self> {
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
    let mut model = live2d::Model::new(gl.clone(), &model_path, &model3)?;
    model.save_parameters();

    let mut enummap_path = model_path.join(model_name);
    enummap_path.set_extension("map");
    let enummap = live2d::animator::load_enum_map(&enummap_path)?;

    let model_renderer = live2d::Renderer::new(
      gl.clone(),
      live2d::config::MODEL_WIDTH,
      live2d::config::MODEL_HEIGHT,
    )
    .context("Failed to create Live2D Renderer")?;

    let texture_id = TextureId::new(1000);
    renderer
      .texture_map_mut()
      .set(texture_id, model_renderer.tex());

    Ok(Self {
      gl,
      texture_id,
      model,
      enummap,
      mvp: glam::Mat4::from_scale(glam::vec3(2.0, 2.0, 1.0)),
      model_renderer,
      animator: live2d::animator::Animator::new(),
      layout: Layout::new(),
    })
  }

  pub fn update(&mut self, deltatime: f32) {
    self.animator.update(deltatime, &mut self.model);
  }

  pub fn draw(&mut self, ui: &mut Ui) {
    self.model_renderer.draw(&self.model, &self.mvp);

    let ctx = EditorContext {
      model: &mut self.model,
      animator: &mut self.animator,
      enummap: &mut self.enummap,
    };
    self.layout.draw(ui, ctx);

    ui.window("Preview").build(|| {
      /*let available = ui.content_region_avail();

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
        .build();*/
    });
  }

  pub fn resize(&mut self, width: u32, height: u32) {}

  pub fn keyboard(&mut self, event: KeyEvent) {}
}
