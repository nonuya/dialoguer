use std::{collections::HashMap, path::PathBuf};

use crate::live2d::Model;
use anyhow::Context;
use cubism::motion::Motion;
use log::{debug, warn};

pub struct Animator<'a> {
  motion: Option<&'a Motion>,
  map: HashMap<String, Value>,
}

#[derive(Clone, Copy)]
pub enum Value {
  Fixed(f32),
  Smooth { actual: f32, target: f32, step: f32 }, // TODO: Maybe actual: Option<f32>?
}

impl Value {
  pub fn smooth(target: f32, step: f32) -> Self {
    Self::Smooth { actual: 0.0, target, step }
  }
}

impl<'a> Animator<'a> {
  pub fn new() -> Self {
    Self {
      motion: None,
      map: HashMap::new(),
    }
  }

  pub fn play_motion(&mut self, motion: &'a mut Motion, looped: bool) {
    motion.play();
    motion.set_looped(looped);
    self.motion = Some(motion);
  }

  pub fn set_parameter(&mut self, id: &str, mut value: Value) {
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

  pub fn update(&mut self, deltatime: f32, model: &mut Model) {
    /*
    if let Some(motion) = self.motion.as_mut() {
      motion.tick(deltatime as f64);
      model.apply_motion(motion).unwrap();
    }*/

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

    Ok(Self(motions))
  }

  pub fn get(&self, name: &str) -> Option<&Motion> {
    self.0.get(name)
  }
}

/*
    let enum_map = HashMap::from([
      (
        "BlushType",
        EnumType(HashMap::from([
          (
            "None",
            vec![ParamValue("Param83", Value::smooth(0.0, 1.0))],
          ),
          (
            "Half",
            vec![ParamValue("Param83", Value::smooth(50.0, 1.0))],
          ),
          (
            "On",
            vec![ParamValue("Param83", Value::smooth(100.0, 1.0))],
          ),
        ])),
      )
    ]);
 */
pub type EnumValue = (String, Value);
pub type EnumType = /*Values*/ HashMap<&'static str, Vec<EnumValue>>;
pub type EnumMap = HashMap<String, EnumType>;
