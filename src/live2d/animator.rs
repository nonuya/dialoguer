use std::collections::HashMap;

use crate::live2d::Model;
use cubism::motion::Motion;
use log::warn;

pub struct Animator {
  motion: Option<Motion>,
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

impl Animator {
  pub fn new() -> Self {
    Self {
      motion: None,
      map: HashMap::new(),
    }
  }

  pub fn play_motion(&mut self, mut motion: Motion, looped: bool) {
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
    if let Some(motion) = &mut self.motion {
      motion.tick(deltatime as f64);
      model.apply_motion(motion).unwrap();
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
