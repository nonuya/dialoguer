use core::renderer::load_texture_from_astc_path;
use std::rc::Rc;

use anyhow::Context;

use crate::project::{ProjectAssets, ProjectManifest};

pub struct EditorRuntime {
  pub model: core::live2d::Model,
  pub model_renderer: core::renderer::ModelRenderer,
  pub animator: core::live2d::animator::Animator,
  // dialog_player: Option<core::dialog::DialogPlayer>,
}

impl EditorRuntime {
  pub fn new(
    manifest: &ProjectManifest,
    assets: &ProjectAssets,
    gl: Rc<glow::Context>,
  ) -> anyhow::Result<Self> {
    let model = {
      let texture = load_texture_from_astc_path(&gl, &manifest.texture)?;

      let model2d = cubism::model::UserModel::from_model3(&manifest.root, &manifest.model3)
        .context("Failed to create UserModel from model3")?;

      core::live2d::Model::new(gl.clone(), model2d, texture)?
    };

    let model_renderer = {
      core::renderer::ModelRenderer::new(gl.clone(), assets.layout)
        .context("Failed to create Live2D Renderer")?
    };

    let animator = {
      let initial_view = assets
        .model_def
        .views
        .get("Default")
        .context("Failed to get 'Default' view")?
        .clone();

      core::live2d::animator::Animator::new(initial_view)
    };

    Ok(Self {
      model,
      model_renderer,
      animator,
      // dialog_player
    })
  }
}
