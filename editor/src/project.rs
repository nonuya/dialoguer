use core::{
  dialog::{DialogManager, serializable::ModelDefinition},
  live2d::animator::MotionManager,
};
use std::{
  collections::HashMap,
  path::{Path, PathBuf},
  rc::Rc,
};

use anyhow::Context;
use cubism::json::model::{Layout, Model3};

pub struct ProjectManifest {
  pub model3: Model3, // .model3.json
  pub root: PathBuf,
  pub definition: PathBuf,   // .def
  pub dialog: PathBuf,       // .dialog
  pub motions: Vec<PathBuf>, // motions
  pub texture: PathBuf,
}

impl ProjectManifest {
  pub fn from_path(path: &Path) -> anyhow::Result<Self> {
    let model_name = path
      .file_name()
      .ok_or_else(|| anyhow::anyhow!("Fail to get model name from '{}'", path.display()))?;

    let model3 = {
      let model3_path = path.join(format!("{}.model3.json", model_name.display()));

      let model_file = std::fs::File::open(&model3_path).context(format!(
        "Failed to read .model3.json in '{}'",
        path.display()
      ))?;

      cubism::json::model::Model3::from_reader(model_file).context(format!(
        "Failed to parse '{}.model3.json'",
        model_name.display()
      ))?
    };

    let mut definition = path.join(model_name);
    definition.set_extension("def");

    let mut dialog = path.join(model_name);
    dialog.set_extension("dialog");

    let motions: Vec<_> = model3
      .file_references
      .motions
      .idle
      .iter()
      .map(|m| path.join(&m.file))
      .collect();

    anyhow::ensure!(
      model3.file_references.textures.len() == 1,
      "We work with just one single texture"
    );

    let texture = {
      let filename = &model3.file_references.textures[0];
      path.join(filename)
    };

    Ok(Self {
      texture,
      model3,
      definition,
      dialog,
      motions,
      root: path.to_path_buf(),
    })
  }
}

pub struct ProjectAssets {
  pub model_def: ModelDefinition,
  pub dialog_mgr: DialogManager,
  pub motion_mgr: MotionManager,
  // sound_mgr: SoundManager,
  pub layout: Layout,
}

impl ProjectAssets {
  pub fn load(manifest: &ProjectManifest) -> anyhow::Result<Self> {
    let model_def = {
      let src = std::fs::read_to_string(&manifest.definition).context(format!(
        "Failed to load EnumMap in {}",
        manifest.definition.display()
      ))?;

      core::dialog::serializable::ModelDefinition::from_str(&src)?
    };

    let dialog_mgr = {
      let dialog_src = std::fs::read_to_string(&manifest.dialog)
        .context(format!("Failed to read {}", manifest.dialog.display()))?;

      let dialog_tokens = core::dialog::parse_dialog_into_tokens(&dialog_src)?;

      core::dialog::DialogManager::from_tokens(dialog_tokens)
        .context("Failed to create DialogManager")?
    };

    let layout = manifest
      .model3
      .layout
      .context("model3.json doesnt have Layout")?;

    // Parallel loading
    let motion_mgr = {
      use rayon::prelude::*;

      let motions = manifest
        .motions
        .par_iter()
        .map(|p| {
          log::info!("[Live2D] Loading Motion '{}'", p.display());

          let name = p
            .file_prefix()
            .and_then(|p| p.to_str())
            .context("Failed to get Motion name")?;

          let motion = cubism::motion::Motion::from_motion3_json(&p)?;

          Ok((name, motion))
        })
        .collect::<anyhow::Result<HashMap<_, _>>>()?;

      let motions = motions
        .into_iter()
        .map(|(name, motion)| (name.into(), motion))
        .collect();

      core::live2d::animator::MotionManager(motions)
    };

    Ok(Self {
      dialog_mgr,
      layout,
      model_def,
      motion_mgr,
    })
  }
}
