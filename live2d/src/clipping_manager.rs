use cubism::model::UserModel;
use glam::{Mat4, Vec3};
use glow::HasContext;
use std::{collections::HashMap, rc::Rc};

use crate::{config, rectf::RectF};

#[derive(Default)]
pub struct ClippingContext {
  ids: Vec<i32>,
  drawable_ids: Vec<u32>,
  is_using: bool,
  all_clipped_draw_rect: RectF,
  layout_channel_index: u32,
  layout_bounds: RectF,
  buffer_index: u32,
  matrix_for_mask: Mat4,
  matrix_for_draw: Mat4,
}

impl ClippingContext {
  pub fn get_draw_indices(&self) -> &Vec<i32> {

    &self.ids
  }

  pub fn get_color_channel(&self) -> [f32; 4] {
    assert!(self.layout_channel_index < 4);

    let mut result = [0.0, 0.0, 0.0, 0.0];

    result[self.layout_channel_index as usize] = 1.0;

    result
  }

  pub fn get_layout_bounds(&self) -> &RectF {
    &self.layout_bounds
  }

  pub fn get_matrix_for_mask(&self) -> &Mat4 {
    &self.matrix_for_mask
  }

  pub fn get_matrix_for_draw(&self) -> &Mat4 {
    &self.matrix_for_draw
  }

  pub fn get_offscreen_index(&self) -> usize {
    self.buffer_index as usize
  }
}

struct ClippingGraph {
  ccs_for_mask: Vec<ClippingContext>,
  ccs_for_draw: HashMap<usize, usize>, // <drawable_index, masks_index>
}

impl ClippingGraph {
  // TODO: Transcipted Live2D code
  fn new(cubism: &UserModel) -> Self {
    let mut ccs_for_mask: Vec<ClippingContext> = Vec::new();
    let mut ccs_for_draw = HashMap::new();

    // =================================
    // Graph: O(n^3)?? Meh
    // =================================
    for drawable in cubism.drawables() {
      if !drawable.is_masked() {
        continue;
      }

      let find_same_clip = |masks: &[i32]| {
        ccs_for_mask.iter().position(|cc| {
          cc.ids.len() == masks.len()
            && cc.ids.iter().all(|id| masks.contains(id))
            && masks.iter().all(|id| cc.ids.contains(id))
        })
      };

      let index = if let Some(idx) = find_same_clip(drawable.masks) {
        idx
      } else {
        ccs_for_mask.push(ClippingContext {
          ids: drawable.masks.to_vec(),
          ..Default::default()
        });

        ccs_for_mask.len() - 1
      };

      ccs_for_mask[index].drawable_ids.push(drawable.index as u32);

      assert!(!ccs_for_draw.contains_key(&drawable.index));
      ccs_for_draw.insert(drawable.index, index);
    }
    // =================================

    Self {
      ccs_for_mask,
      ccs_for_draw,
    }
  }

  pub fn setup(&mut self, cubism: &UserModel, mask_buffer_count: u32) {
    for cc in &mut self.ccs_for_mask {
      (cc.is_using, cc.all_clipped_draw_rect) = Self::calc_clipped_draw_total_bounds(cc, cubism);
    }

    let using_clip_count = self.ccs_for_mask.iter().filter(|cc| cc.is_using).count() as u32;

    assert!(using_clip_count != 0, "Unimplemented case here");

    let mut setup_layout_bounds = || {
      const COLOR_CHANNEL_COUNT: u32 = 4;
      const CLIPPING_MASK_MAX_COUNT_ON_DEFAULT: u32 = 36;
      const CLIPPING_MASK_MAX_COUNT_ON_MULTI_RENDER_TEXTURE: u32 = 32;

      let use_clipping_mask_max_count = if mask_buffer_count <= 1 {
        CLIPPING_MASK_MAX_COUNT_ON_DEFAULT
      } else {
        CLIPPING_MASK_MAX_COUNT_ON_MULTI_RENDER_TEXTURE * mask_buffer_count
      };

      // Caso de overflow o sin máscaras
      if using_clip_count == 0 || using_clip_count > use_clipping_mask_max_count {
        if using_clip_count > use_clipping_mask_max_count {
          log::error!(
            "Not supported mask count.\nrender textures: {}\nmask count: {}",
            mask_buffer_count,
            using_clip_count
          );
        }

        for cc in &mut self.ccs_for_mask {
          cc.layout_channel_index = 0;
          cc.layout_bounds = RectF {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
          };
          cc.buffer_index = 0;
        }

        return;
      }

      let layout_count_max = if mask_buffer_count <= 1 { 9 } else { 8 };

      let count_per_sheet = (using_clip_count + mask_buffer_count - 1) / mask_buffer_count;

      let reduce_layout_texture_count = using_clip_count % mask_buffer_count;

      let div_count = count_per_sheet / COLOR_CHANNEL_COUNT;
      let mod_count = count_per_sheet % COLOR_CHANNEL_COUNT;

      let mut clip_index = 0;

      for render_texture in 0..mask_buffer_count {
        for channel in 0..COLOR_CHANNEL_COUNT {
          let mut layout_count = div_count + u32::from(channel < mod_count);

          let check_channel = mod_count as isize + if div_count == 0 { -1 } else { 0 };

          if channel as isize == check_channel
            && reduce_layout_texture_count > 0
            && render_texture >= reduce_layout_texture_count
          {
            layout_count -= 1;
          }

          let mut assign_layout =
            |clip_index: usize, channel: u32, buffer: u32, x: f32, y: f32, w: f32, h: f32| {
              let cc = &mut self.ccs_for_mask[clip_index];

              cc.layout_channel_index = channel;
              cc.buffer_index = buffer;

              cc.layout_bounds = RectF {
                x,
                y,
                width: w,
                height: h,
              };
            };

          match layout_count {
            0 => {}

            1 => {
              assign_layout(clip_index, channel, render_texture, 0.0, 0.0, 1.0, 1.0);
              clip_index += 1;
            }

            2 => {
              for i in 0..2 {
                assign_layout(
                  clip_index,
                  channel,
                  render_texture,
                  (i % 2) as f32 * 0.5,
                  0.0,
                  0.5,
                  1.0,
                );
                clip_index += 1;
              }
            }

            3..=4 => {
              for i in 0..layout_count {
                let x = (i % 2) as f32 * 0.5;
                let y = (i / 2) as f32 * 0.5;

                assign_layout(clip_index, channel, render_texture, x, y, 0.5, 0.5);

                clip_index += 1;
              }
            }

            5..=9 if layout_count <= layout_count_max => {
              for i in 0..layout_count {
                let x = (i % 3) as f32 / 3.0;
                let y = (i / 3) as f32 / 3.0;

                assign_layout(
                  clip_index,
                  channel,
                  render_texture,
                  x,
                  y,
                  1.0 / 3.0,
                  1.0 / 3.0,
                );

                clip_index += 1;
              }
            }

            _ => {
              panic!("Unsupported mask count");
            }
          }
        }
      }
    };

    // TODO: This is horrible but i dont care :D
    setup_layout_bounds();

    for cc in &mut self.ccs_for_mask {
      const MARGIN: f32 = 0.05;

      let all_clipped_draw_rect = &cc.all_clipped_draw_rect;
      let layout_bounds_on_tex_01 = &cc.layout_bounds;
      let tmp_bounds_on_model = all_clipped_draw_rect.expand(
        all_clipped_draw_rect.width * MARGIN,
        all_clipped_draw_rect.height * MARGIN,
      );

      let scale_x = layout_bounds_on_tex_01.width / tmp_bounds_on_model.width;
      let scale_y = layout_bounds_on_tex_01.height / tmp_bounds_on_model.height;

      (cc.matrix_for_mask, cc.matrix_for_draw) = Self::create_matrix_for_mask(
        false,
        tmp_bounds_on_model,
        layout_bounds_on_tex_01,
        scale_x,
        scale_y,
      );
    }
  }

  // TODO: A try of functional code
  fn calc_clipped_draw_total_bounds(cc: &ClippingContext, cubism: &UserModel) -> (bool, RectF) {
    let mut total_min_x = f32::INFINITY;
    let mut total_min_y = f32::INFINITY;
    let mut total_max_x = f32::NEG_INFINITY;
    let mut total_max_y = f32::NEG_INFINITY;

    for &drawable_index in &cc.drawable_ids {
      let (min_x, min_y, max_x, max_y) = cubism
        .drawable_vertex_positions(drawable_index as usize)
        .iter()
        .fold(
          (
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
          ),
          |(min_x, min_y, max_x, max_y), &[x, y]| {
            (min_x.min(x), min_y.min(y), max_x.max(x), max_y.max(y))
          },
        );

      if min_x == f32::INFINITY {
        continue;
      }

      total_min_x = total_min_x.min(min_x);
      total_min_y = total_min_y.min(min_y);
      total_max_x = total_max_x.max(max_x);
      total_max_y = total_max_y.max(max_y);
    }

    if total_min_x == f32::INFINITY {
      (false, RectF::default())
    } else {
      (
        true,
        RectF {
          x: total_min_x,
          y: total_min_y,
          width: total_max_x - total_min_x,
          height: total_max_y - total_min_y,
        },
      )
    }
  }

  // TODO: A try of functional code
  fn create_matrix_for_mask(
    is_right_handed: bool,
    tmp_bounds_on_model: RectF,
    layout_bounds_on_tex_01: &RectF,
    scale_x: f32,
    scale_y: f32,
  ) -> (Mat4 /*matrix_for_mask*/, Mat4 /*matrix_for_draw*/) {
    // ---------- MatrixForMask ----------
    let mut m = Mat4::IDENTITY;

    // Layout [0,1] -> [-1,1]
    m *= Mat4::from_translation(Vec3::new(-1.0, -1.0, 0.0));
    m *= Mat4::from_scale(Vec3::new(2.0, 2.0, 1.0));

    // View -> Layout
    m *= Mat4::from_translation(Vec3::new(
      layout_bounds_on_tex_01.x,
      layout_bounds_on_tex_01.y,
      0.0,
    ));
    m *= Mat4::from_scale(Vec3::new(scale_x, scale_y, 1.0));
    m *= Mat4::from_translation(Vec3::new(
      -tmp_bounds_on_model.x,
      -tmp_bounds_on_model.y,
      0.0,
    ));

    // ---------- MatrixForDraw ----------
    let sign = if is_right_handed { -1.0 } else { 1.0 };

    let mut d = Mat4::IDENTITY;

    d *= Mat4::from_translation(Vec3::new(
      layout_bounds_on_tex_01.x,
      layout_bounds_on_tex_01.y * sign,
      0.0,
    ));

    d *= Mat4::from_scale(Vec3::new(scale_x, scale_y * sign, 1.0));

    d *= Mat4::from_translation(Vec3::new(
      -tmp_bounds_on_model.x,
      -tmp_bounds_on_model.y,
      0.0,
    ));

    (m, d)
  }
}

pub struct ClippingManager {
  gl: Rc<glow::Context>,
  graph: ClippingGraph,
  offscreens: Vec<Offscreen>,
}

impl Drop for ClippingManager {
  fn drop(&mut self) {
    for offscreen in &self.offscreens {
      unsafe {
        self.gl.delete_texture(offscreen.texture);
        self.gl.delete_framebuffer(offscreen.framebuffer);
      }
    }
  }
}

impl ClippingManager {
  pub fn new(
    gl: Rc<glow::Context>,
    cubism: &UserModel,
    mask_buffer_count: u32,
  ) -> anyhow::Result<Self> {
    assert_ne!(
      mask_buffer_count, 0,
      "The number of render textures must be an integer greater than or equal to 1. Set the number of render textures to 1."
    );

    assert!(
      cubism.is_masked(),
      "Until now we just implemented for masked drawables"
    );

    let create_texture = || unsafe {
      let texture = gl.create_texture().map_err(anyhow::Error::msg)?;

      gl.bind_texture(glow::TEXTURE_2D, Some(texture));

      gl.tex_image_2d(
        glow::TEXTURE_2D,
        0,
        glow::RGBA8 as i32,
        config::MASK_SIZE as i32,
        config::MASK_SIZE as i32,
        0,
        glow::RGBA,
        glow::UNSIGNED_BYTE,
        glow::PixelUnpackData::Slice(None),
      );

      gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_MIN_FILTER,
        glow::LINEAR as i32,
      );

      gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_MAG_FILTER,
        glow::LINEAR as i32,
      );

      gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_WRAP_S,
        glow::CLAMP_TO_EDGE as i32,
      );

      gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_WRAP_T,
        glow::CLAMP_TO_EDGE as i32,
      );

      let framebuffer = gl.create_framebuffer().map_err(anyhow::Error::msg)?;

      gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));

      gl.framebuffer_texture_2d(
        glow::FRAMEBUFFER,
        glow::COLOR_ATTACHMENT0,
        glow::TEXTURE_2D,
        Some(texture),
        0,
      );

      if gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
        anyhow::bail!("Framebuffer is incomplete");
      }

      gl.bind_texture(glow::TEXTURE_2D, None);
      gl.bind_framebuffer(glow::FRAMEBUFFER, None);

      Ok(Offscreen {
        texture,
        framebuffer,
      })
    };

    let mut graph = ClippingGraph::new(cubism);
    let offscreens = (0..mask_buffer_count)
      .map(|_| {create_texture()})
      .collect::<anyhow::Result<Vec<_>>>()?;

    graph.setup(cubism, mask_buffer_count);

    Ok(Self {
      gl,
      graph,
      offscreens,
    })
  }

  pub fn update_graph(&mut self, cubism: &UserModel) {
    self.graph.setup(cubism, self.offscreens.len() as u32);
  }

  pub fn get_clipping_contexts_for_mask(&self) -> &Vec<ClippingContext> {
    &self.graph.ccs_for_mask
  }

  pub fn try_get_clipping_context_for_draw(
    &self,
    drawable_index: usize,
  ) -> Option<&ClippingContext> {
    self
      .graph
      .ccs_for_draw
      .get(&drawable_index)
      .map(|&index| &self.graph.ccs_for_mask[index])
  }

  pub fn get_offscreens(&self) -> &Vec<Offscreen> {
    &self.offscreens
  }

  pub fn get_offscreen_by_idx(&self, buffer_index: usize) -> &Offscreen {
    assert!(buffer_index < 4, "Buffer index must be until 3 'cause RGBA");
    &self.offscreens[buffer_index]
  }
}

pub struct Offscreen {
  pub texture: glow::Texture,
  pub framebuffer: glow::Framebuffer,
}
