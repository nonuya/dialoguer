use crate::live2d::{Model, config};
use crate::renderer::RenderContext;
use crate::renderer::shader::create_program_from_source;
use cubism::core::{ConstantFlags, DynamicFlags};
use cubism::json::model::Layout;
use glow::HasContext;
use std::rc::Rc;

const VERTEX_SHADER_SRC_SETUP_MASK: &str = include_str!("./shaders/VertShaderSrcSetupMask.vert");
const FRAGMENT_SHADER_SRC_SETUP_MASK: &str = include_str!("./shaders/FragShaderSrcSetupMask.frag");
const VERTEX_SHADER_SRC: &str = include_str!("./shaders/VertShaderSrc.vert");
const FRAGMENT_SHADER_SRC: &str = include_str!("./shaders/FragShaderSrc.frag");
const VERTEX_SHADER_SRC_MASKED: &str = include_str!("./shaders/VertShaderSrcMasked.vert");
const FRAGMENT_SHADER_SRC_MASK: &str = include_str!("./shaders/FragShaderSrcMask.frag");
const FRAGMENT_SHADER_SRC_MASK_INVERTED: &str =
  include_str!("./shaders/FragShaderSrcMaskInverted.frag");

struct Shader {
  program: glow::Program,

  texture0: Option<glow::UniformLocation>,
  texture1: Option<glow::UniformLocation>,
  channel_flag: Option<glow::UniformLocation>,
  base_color: Option<glow::UniformLocation>,
  multiply_color: Option<glow::UniformLocation>,
  screen_color: Option<glow::UniformLocation>,
  clip_matrix: Option<glow::UniformLocation>,
  matrix: Option<glow::UniformLocation>,
}

fn create_shader_from_source(
  gl: &glow::Context,
  vertex_src: &str,
  fragment_src: &str,
) -> anyhow::Result<Shader> {
  let program = create_program_from_source(gl, vertex_src, fragment_src)?;

  unsafe {
    Ok(Shader {
      program,
      matrix: gl.get_uniform_location(program, "u_matrix"),
      texture0: gl.get_uniform_location(program, "s_texture0"),
      texture1: gl.get_uniform_location(program, "s_texture1"),
      channel_flag: gl.get_uniform_location(program, "u_channelFlag"),
      base_color: gl.get_uniform_location(program, "u_baseColor"),
      multiply_color: gl.get_uniform_location(program, "u_multiplyColor"),
      screen_color: gl.get_uniform_location(program, "u_screenColor"),
      clip_matrix: gl.get_uniform_location(program, "u_clipMatrix"),
    })
  }
}

struct ModelShaders {
  setup: Shader,
  normal: Shader,
  masked: Shader,
  inverted_mask: Shader,
}

impl ModelShaders {
  pub fn new(gl: &glow::Context) -> anyhow::Result<Self> {
    let setup = create_shader_from_source(
      gl,
      VERTEX_SHADER_SRC_SETUP_MASK,
      FRAGMENT_SHADER_SRC_SETUP_MASK,
    )?;
    let normal = create_shader_from_source(gl, VERTEX_SHADER_SRC, FRAGMENT_SHADER_SRC)?;
    let masked = create_shader_from_source(gl, VERTEX_SHADER_SRC_MASKED, FRAGMENT_SHADER_SRC_MASK)?;
    let inverted_mask = create_shader_from_source(
      gl,
      VERTEX_SHADER_SRC_MASKED,
      FRAGMENT_SHADER_SRC_MASK_INVERTED,
    )?;

    Ok(Self {
      setup,
      normal,
      masked,
      inverted_mask,
    })
  }
}

pub struct ModelRenderer {
  shaders: ModelShaders,
  ctx: Rc<RenderContext>,
  layout: Layout,
}

impl ModelRenderer {
  pub fn new(ctx: Rc<RenderContext>, layout: Layout) -> anyhow::Result<Self> {
    let shaders = ModelShaders::new(ctx.get_context())?;

    Ok(Self {
      ctx,
      shaders,
      layout,
    })
  }

  pub fn layout(&self) -> Layout {
    self.layout
  }

  pub fn draw(&self, model: &Model, matrix: glam::Mat4) {
    let projection = glam::camera::rh::proj::opengl::orthographic(
      self.layout.x,
      self.layout.x + self.layout.width,
      -(self.layout.y + self.layout.height),
      -self.layout.y,
      -1.0,
      1.0,
    );
    let center =
      glam::Mat4::from_translation(glam::vec3(self.layout.center_x, -self.layout.center_y, 0.0));

    let mvp = projection * center * matrix;

    self.draw_masks(model);
    self.draw_model(model, mvp);
  }

  fn draw_masks(&self, model: &Model) {
    let clipping_manager = model.get_clipping_manager();
    let gl = self.ctx.get_context();

    unsafe {
      gl.disable(glow::DEPTH_TEST);
      gl.disable(glow::CULL_FACE);

      gl.enable(glow::BLEND);

      // MASK_BLENDING
      gl.blend_equation_separate(glow::FUNC_ADD, glow::FUNC_ADD);

      gl.blend_func_separate(
        glow::ZERO,
        glow::ONE_MINUS_SRC_COLOR,
        glow::ZERO,
        glow::ONE_MINUS_SRC_ALPHA,
      );
    }

    // limpiar todos los framebuffers de máscara
    for offscreen in clipping_manager.get_offscreens() {
      let _fbo = self.ctx.push_framebuffer(Some(offscreen.framebuffer));

      unsafe {
        gl.viewport(0, 0, config::MASK_SIZE as i32, config::MASK_SIZE as i32);

        gl.clear_color(1.0, 1.0, 1.0, 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT);
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

          let fb = clipping_manager
            .get_offscreen_by_idx(cc.get_offscreen_index())
            .framebuffer;

          let _fbo = self.ctx.push_framebuffer(Some(fb));

          gl.viewport(0, 0, config::MASK_SIZE as i32, config::MASK_SIZE as i32);

          //-------------------------
          // shader
          //-------------------------
          let shader = &self.shaders.setup;

          gl.use_program(Some(shader.program));

          //-------------------------
          // texture
          //-------------------------
          gl.active_texture(glow::TEXTURE0);

          gl.bind_texture(glow::TEXTURE_2D, Some(*model.get_texture()));

          gl.uniform_1_i32(shader.texture0.as_ref(), 0);

          //-------------------------
          // uniforms
          //-------------------------
          gl.uniform_matrix_4_f32_slice(
            shader.clip_matrix.as_ref(),
            false,
            cc.get_matrix_for_mask().as_ref(),
          );

          let bounds = cc.get_layout_bounds();

          gl.uniform_4_f32(
            shader.base_color.as_ref(),
            bounds.x * 2.0 - 1.0,
            bounds.y * 2.0 - 1.0,
            bounds.right() * 2.0 - 1.0,
            bounds.bottom() * 2.0 - 1.0,
          );

          let c = cc.get_color_channel();

          gl.uniform_4_f32(shader.channel_flag.as_ref(), c[0], c[1], c[2], c[3]);

          gl.uniform_4_f32(shader.multiply_color.as_ref(), 1.0, 1.0, 1.0, 1.0);

          gl.uniform_4_f32(shader.screen_color.as_ref(), 0.0, 0.0, 0.0, 0.0);

          model.get_mesh_by_index(draw_index).draw(gl);
        }
      }
    }
  }

  fn draw_model(&self, model: &Model, mvp: glam::Mat4) {
    let clipping_manager = model.get_clipping_manager();
    let gl = self.ctx.get_context();

    unsafe {
      gl.viewport(
        self.layout.x as i32,
        self.layout.y as i32,
        self.layout.width as i32,
        self.layout.height as i32,
      );

      gl.enable(glow::BLEND);
      gl.disable(glow::DEPTH_TEST);
      gl.disable(glow::CULL_FACE);

      gl.clear_color(0.0, 0.0, 0.0, 0.0);
      gl.clear(glow::COLOR_BUFFER_BIT);
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
            Some(clipping_manager.get_offscreen_by_idx(cc.get_offscreen_index())),
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
        Self::set_blend_mode(gl, cflags);
      }

      //----------------------------------
      // Program
      //----------------------------------
      unsafe {
        gl.use_program(Some(shader.program));

        gl.uniform_matrix_4_f32_slice(shader.matrix.as_ref(), false, mvp.as_ref());

        gl.uniform_4_f32(shader.base_color.as_ref(), 1.0, 1.0, 1.0, drawable.opacity);

        gl.uniform_4_f32(shader.multiply_color.as_ref(), 1.0, 1.0, 1.0, 1.0);

        gl.uniform_4_f32(shader.screen_color.as_ref(), 0.0, 0.0, 0.0, 0.0);

        gl.uniform_4_f32(
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
        gl.active_texture(glow::TEXTURE0);

        gl.bind_texture(glow::TEXTURE_2D, Some(*model.get_texture()));

        gl.uniform_1_i32(shader.texture0.as_ref(), 0);
      }

      //----------------------------------
      // Textura de máscara
      //----------------------------------

      if let Some(mask) = mask_texture {
        unsafe {
          gl.active_texture(glow::TEXTURE1);

          gl.bind_texture(glow::TEXTURE_2D, Some(mask.texture));

          gl.uniform_1_i32(shader.texture1.as_ref(), 1);

          gl.uniform_matrix_4_f32_slice(
            shader.clip_matrix.as_ref(),
            false,
            clip_matrix.unwrap().as_ref(),
          );
        }
      }

      //----------------------------------
      // Draw
      //----------------------------------
      model.get_mesh_by_index(drawable.index).draw(gl);
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

impl Drop for ModelRenderer {
  fn drop(&mut self) {
    let gl = self.ctx.get_context();
    unsafe {
      gl.delete_program(self.shaders.setup.program);
      gl.delete_program(self.shaders.normal.program);
      gl.delete_program(self.shaders.masked.program);
      gl.delete_program(self.shaders.inverted_mask.program);
    }
  }
}
