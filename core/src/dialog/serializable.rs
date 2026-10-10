use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, rc::Rc};

#[derive(Serialize, Deserialize)]
pub struct ModelDefinition {
  pub enums: HashMap<Rc<str>, EnumType>,
  pub views: HashMap<Rc<str>, ViewType>,
}

impl ModelDefinition {
  pub fn from_str(src: &str) -> anyhow::Result<Self> {
    let def = ron::from_str(&src).context("Failed to parse DialogMap")?;
    Ok(def)
  }
}

#[derive(Serialize, Deserialize, Default)]
pub struct EnumType {
  pub values: HashMap<Rc<str>, Vec<ParamValue>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ParamValue {
  pub name: Rc<str>,
  pub value: Value,

  #[serde(default)]
  pub modification: Option<Modification>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Modification {
  pub lhs: Rc<str>,
  pub rhs: Rc<str>,
  pub then: f32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct ViewType {
  pub position: (f32, f32, f32),
  pub rotation: f32,
  pub scale: f32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
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
