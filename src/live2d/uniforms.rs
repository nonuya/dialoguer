use glam::Mat4;
use glium::uniforms::{UniformValue, Uniforms};

#[derive(Copy, Clone)]
pub struct CubismUniforms<'a> {
  pub s_texture0: &'a glium::texture::Texture2d,

  pub s_texture1: Option<&'a glium::texture::Texture2d>,

  pub u_matrix: Option<Mat4>,

  pub u_clip_matrix: Option<Mat4>,

  pub u_channel_flag: [f32; 4],

  pub u_base_color: [f32; 4],

  pub u_multiply_color: [f32; 4],

  pub u_screen_color: [f32; 4],
}

impl<'a> Uniforms for CubismUniforms<'a> {
  fn visit_values<'b, F>(&'b self, mut set: F)
  where
    F: FnMut(&str, UniformValue<'b>),
  {
    set("s_texture0", UniformValue::Texture2d(self.s_texture0, None));

    if let Some(tex) = self.s_texture1 {
      set("s_texture1", UniformValue::Texture2d(tex, None));
    }

    if let Some(matrix) = self.u_matrix {
      set("u_matrix", UniformValue::Mat4(matrix.to_cols_array_2d()));
    }

    if let Some(matrix) = self.u_clip_matrix {
      set("u_clipMatrix", UniformValue::Mat4(matrix.to_cols_array_2d()));
    }

    set("u_channelFlag", UniformValue::Vec4(self.u_channel_flag));

    set("u_baseColor", UniformValue::Vec4(self.u_base_color));

    set("u_multiplyColor", UniformValue::Vec4(self.u_multiply_color));

    set("u_screenColor", UniformValue::Vec4(self.u_screen_color));
  }
}
