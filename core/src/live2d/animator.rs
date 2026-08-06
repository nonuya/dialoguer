use std::{
  collections::HashMap,
  fs,
  path::{Path, PathBuf}, rc::Rc,
};

use crate::live2d::Model;
use anyhow::Context;
use cubism::motion::Motion;
use log::{debug, warn};
use serde::{Deserialize, Serialize};

pub struct Animator {
  motion: Option<Motion>,
  timer: Option<f32>,
  map: HashMap<Rc<str>, Value>,
}

#[derive(Serialize, Deserialize)]
pub struct EnumMap {
  pub enums: HashMap<Rc<str>, EnumType>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct EnumType {
  pub values: HashMap<Rc<str>, Vec<ParamValue>>,
}
/// Ver "assets/example.map" para un ejemplo
#[derive(Serialize, Deserialize)]
pub struct ParamValue {
  pub name: Rc<str>,
  pub value: Value,

  #[serde(default)]
  pub modification: Option<Modification>,
}

#[derive(Serialize, Deserialize)]
pub struct Modification {
  pub lhs: Rc<str>,
  pub rhs: Rc<str>,
  pub then: f32,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum Value {
  Fixed(f32),
  Smooth {
    #[serde(default = "default_actual")]
    actual: f32,

    target: f32,
    step: f32,
  },
}

fn default_actual() -> f32 {
  0.0
}

impl Value {
  pub fn smooth(target: f32, step: f32) -> Self {
    Self::Smooth {
      actual: 0.0,
      target,
      step,
    }
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

  pub fn is_parameter_equal_to_value(&self, id: &str, b: &Value) -> bool {
    self.map.get(id).is_some_and(|a| {
      let lhs = match a {
        Value::Fixed(f) => f,
        Value::Smooth { target, .. } => target,
      };
      let rhs = match b {
        Value::Fixed(f) => f,
        Value::Smooth { target, .. } => target,
      };

      lhs == rhs
    })
  }

  pub fn clear_parameters(&mut self) {
    self.map.clear();
  }

  pub fn set_parameter(&mut self, id: Rc<str>, mut value: Value) {
    // If we change an existing value, we need to start from that value and go to new value
    if let Some(old) = self.map.get(&id) {
      match (old, &mut value) {
        (
          Value::Smooth {
            actual: old_actual, ..
          },
          Value::Smooth {
            actual: new_actual, ..
          },
        ) => *new_actual = *old_actual,
        (
          Value::Fixed(old_actual),
          Value::Smooth {
            actual: new_actual, ..
          },
        ) => *new_actual = *old_actual,
        _ => {}
      }
    }

    match value {
      Value::Smooth { step, .. } => {
        if step == 0.0 {
          warn!("Step of Parameter '{}' is zero!", id)
        }
      }
      _ => {}
    }

    self.map.insert(id, value);
  }

  pub fn remove_parameter(&mut self, id: &Rc<str>) {
    self.map.remove(id);
  }

  pub fn set_timer(&mut self, seconds: f32) {
    self.timer = Some(seconds);
  }

  pub fn is_timer_playing(&self) -> bool {
    self.timer.is_some()
  }

  pub fn stop_timer(&mut self) {
    self.timer = None;
  }

  pub fn clear_motion(&mut self) {
    self.motion = None;
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

  pub fn motion_names(&self) -> Vec<&Rc<str>> {
    let mut names: Vec<_> = self.map.keys().collect(); 
    names.sort();
    names
  }
}

pub struct MotionManager(HashMap<Rc<str>, Motion>);

impl MotionManager {
  pub fn new(path: &PathBuf, model3: &cubism::json::model::Model3) -> anyhow::Result<Self> {
    /*
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
    .collect::<anyhow::Result<HashMap<_,_>>>()?;*/
    let motions = HashMap::new();

    Ok(Self(motions))
  }

  pub fn get(&self, name: &str) -> Option<&Motion> {
    self.0.get(name)
  }
}

pub fn load_enum_map(filepath: &Path) -> anyhow::Result<EnumMap> {
  let src =
    fs::read_to_string(filepath).context(format!("Failed to load {}", filepath.display()))?;

  let myenums: EnumMap = ron::from_str(&src).context("Failed to parse DialogMap")?;

  Ok(myenums)
}
