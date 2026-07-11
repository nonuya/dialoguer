use cubism::core::{ConstantFlags, DynamicFlags};
use glam::{Mat4, vec3};
use glium::{
  BackfaceCullingMode, Blend, BlendingFunction, DepthTest, Display, DrawParameters, Frame,
  LinearBlendingFactor, Rect, Surface,
  framebuffer::{self, SimpleFrameBuffer},
  glutin::surface::WindowSurface,
  uniform,
};

use crate::live2d::{self, vertex::Vertex};

const MASK_BLENDING: Blend = Blend {
  color: BlendingFunction::Addition {
    source: LinearBlendingFactor::Zero,
    destination: LinearBlendingFactor::OneMinusSourceColor,
  },
  alpha: BlendingFunction::Addition {
    source: LinearBlendingFactor::Zero,
    destination: LinearBlendingFactor::OneMinusSourceColor,
  },
  constant_value: (0.0, 0.0, 0.0, 0.0),
};

// FIXME [11-07-2026]: Alpha channel isn't drawing

pub fn draw_masks(
  display: &Display<WindowSurface>,
  model: &live2d::model::Model,
  shaders: &live2d::shaders::GlobalShaders,
) {
  let clipping_manager = model.get_clipping_manager();

  let mut offscreen_surfaces: Vec<_> = clipping_manager
    .get_offscreen_surfaces()
    .iter()
    .map(|t| SimpleFrameBuffer::new(display, t.get_texture()).unwrap())
    .collect();

  for offscreen in &mut offscreen_surfaces {
    // Draw masks
    offscreen.clear_color(1.0, 1.0, 1.0, 1.0);
  }

  for cc in clipping_manager.get_clipping_contexts_for_mask() {
    for &clip_draw_index in cc.draw_indices() {
      let clip_draw_index = clip_draw_index as usize;
      let dflags = model.get_drawable_dynamic_flag(clip_draw_index);

      if !dflags.intersects(DynamicFlags::VERTEX_POSITIONS_CHANGED) {
        continue;
      }

      let s_texture0 = model.get_texture();
      let vertices = model.get_drawable_vertices(clip_draw_index);
      let uvs = model.get_drawable_indices(clip_draw_index);
      let u_channel_flag = cc.get_color_channel();
      let u_clip_matrix = Some(cc.get_matrix_for_mask().clone());
      let layout_bounds = cc.get_layout_bounds();
      let u_base_color = [
        layout_bounds.x * 2.0 - 1.0,
        layout_bounds.y * 2.0 - 1.0,
        layout_bounds.right() * 2.0 - 1.0,
        layout_bounds.bottom() * 2.0 - 1.0,
      ];

      let uniforms = live2d::uniforms::CubismUniforms {
        s_texture0,
        s_texture1: None,
        u_clip_matrix,
        u_matrix: None,
        u_channel_flag,
        u_base_color,
        u_multiply_color: [1.0, 1.0, 1.0, 1.0],
        u_screen_color: [0.0, 0.0, 0.0, 0.0],
      };

      let draw_parameters = DrawParameters {
        blend: MASK_BLENDING,
        viewport: Some(Rect {
          left: 0,
          bottom: 0,
          width: 1024,
          height: 1024,
        }),
        depth: glium::Depth {
          test: DepthTest::Overwrite,
          write: false,
          ..Default::default()
        },
        backface_culling: BackfaceCullingMode::CullingDisabled,
        ..Default::default()
      };

      let buffer_index = cc.get_buffer_index() as usize;
      let framebuffer = &mut offscreen_surfaces[buffer_index];

      framebuffer
        .draw(vertices, uvs, &shaders.setup, &uniforms, &draw_parameters)
        .unwrap();
    }
  }
}

pub fn draw_model(
  frame: &mut Frame,
  model: &live2d::model::Model,
  shaders: &live2d::shaders::GlobalShaders,
) {
  let clipping_manager = model.get_clipping_manager();
  for drawable in model.get_sorted_drawables() {
    let dflags = drawable.dynamic_flags;
    let cflags = drawable.constant_flags;

    if drawable.opacity <= 0.0 || !dflags.intersects(DynamicFlags::IS_VISIBLE) {
      continue;
    }

    let (masked, s_texture1, u_channel_flag, u_clip_matrix) =
      if let Some(cc) = clipping_manager.try_get_clipping_context_for_draw(drawable.index) {
        (
          true,
          Some(clipping_manager.get_offscreen_surface(cc.get_buffer_index())),
          cc.get_color_channel(),
          Some(cc.get_matrix_for_draw().clone()),
        )
      } else {
        (false, None, [0.0, 0.0, 0.0, 0.0], None)
      };

    let inverted_mask = cflags.intersects(ConstantFlags::IS_INVERTED_MASK);

    let program = if masked {
      if inverted_mask {
        &shaders.inverted_mask
      } else {
        &shaders.masked
      }
    } else {
      &shaders.normal
    };

    let vertices = model.get_drawable_vertices(drawable.index);
    let uvs = model.get_drawable_indices(drawable.index);

    let draw_parameters = DrawParameters {
      blend: get_draw_blend_from_cflags(cflags),
      ..Default::default()
    };

    let uniforms = live2d::uniforms::CubismUniforms {
      s_texture0: model.get_texture(),
      s_texture1,
      u_clip_matrix,
      u_matrix: Some(glam::Mat4::from_scale(vec3(2.0, 2.0, 2.0))),
      u_channel_flag,
      u_base_color: [1.0, 1.0, 1.0, drawable.opacity],
      u_multiply_color: [1.0, 1.0, 1.0, 1.0],
      u_screen_color: [0.0, 0.0, 0.0, 0.0],
    };

    frame
      .draw(vertices, uvs, program, &uniforms, &draw_parameters)
      .unwrap();
  }
}

fn get_draw_blend_from_cflags(cflags: ConstantFlags) -> Blend {
  if cflags.intersects(ConstantFlags::BLEND_ADDITIVE) {
    Blend {
      color: BlendingFunction::Addition {
        source: LinearBlendingFactor::One,

        destination: LinearBlendingFactor::One,
      },

      alpha: BlendingFunction::Addition {
        source: LinearBlendingFactor::Zero,

        destination: LinearBlendingFactor::One,
      },

      constant_value: (0.0, 0.0, 0.0, 0.0),
    }
  } else if cflags.intersects(ConstantFlags::BLEND_MULTIPLICATIVE) {
    Blend {
      color: BlendingFunction::Addition {
        source: LinearBlendingFactor::DestinationColor,

        destination: LinearBlendingFactor::OneMinusSourceAlpha,
      },

      alpha: BlendingFunction::Addition {
        source: LinearBlendingFactor::Zero,

        destination: LinearBlendingFactor::One,
      },

      constant_value: (0.0, 0.0, 0.0, 0.0),
    }
  } else {
    Blend {
      color: BlendingFunction::Addition {
        source: LinearBlendingFactor::One,

        destination: LinearBlendingFactor::OneMinusSourceAlpha,
      },

      alpha: BlendingFunction::Addition {
        source: LinearBlendingFactor::One,

        destination: LinearBlendingFactor::OneMinusSourceAlpha,
      },

      constant_value: (0.0, 0.0, 0.0, 0.0),
    }
  }
}
