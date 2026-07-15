use crate::live2d::{Model, config, shaders::GlobalShaders};
use cubism::core::{ConstantFlags, DynamicFlags};
use glow::HasContext;
use std::rc::Rc;

pub struct Renderer {
  shaders: GlobalShaders,
  gl: Rc<glow::Context>,
  pub width: i32,
  pub height: i32,
}

impl Renderer {
  pub fn new(gl: Rc<glow::Context>) -> anyhow::Result<Self> {
    let shaders = GlobalShaders::new(&gl)?;

    Ok(Self { gl, shaders, width: 800, height: 800 })
  }

  pub fn draw(&self, model: &Model, mvp: &glam::Mat4) {
    self.draw_masks(model);
    self.draw_model(model, mvp);
  }

  fn draw_masks(&self, model: &Model) {
    let clipping_manager = model.get_clipping_manager();

    unsafe {
      self.gl.disable(glow::DEPTH_TEST);
      self.gl.disable(glow::CULL_FACE);

      self.gl.enable(glow::BLEND);

      // MASK_BLENDING
      self
        .gl
        .blend_equation_separate(glow::FUNC_ADD, glow::FUNC_ADD);

      self.gl.blend_func_separate(
        glow::ZERO,
        glow::ONE_MINUS_SRC_COLOR,
        glow::ZERO,
        glow::ONE_MINUS_SRC_ALPHA,
      );
    }

    // limpiar todos los framebuffers de máscara
    for offscreen in clipping_manager.get_offscreens() {
      unsafe {
        self
          .gl
          .bind_framebuffer(glow::FRAMEBUFFER, Some(offscreen.framebuffer));

        self
          .gl
          .viewport(0, 0, config::MASK_SIZE as i32, config::MASK_SIZE as i32);

        self.gl.clear_color(1.0, 1.0, 1.0, 1.0);
        self.gl.clear(glow::COLOR_BUFFER_BIT);
      }
    }

    for cc in clipping_manager.get_clipping_contexts_for_mask() {
      for &draw_index in cc.get_draw_indices() {
        let draw_index = draw_index as usize;

        let dflags = model.get_drawable_dynamic_flag(draw_index);

        if !dflags.intersects(DynamicFlags::VERTEX_POSITIONS_CHANGED) {
          continue;
        }

        unsafe {
          //-------------------------
          // framebuffer
          //-------------------------

          let fb = clipping_manager.get_offscreens()[cc.get_buffer_index() as usize].framebuffer;

          self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fb));

          self
            .gl
            .viewport(0, 0, config::MASK_SIZE as i32, config::MASK_SIZE as i32);

          //-------------------------
          // shader
          //-------------------------
          let shader = &self.shaders.setup;

          self.gl.use_program(Some(shader.program));

          //-------------------------
          // texture
          //-------------------------
          self.gl.active_texture(glow::TEXTURE0);

          self
            .gl
            .bind_texture(glow::TEXTURE_2D, Some(*model.get_texture()));

          self.gl.uniform_1_i32(shader.texture0.as_ref(), 0);

          //-------------------------
          // uniforms
          //-------------------------
          self.gl.uniform_matrix_4_f32_slice(
            shader.clip_matrix.as_ref(),
            false,
            cc.get_matrix_for_mask().as_ref(),
          );

          let bounds = cc.get_layout_bounds();

          self.gl.uniform_4_f32(
            shader.base_color.as_ref(),
            bounds.x * 2.0 - 1.0,
            bounds.y * 2.0 - 1.0,
            bounds.right() * 2.0 - 1.0,
            bounds.bottom() * 2.0 - 1.0,
          );

          let c = cc.get_color_channel();

          self
            .gl
            .uniform_4_f32(shader.channel_flag.as_ref(), c[0], c[1], c[2], c[3]);

          self
            .gl
            .uniform_4_f32(shader.multiply_color.as_ref(), 1.0, 1.0, 1.0, 1.0);

          self
            .gl
            .uniform_4_f32(shader.screen_color.as_ref(), 0.0, 0.0, 0.0, 0.0);
          

          let mesh = model.get_mesh_by_index(draw_index);
          mesh.draw(&self.gl);
        }
      }
    }

    unsafe {
      self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
    }
  }

  fn draw_model(&self, model: &Model, mvp: &glam::Mat4) {
    let clipping_manager = model.get_clipping_manager();

    unsafe {
      self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);

      self.gl.viewport(0, 0, self.width, self.height);

      self.gl.enable(glow::BLEND);
      self.gl.disable(glow::DEPTH_TEST);
      self.gl.disable(glow::CULL_FACE);

      self.gl.clear_color(0.0, 0.0, 0.0, 0.0);
      self.gl.clear(glow::COLOR_BUFFER_BIT);
    }

    for drawable in model.get_sorted_drawables() {
      let dflags = drawable.dynamic_flags;
      let cflags = drawable.constant_flags;

      if drawable.opacity <= 0.0 || !dflags.intersects(DynamicFlags::IS_VISIBLE) {
        continue;
      }

      //----------------------------------
      // Shader
      //----------------------------------
      let (shader, mask_texture, clip_matrix, channel_flag) =
        if let Some(cc) = clipping_manager.try_get_clipping_context_for_draw(drawable.index) {
          let shader = if cflags.intersects(ConstantFlags::IS_INVERTED_MASK) {
            &self.shaders.inverted_mask
          } else {
            &self.shaders.masked
          };

          (
            shader,
            Some(clipping_manager.get_offscreen_by_idx(cc.get_buffer_index())),
            Some(cc.get_matrix_for_draw()),
            cc.get_color_channel(),
          )
        } else {
          (&self.shaders.normal, None, None, [0.0; 4])
        };

      //----------------------------------
      // Blend
      //----------------------------------
      unsafe {
        Self::set_blend_mode(&self.gl, cflags);
      }

      //----------------------------------
      // Program
      //----------------------------------
      unsafe {
        self.gl.use_program(Some(shader.program));

        self
          .gl
          .uniform_matrix_4_f32_slice(shader.matrix.as_ref(), false, mvp.as_ref());

        self
          .gl
          .uniform_4_f32(shader.base_color.as_ref(), 1.0, 1.0, 1.0, drawable.opacity);

        self
          .gl
          .uniform_4_f32(shader.multiply_color.as_ref(), 1.0, 1.0, 1.0, 1.0);

        self
          .gl
          .uniform_4_f32(shader.screen_color.as_ref(), 0.0, 0.0, 0.0, 0.0);

        self.gl.uniform_4_f32(
          shader.channel_flag.as_ref(),
          channel_flag[0],
          channel_flag[1],
          channel_flag[2],
          channel_flag[3],
        );
      }

      //----------------------------------
      // Textura principal
      //----------------------------------
      unsafe {
        self.gl.active_texture(glow::TEXTURE0);

        self
          .gl
          .bind_texture(glow::TEXTURE_2D, Some(*model.get_texture()));

        self.gl.uniform_1_i32(shader.texture0.as_ref(), 0);
      }

      //----------------------------------
      // Textura de máscara
      //----------------------------------

      if let Some(mask) = mask_texture {
        unsafe {
          self.gl.active_texture(glow::TEXTURE1);

          self.gl.bind_texture(glow::TEXTURE_2D, Some(mask.texture));

          self.gl.uniform_1_i32(shader.texture1.as_ref(), 1);

          self.gl.uniform_matrix_4_f32_slice(
            shader.clip_matrix.as_ref(),
            false,
            clip_matrix.unwrap().as_ref(),
          );
        }
      }

      //----------------------------------
      // Draw
      //----------------------------------
      // model.get_mesh_by_index(drawable.index).draw(&self.gl);
      model.get_mesh_by_index(drawable.index).draw(&self.gl);
    }
  }

  unsafe fn set_blend_mode(gl: &glow::Context, cflags: ConstantFlags) {
    unsafe {
      gl.blend_equation_separate(glow::FUNC_ADD, glow::FUNC_ADD);

      if cflags.intersects(ConstantFlags::BLEND_ADDITIVE) {
        gl.blend_func_separate(glow::ONE, glow::ONE, glow::ZERO, glow::ONE);
      } else if cflags.intersects(ConstantFlags::BLEND_MULTIPLICATIVE) {
        gl.blend_func_separate(
          glow::DST_COLOR,
          glow::ONE_MINUS_SRC_ALPHA,
          glow::ZERO,
          glow::ONE,
        );
      } else {
        gl.blend_func_separate(
          glow::ONE,
          glow::ONE_MINUS_SRC_ALPHA,
          glow::ONE,
          glow::ONE_MINUS_SRC_ALPHA,
        );
      }
    }
  }
}

impl Drop for Renderer {
  fn drop(&mut self) {
    unsafe {
      self.gl.delete_program(self.shaders.setup.program);
      self.gl.delete_program(self.shaders.normal.program);
      self.gl.delete_program(self.shaders.masked.program);
      self.gl.delete_program(self.shaders.inverted_mask.program);
    }
  }
}
