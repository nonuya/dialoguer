#[derive(Default)]
pub struct RectF {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
}

impl RectF {
  pub fn expand(&self, width: f32, height: f32) -> Self {
    RectF {
      x: self.x - width,
      y: self.y - height,
      width: self.width + width*2.0,
      height: self.height + height*2.0,
    }
  } 

  pub fn bottom(&self) -> f32 {
    self.y + self.height
  }

  pub fn right(&self) -> f32 {
    self.x + self.width
  }
}
