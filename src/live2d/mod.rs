mod clipping_manager;
mod rectf;
mod uniforms;
mod vertex;
mod shaders;
mod renderer;
mod model;
pub mod config;

pub use shaders::GlobalShaders;
pub use shaders::load_global_shaders;
pub use model::Model;
pub use renderer::{draw_masks, draw_model};
