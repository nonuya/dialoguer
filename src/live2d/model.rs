use std::{fs::File, path::PathBuf, rc::Rc};

use anyhow::Context;
use bytemuck::{Pod, Zeroable};
use cubism::{
  core::{Drawable, DynamicFlags},
  model::UserModel,
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
  motion: cubism::motion::Motion, // FIXME: Test
  clipping_manager: ClippingManager,
}

impl Model {
  pub fn new(gl: Rc<glow::Context>, model_path: PathBuf) -> anyhow::Result<Self> {
    // ==================================
    // CUBISM
    // ==================================
    let model_name = model_path
      .file_name()
      .ok_or_else(|| anyhow::anyhow!("Fail to get model name from '{}'", model_path.display()))?;

    let model_file = File::open(model_path.join(format!("{}.model3.json", model_name.display())))?;
    let model_json = cubism::json::model::Model3::from_reader(model_file)?;
    let cubism = UserModel::from_model3(&model_path, &model_json)?;
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
      model_json.file_references.textures.len(),
      1,
      "Until now, Saltpeter-Art textures have just one single texture."
    );
    let texture_path = &model_json.file_references.textures[0];
    let texture = load_texture_from_astc_path(&gl, &model_path.join(texture_path))?;
    // ==================================

    let clipping_manager = ClippingManager::new(gl.clone(), &cubism, config::MASK_BUFFER_COUNT)?;

    let motion_path = &model_json.file_references.motions.idle[7].file;
    let motion_path = model_path.join(motion_path);
    debug!("Reading motion at '{}'", motion_path.display());
    let mut motion = cubism::motion::Motion::from_motion3_json(motion_path)?;
    motion.play();
    motion.set_looped(true);

    Ok(Self {
      gl,
      cubism,
      texture,
      meshes,
      clipping_manager,
      motion,
    })
  }

  pub fn update(&mut self, deltatime: f32) {
    self.motion.tick(deltatime as f64);
    self.motion.update(self.cubism.model_mut()).unwrap();
    self.cubism.model_mut().update();
   
    self.clipping_manager.update_graph(&self.cubism);
    for drawable in self.cubism.drawables() {
      self.get_mesh_by_index(drawable.index).update(&self.gl, drawable);
    }
  }

  pub fn get_clipping_manager(&self) -> &ClippingManager {
    &self.clipping_manager
  }

  pub fn get_drawable_dynamic_flag(&self, drawable_index: usize) -> DynamicFlags {
    self.cubism.drawable_dynamic_flags()[drawable_index]
  }

  pub fn get_texture(&self) -> &glow::Texture {
    &self.texture
  }

  pub fn get_mesh_by_index(&self, drawable_index: usize) -> &Mesh {
    &self.meshes[drawable_index]
  }

  pub fn get_sorted_drawables(&self) -> Vec<Drawable<'_>> {
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
    let vertices: Vec<_> = drawable.vertex_positions
      .iter()
      .zip(drawable.vertex_uvs)
      .map(|(&pos, &uv)| Vertex {pos, uv})
      .collect();

    unsafe {
      gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));

      gl.buffer_sub_data_u8_slice(glow::ARRAY_BUFFER, 0, bytemuck::cast_slice(vertices.as_slice()));

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
