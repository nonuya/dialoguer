use std::{collections::HashMap, path::PathBuf, rc::Rc};

use anyhow::Context;
use bytemuck::{Pod, Zeroable};
use cubism::{
  core::{Drawable, DynamicFlags, ParameterIterMut}, error::CubismResult, model::UserModel, motion::Motion
};
use glow::HasContext;
use log::debug;

use crate::live2d::{
  clipping_manager::ClippingManager, config, texture::load_texture_from_astc_path,
};

pub struct Model {
  gl: Rc<glow::Context>,
  cubism: UserModel,
  texture: glow::Texture,
  meshes: Vec<Mesh>,
  clipping_manager: ClippingManager,
  parameters: HashMap<Rc<str>, usize>,
}

impl Model {
  pub fn new(gl: Rc<glow::Context>, path: &PathBuf, model3: &cubism::json::model::Model3) -> anyhow::Result<Self> {
    // ==================================
    // CUBISM
    // ==================================
    let cubism = UserModel::from_model3(path, model3)?;
    
    let parameters = cubism
      .parameters()
      .enumerate()
      .map(|(i, p)| (p.id.into(), i))
      .collect();
    // ==================================

    // ==================================
    // OpenGL buffers
    // ==================================
    let meshes: Vec<_> = cubism
      .drawables()
      .map(
        |cubism::core::Drawable {
           vertex_positions,
           vertex_uvs,
           indices,
           ..
         }| {
          let vertices: Vec<_> = vertex_positions
            .iter()
            .zip(vertex_uvs)
            .map(|(&pos, &uv)| Vertex { pos, uv })
            .collect();

          Mesh::new(&gl, vertices, indices)
        },
      )
      .collect::<anyhow::Result<_>>()?;

    debug!("Nro of Mesh: {}", meshes.len());
    // ==================================

    // ==================================
    // Texture2D
    // ==================================
    assert_eq!(
      model3.file_references.textures.len(),
      1,
      "Until now, Saltpeter-Art textures have just one single texture."
    );
    let texture_path = &model3.file_references.textures[0];
    let texture = load_texture_from_astc_path(&gl, &path.join(texture_path))?;
    // ==================================

    let clipping_manager = ClippingManager::new(gl.clone(), &cubism, config::MASK_BUFFER_COUNT)?;

    Ok(Self {
      gl,
      cubism,
      texture,
      meshes,
      clipping_manager,
      parameters,
    })
  }

  pub fn apply_motion(&mut self, motion: &Motion) -> CubismResult<()> {
    motion.update(self.cubism.model_mut()) 
  }

  pub fn set_parameter_value(&mut self, id: &Rc<str>, val: f32) -> bool {
    self.parameters.get(id)
      .is_some_and(|&idx| {
        self.cubism.model_mut().set_parameter_value(idx, val);
        true
      })
  }

  pub fn get_parameter_value(&self, id: &Rc<str>) -> Option<f32> {
    self.parameters.get(id).and_then(|&idx| Some(self.cubism.parameter_at(idx).value))
  }

  pub fn get_parameters_iter(&mut self) -> ParameterIterMut {
    self.cubism.model_mut().parameters_mut()
  }

  pub fn save_parameters(&mut self) {
    self.cubism.save_parameters();
  }

  pub fn load_saved_parameters(&mut self) {
    self.cubism.load_parameters();
    self.update_parameters();
  }

  pub fn update_parameters(&mut self) {
    self.cubism.model_mut().update();

    self.clipping_manager.update_graph(&self.cubism);
    for drawable in self.cubism.drawables() {
      self
        .get_mesh_by_index(drawable.index)
        .update(&self.gl, drawable);
    }
  }

  pub(crate) fn get_clipping_manager(&self) -> &ClippingManager {
    &self.clipping_manager
  }

  pub(crate) fn get_drawable_dynamic_flag(&self, drawable_index: usize) -> DynamicFlags {
    self.cubism.drawable_dynamic_flags()[drawable_index]
  }

  pub(crate) fn get_texture(&self) -> &glow::Texture {
    &self.texture
  }

  pub(crate) fn get_mesh_by_index(&self, drawable_index: usize) -> &Mesh {
    &self.meshes[drawable_index]
  }

  pub(crate) fn get_sorted_drawables(&self) -> Vec<Drawable<'_>> {
    let mut drawables: Vec<_> = self.cubism.drawables().collect();
    drawables.sort_unstable_by_key(|d| d.render_order);
    drawables
  }
}

impl Drop for Model {
  fn drop(&mut self) {
    for mesh in &self.meshes {
      unsafe {
        self.gl.delete_vertex_array(mesh.vao);
        self.gl.delete_buffer(mesh.vbo);
        self.gl.delete_buffer(mesh.ebo);
      }
    }

    unsafe {
      self.gl.delete_texture(self.texture);
    }
  }
}

#[repr(C)]
#[allow(non_snake_case)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Vertex {
  pos: [f32; 2],
  uv: [f32; 2],
}

pub struct Mesh {
  vao: glow::NativeVertexArray,
  vbo: glow::NativeBuffer,
  ebo: glow::NativeBuffer,
  index_count: i32,
}

impl Mesh {
  fn new(gl: &glow::Context, vertices: Vec<Vertex>, indices: &[u16]) -> anyhow::Result<Self> {
    unsafe {
      let vao = gl
        .create_vertex_array()
        .map_err(anyhow::Error::msg)
        .context("Failed to create VAO")?;

      let vbo = gl
        .create_buffer()
        .map_err(anyhow::Error::msg)
        .context("Failed to create VBO")?;

      let ebo = gl
        .create_buffer()
        .map_err(anyhow::Error::msg)
        .context("Failed to create EBO")?;

      gl.bind_vertex_array(Some(vao));

      //-------------------------
      // Dynamic vertex buffer
      //-------------------------
      gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));

      gl.buffer_data_size(
        glow::ARRAY_BUFFER,
        (vertices.len() * std::mem::size_of::<Vertex>()) as i32,
        glow::DYNAMIC_DRAW,
      );

      gl.buffer_sub_data_u8_slice(glow::ARRAY_BUFFER, 0, bytemuck::cast_slice(&vertices));

      //-------------------------
      // Static index buffer
      //-------------------------
      gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));

      gl.buffer_data_u8_slice(
        glow::ELEMENT_ARRAY_BUFFER,
        bytemuck::cast_slice(indices),
        glow::STATIC_DRAW,
      );

      let stride = std::mem::size_of::<Vertex>() as i32;

      // location = 0
      gl.enable_vertex_attrib_array(0);
      gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, stride, 0);

      // location = 1
      gl.enable_vertex_attrib_array(1);
      gl.vertex_attrib_pointer_f32(
        1,
        2,
        glow::FLOAT,
        false,
        stride,
        (2 * std::mem::size_of::<f32>()) as i32,
      );

      gl.bind_vertex_array(None);

      Ok(Mesh {
        vao,
        vbo,
        ebo,
        index_count: indices.len() as i32,
      })
    }
  }

  pub fn update(&self, gl: &glow::Context, drawable: Drawable) {
    let vertices: Vec<_> = drawable
      .vertex_positions
      .iter()
      .zip(drawable.vertex_uvs)
      .map(|(&pos, &uv)| Vertex { pos, uv })
      .collect();

    unsafe {
      gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));

      gl.buffer_sub_data_u8_slice(
        glow::ARRAY_BUFFER,
        0,
        bytemuck::cast_slice(vertices.as_slice()),
      );

      gl.bind_buffer(glow::ARRAY_BUFFER, None);
    }
  }

  pub fn draw(&self, gl: &glow::Context) {
    unsafe {
      gl.bind_vertex_array(Some(self.vao));

      gl.draw_elements(glow::TRIANGLES, self.index_count, glow::UNSIGNED_SHORT, 0);

      gl.bind_vertex_array(None);
    }
  }
}
