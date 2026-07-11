use std::{fs::File, path::PathBuf};

use anyhow::anyhow;
use cubism::{
  core::{Drawable, DynamicFlags},
  model::UserModel,
};
use glium::{
  Display, IndexBuffer, Texture2d, VertexBuffer, glutin::surface::WindowSurface,
  texture::RawImage2d,
};

use crate::live2d::{clipping_manager::ClippingManager, config, vertex::Vertex};

pub struct Model {
  cubism: UserModel,
  texture: Texture2d,
  vertex_buffers: Vec<VertexBuffer<Vertex>>,
  indice_buffers: Vec<IndexBuffer<u16>>,
  clipping_manager: ClippingManager,
}

impl Model {
  pub fn new(display: &Display<WindowSurface>, model_path: PathBuf) -> anyhow::Result<Self> {
    // ==================================
    // CUBISM
    // ==================================
    let model_name = model_path
      .file_name()
      .ok_or_else(|| anyhow!("Fail to get model name from '{}'", model_path.display()))?;

    let model_file = File::open(model_path.join(format!("{}.model3.json", model_name.display())))?;
    let model_json = cubism::json::model::Model3::from_reader(model_file)?;
    let cubism = UserModel::from_model3(&model_path, &model_json)?;
    // ==================================

    // ==================================
    // OpenGL buffers
    // ==================================
    let (vertex_buffers, indice_buffers) = cubism.drawables().map(
      |cubism::core::Drawable {
         vertex_positions,
         vertex_uvs,
         indices,
         ..
       }| {
        let buffer: Vec<_> = vertex_positions
          .iter()
          .zip(vertex_uvs)
          .map(|(&pos, &uv)| Vertex {
            a_position: pos,
            a_texCoord: uv,
          })
          .collect();

        let vb = glium::VertexBuffer::dynamic(display, &buffer)?;
        let vi = glium::IndexBuffer::new(
          display,
          glium::index::PrimitiveType::TrianglesList,
          &indices,
        )?;

        Ok((vb, vi))
      },
    ).collect::<anyhow::Result<_>>()?;
    // ==================================
    
    // ==================================
    // Texture2D
    // ==================================
    assert_eq!(model_json.file_references.textures.len(), 1, "Until now, Saltpeter-Art textures have just one single texture.");
    let texture_path = &model_json.file_references.textures[0];
    let image = image::open(model_path.join(texture_path))?.to_rgba8();
    let dimensions = image.dimensions();
    let raw = RawImage2d::from_raw_rgba(image.into_raw(), dimensions);
    let texture = Texture2d::new(display, raw)?;
    // ==================================

    let clipping_manager = ClippingManager::new(display, &cubism, config::MASK_BUFFER_COUNT)?;

    Ok(Self {
      cubism,
      vertex_buffers,
      indice_buffers,
      texture,
      clipping_manager,
    })
  }

  pub fn get_clipping_manager(&self) -> &ClippingManager {
    &self.clipping_manager
  }

  pub fn get_drawable_dynamic_flag(&self, drawable_index: usize) -> DynamicFlags {
    self.cubism.drawable_dynamic_flags()[drawable_index]
  }

  pub fn get_texture(&self) -> &Texture2d {
    &self.texture
  }

  pub fn get_drawable_vertices(&self, drawable_index: usize) -> &VertexBuffer<Vertex> {
    &self.vertex_buffers[drawable_index]
  }

  pub fn get_drawable_indices(&self, drawable_index: usize) -> &IndexBuffer<u16> {
    &self.indice_buffers[drawable_index]
  }

  pub fn get_sorted_drawables(&self) -> Vec<Drawable<'_>> {
    let mut drawables: Vec<_> = self.cubism.drawables().collect();
    drawables.sort_unstable_by_key(|d| d.render_order);
    drawables
  }
}
