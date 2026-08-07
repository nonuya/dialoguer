use std::rc::Rc;
use glow::HasContext;
use anyhow::Context;

pub struct TextureTarget {
  fbo: glow::Framebuffer,
  texture: glow::Texture,
  gl: Rc<glow::Context>,
}

impl TextureTarget {
  pub fn new(gl: Rc<glow::Context>, width: u32, height: u32) -> anyhow::Result<Self> {
    let texture = unsafe {
      gl.create_texture()
        .map_err(anyhow::Error::msg)
        .context("Failed to create texture for render Model")?
    };

    unsafe {
      // 1. Crear la textura donde vas a "pintar"
      gl.bind_texture(glow::TEXTURE_2D, Some(texture));
      gl.tex_image_2d(
        glow::TEXTURE_2D,
        0,
        glow::RGBA8 as i32,
        width as i32,
        height as i32,
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
      gl.bind_texture(glow::TEXTURE_2D, None);
    }

    let fbo = unsafe {
      gl.create_framebuffer()
        .map_err(anyhow::Error::msg)
        .context("Failed to create framebuffer for render Model")?
    };
    unsafe {
      gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
      gl.framebuffer_texture(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, Some(texture), 0);

      anyhow::ensure!(gl.check_framebuffer_status(glow::FRAMEBUFFER) == glow::FRAMEBUFFER_COMPLETE);

      gl.bind_framebuffer(glow::FRAMEBUFFER, None);
    }

    Ok(Self {
      fbo,
      texture,
      gl,
    })
  }

  pub fn draw<F: Fn() -> ()>(&self, f: F) {
    unsafe {
      self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
    }

    f();

    unsafe {
      self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
    }
  }

  pub fn tex(&self) -> glow::Texture {
    self.texture
  }
}

impl Drop for TextureTarget {
  fn drop(&mut self) {
    unsafe {
      self.gl.delete_texture(self.texture);
      self.gl.delete_framebuffer(self.fbo);
    }
  }
}
