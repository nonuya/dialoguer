use std::{
  collections::{HashMap, VecDeque},
  fs,
  path::{Path, PathBuf},
  rc::Rc,
};

use crate::live2d::Model;
use anyhow::Context;
use cubism::motion::Motion;
use log::{debug, warn};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct EnumMap {
  pub enums: HashMap<Rc<str>, EnumType>,
  pub views: HashMap<Rc<str>, ViewType>,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct ViewType {
  pub position: (f32, f32, f32),
  pub rotation: f32,
  pub scale: f32,
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

enum FadeStatus {
  FadeIn(Motion, bool),
  FadeOut,
  None,
}

const FADE_DURATION: f32 = 0.10;
const FADE_STEP: f32 = 1.0 / FADE_DURATION;

pub struct ParameterChange {
  pub enumtype: Rc<str>,
  pub enumname: Rc<str>,
  pub params: Vec<(Rc<str>, Value)>,
}

pub struct Animator {
  motion: Option<Motion>,
  fade_status: FadeStatus,
  timer: Option<f32>,
  blackscreen_alpha: f32,
  parameters_map: HashMap<Rc<str>, Value>,
  activeenum_map: HashMap<Rc<str>, Rc<str>>,
  view: ViewType,
  target_view: ViewType,
  pending_set_parameter: VecDeque<ParameterChange>,
  pending_remove_enum: VecDeque<Rc<str>>,
}

impl Animator {
  pub fn new(initial_view: ViewType) -> Self {
    Self {
      motion: None,
      timer: None,
      view: initial_view,
      target_view: initial_view,
      fade_status: FadeStatus::None,
      pending_set_parameter: VecDeque::new(),
      pending_remove_enum: VecDeque::new(),
      blackscreen_alpha: 0.0,
      parameters_map: HashMap::new(),
      activeenum_map: HashMap::new(),
    }
  }

  pub fn blackscreen_alpha(&self) -> f32 {
    self.blackscreen_alpha
  }

  pub fn play_motion(&mut self, motion: Motion, looped: bool) {
    self.fade_status = FadeStatus::FadeIn(motion, looped);
    self.blackscreen_alpha = 0.0;
  }

  pub fn set_motion(&mut self, mut motion: Motion, looped: bool) {
    motion.play();
    motion.set_looped(looped);
    self.motion = Some(motion);
  }

  pub fn clear_parameters(&mut self) {
    self.parameters_map.clear();
    self.activeenum_map.clear();
  }

  pub fn deferred_set_parameter(&mut self, change: ParameterChange) {
    self.pending_set_parameter.push_back(change);
  }

  pub fn set_parameter_change(&mut self, change: ParameterChange) {
    for (id, value) in change.params.into_iter() {
      self.set_parameter(id, value);
    }

    self.activeenum_map.insert(change.enumtype, change.enumname);
  }

  fn set_parameter(&mut self, id: Rc<str>, mut value: Value) {
    // If we change an existing value, we need to start from that value and go to new value
    if let Some(old) = self.parameters_map.get(&id) {
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

    self.parameters_map.insert(id, value);
  }

  pub fn is_enum_active(&self, enumtype: &Rc<str>, enumvalue: &Rc<str>) -> bool {
    self
      .activeenum_map
      .get(enumtype)
      .is_some_and(|v| v.as_ref() == enumvalue.as_ref())
  }

  pub fn remove_enum(&mut self, name: &Rc<str>, enummap: &EnumMap) {
    if let Some(enumvalue) = self.activeenum_map.remove(name)
      && let Some(params) = enummap
        .enums
        .get(name)
        .unwrap()
        .values
        .get(&enumvalue)
    {
      for p in params.iter() {
        self.parameters_map.remove(&p.name);
      }
    }
  }

  pub fn deferred_remove_enum(&mut self, name: Rc<str>) {
    self.pending_remove_enum.push_back(name);
  }

  pub fn set_timer(&mut self, seconds: f32) {
    self.timer = Some(seconds);
  }

  pub fn is_timer_playing(&self) -> bool {
    self.timer.is_some() || matches!(self.fade_status, FadeStatus::FadeIn(..))
  }

  pub fn set_target_view(&mut self, target_view: ViewType) {
    self.target_view = target_view;
  }

  pub fn set_view(&mut self, view: ViewType) {
    self.view = view;
  }

  pub fn get_matrix(&self) -> glam::Mat4 {
    glam::Mat4::from_scale_rotation_translation(
      glam::Vec3::splat(self.view.scale),
      glam::Quat::from_rotation_z(self.view.rotation.to_radians()),
      glam::vec3(
        self.view.position.0,
        self.view.position.1,
        self.view.position.2,
      ),
    )
  }

  pub fn stop_timer(&mut self) {
    self.timer = None;
  }

  pub fn clear_motion(&mut self) {
    self.motion = None;
  }

  // FIXME: Hay un Delay en Wait, no sé si termina el Wait y se espera unos microsegundos para empezar
  pub fn update(&mut self, deltatime: f32, model: &mut Model, enummap: &EnumMap) {
    // Fade
    match std::mem::replace(&mut self.fade_status, FadeStatus::None) {
      FadeStatus::FadeIn(motion, looped) => {
        self.blackscreen_alpha += FADE_STEP * deltatime;

        if self.blackscreen_alpha >= 1.0 {
          self.blackscreen_alpha = 1.0 + FADE_DURATION * 10.0; // Little delay for update motion
          self.fade_status = FadeStatus::FadeOut;
          while let Some(change) = self.pending_set_parameter.pop_front() {
            self.set_parameter_change(change);
          }
          while let Some(name) = self.pending_remove_enum.pop_front() {
            self.remove_enum(&name, enummap);
          }
          self.set_motion(motion, looped);
        } else {
          self.fade_status = FadeStatus::FadeIn(motion, looped);
        }
      }
      FadeStatus::FadeOut => {
        self.blackscreen_alpha -= FADE_STEP * deltatime;
        if self.blackscreen_alpha <= 0.0 {
          self.blackscreen_alpha = 0.0;
          self.fade_status = FadeStatus::None;
        } else {
          self.fade_status = FadeStatus::FadeOut;
        }
      }
      FadeStatus::None => {}
    }

    // Motion
    if let Some(motion) = self.motion.as_mut() {
      motion.tick(deltatime as f64);
      model.apply_motion(motion).unwrap();
    }

    for (id, value) in &mut self.parameters_map {
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
            let step = *step * deltatime * 30.0;

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

    // Timer
    if !matches!(self.fade_status, FadeStatus::FadeIn(..))
      && let Some(remaining) = self.timer.as_mut()
    {
      *remaining -= deltatime;

      if *remaining <= 0.0 {
        self.timer = None;
      }
    }

    // ViewType
    const VIEW_SPEED: f32 = 0.20;
    const EPSILON: f32 = 0.01;
    let t = (VIEW_SPEED * deltatime).min(1.0);

    self.view.position.0 += (self.target_view.position.0 - self.view.position.0) * t;
    self.view.position.1 += (self.target_view.position.1 - self.view.position.1) * t;
    self.view.position.2 += (self.target_view.position.2 - self.view.position.2) * t;

    self.view.rotation += (self.target_view.rotation - self.view.rotation) * t;

    self.view.scale += (self.target_view.scale - self.view.scale) * t;

    if (self.view.position.0 - self.target_view.position.0).abs() < EPSILON
      && (self.view.position.1 - self.target_view.position.1).abs() < EPSILON
      && (self.view.position.2 - self.target_view.position.2).abs() < EPSILON
      && (self.view.rotation - self.target_view.rotation).abs() < EPSILON
      && (self.view.scale - self.target_view.scale).abs() < EPSILON
    {
      self.view = self.target_view.clone();
    }
  }
}

pub struct MotionManager(HashMap<Rc<str>, Motion>);

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

        Ok((name.into(), motion))
      })
      .collect::<anyhow::Result<HashMap<_, _>>>()?;
    // let motions = HashMap::new();

    Ok(Self(motions))
  }

  pub fn names(&self) -> Vec<&Rc<str>> {
    self.0.keys().collect()
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
