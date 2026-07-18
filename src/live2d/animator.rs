use std::{collections::HashMap, fs, path::{Path, PathBuf}};

use crate::live2d::Model;
use anyhow::Context;
use cubism::motion::Motion;
use log::{debug, warn};
use serde::{Deserialize, Serialize};

pub struct Animator {
  motion: Option<Motion>,
  timer: Option<f32>,
  map: HashMap<String, Value>,
}


#[derive(Serialize, Deserialize)]
pub struct EnumMap {
    pub enums: HashMap<String, EnumType>,
}

#[derive(Serialize, Deserialize)]
pub struct EnumType {
    pub values: HashMap<String, Vec<ParamValue>>,
}
/// Ver "assets/example.map" para un ejemplo
#[derive(Serialize, Deserialize)]
pub struct ParamValue {
    pub name: String,
    pub value: Value,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum Value {
  Fixed(f32),
  Smooth {
    #[serde(default = "default_actual")]
    actual: f32,

    target: f32,
    step: f32 },
}

fn default_actual() -> f32 {
  0.0
}

impl Value {
  pub fn smooth(target: f32, step: f32) -> Self {
    Self::Smooth { actual: 0.0, target, step }
  }
}

impl Animator {
  pub fn new() -> Self {
    Self {
      motion: None,
      timer: None,
      map: HashMap::new(),
    }
  }

  pub fn play_motion(&mut self, mut motion: Motion, looped: bool) {
    motion.play();
    motion.set_looped(looped);
    self.motion = Some(motion);
  }

  pub fn set_parameter(&mut self, id: &String, mut value: Value) {
    // If we change an existing value, we need to start from that value and go to new value
    if let Some(old) = self.map.get(id) {
      match (old, &mut value) {
        (
          Value::Smooth { actual: old_actual, .. },
          Value::Smooth { actual: new_actual, .. }
        ) => *new_actual = *old_actual,
        (
          Value::Fixed(old_actual),
          Value::Smooth { actual: new_actual, ..}
        ) => *new_actual = *old_actual,
        _ => {}
      }
    }

    self.map.insert(id.to_string(), value);
  }

  pub fn remove_parameter(&mut self, id: &String) {
    self.map.remove(id);
  }

  pub fn set_timer(&mut self, seconds: f32) {
    self.timer = Some(seconds);
  }

  pub fn is_timer_playing(&self) -> bool {
    self.timer.is_some()
  }

  pub fn update(&mut self, deltatime: f32, model: &mut Model) {
    if let Some(motion) = self.motion.as_mut() {
      motion.tick(deltatime as f64);
      model.apply_motion(motion).unwrap();
    }
    if let Some(remaining) = self.timer.as_mut() {
      *remaining -= deltatime;

      if *remaining <= 0.0 {
        self.timer = None;
      }
    }

    for (id, value) in &mut self.map {
      match value {
        Value::Fixed(val) => {
          if !model.set_parameter_value(id, *val) {
            warn!("Failed to set '{}' to '{}'", id, val);
          }
        }
        Value::Smooth {
          actual,
          target,
          step,
        } => {
          if actual != target {
            let delta = *target - *actual;
            let step = *step;

            if delta.abs() <= step {
              *actual = *target;
            } else {
              *actual += delta.signum() * step;
            }
          }

          if !model.set_parameter_value(id, *actual) {
            warn!("Failed to set '{}' to '{}'", id, target);
          }
        }
      }
    }

    // Otros ajustes de parametros
    model.update_parameters();
  }
}

pub struct MotionManager(HashMap<String, Motion>);

impl MotionManager {
  pub fn new(path: &PathBuf, model3: &cubism::json::model::Model3) -> anyhow::Result<Self> {
    let motions = model3
      .file_references
      .motions
      .idle
      .iter()
      .map(|m| path.join(&m.file))
      .map(|path| {
        debug!("[Live2D] Loading Motion '{}'", path.display());
        let name = path
          .file_prefix()
          .and_then(|p| p.to_str())
          .context("Failed to get Motion name")?;
        let motion = cubism::motion::Motion::from_motion3_json(&path)?;

        Ok((name.to_string(), motion))
      })
    .collect::<anyhow::Result<HashMap<_,_>>>()?;
    // let motions = HashMap::new();

    Ok(Self(motions))
  }

  pub fn get(&self, name: &str) -> Option<&Motion> {
    self.0.get(name)
  }
}

pub fn load_enum_map(filepath: &Path) -> anyhow::Result<EnumMap> {
  let src =
    fs::read_to_string(filepath)
      .context(format!("Failed to load {}", filepath.display()))?;

  let myenums: EnumMap =
    ron::from_str(&src)
    .context("Failed to parse DialogMap")?;

  Ok(myenums)
}
