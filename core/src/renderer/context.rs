use std::{cell::Cell, rc::Rc};

use glow::HasContext;

pub struct FramebufferGuard<'a> {
  previous: Option<glow::Framebuffer>,
  ctx: &'a RenderContext,
}

impl<'a> Drop for FramebufferGuard<'a> {
  fn drop(&mut self) {
    self.ctx.last_fbo.set(self.previous);
    unsafe {
      self.ctx.gl.bind_framebuffer(glow::FRAMEBUFFER, self.previous);
    }
  }
}

pub struct RenderContext {
  gl: Rc<glow::Context>,
  last_fbo: Cell<Option<glow::Framebuffer>>,
}

impl RenderContext {
  pub fn from_gl(gl: Rc<glow::Context>) -> Self {
    Self {
      gl,
      last_fbo: Cell::new(None)
    }
  }

  pub fn get_context(&self) -> &glow::Context {
    &self.gl
  }

  pub (in crate) fn push_framebuffer(&self, fbo: Option<glow::Framebuffer>) -> FramebufferGuard<'_> {
    let previous = self.last_fbo.replace(fbo);

    unsafe {
      self.gl.bind_framebuffer(glow::FRAMEBUFFER, fbo);
    }

    FramebufferGuard { previous, ctx: self }
  }
}
