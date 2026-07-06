mod app;

use glium::{
  Display,
  backend::glutin::SimpleWindowBuilder,
  glutin::surface::WindowSurface,
  winit::{application::ApplicationHandler, window::Window},
};
use log::error;

struct MainWindow {
  // This fields are optional because we need to create them later.
  window: Option<Window>,
  display: Option<Display<WindowSurface>>,
  app: app::App,
}

impl MainWindow {
  fn new() -> Self {
    MainWindow {
      window: None,
      display: None,
      app: app::App {},
    }
  }
}

impl ApplicationHandler for MainWindow {
  fn resumed(&mut self, event_loop: &glium::winit::event_loop::ActiveEventLoop) {
    let (window, display) = SimpleWindowBuilder::new()
      .with_title("IVAV")
      .with_inner_size(800, 400)
      .build(event_loop);

    if let Err(e) = self.app.initialize(&display) {
      error!("[MainWindow] {e}");
      event_loop.exit();
    }

    self.window = Some(window);
    self.display = Some(display);
  }

  fn window_event(
    &mut self,
    event_loop: &glium::winit::event_loop::ActiveEventLoop,
    _window_id: glium::winit::window::WindowId,
    event: glium::winit::event::WindowEvent,
  ) {
    use glium::winit::event::WindowEvent;

    match event {
      WindowEvent::RedrawRequested => {
        if let Some(display) = &self.display {
          let mut frame = display.draw();
          
          self.app.draw(&mut frame);

          frame.finish().unwrap();
        }
      }
      WindowEvent::CloseRequested => event_loop.exit(),
      _ => {}
    }
  }
}

/*
impl App {
  fn draw(&self, frame: &mut Frame) {
    use cubism::core::DynamicFlags;

    frame.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);

    let mut drawables: Vec<_> = self.model.drawables().collect();
    drawables.sort_unstable_by_key(|d| d.render_order);

    for drawable in &drawables {
      let dflags = drawable.dynamic_flags;
      if drawable.opacity <= 0.0 || !dflags.intersects(DynamicFlags::IS_VISIBLE) {
        continue;
      }

      let vb = &self.vertex_buffers[drawable.index];
      let ib = &self.indices_buffers[drawable.index];
      let u_texture = &self.textures[drawable.texture_index as usize];

      if dflags.intersects(DynamicFlags::VERTEX_POSITIONS_CHANGED) {
        let vtx_buffer: Vec<_> = drawable
          .vertex_positions
          .iter()
          .zip(drawable.vertex_uvs)
          .map(|(&pos, &uv)| Vertex { pos, uv })
          .collect();
        vb.write(&vtx_buffer);
      }

      let params = glium::DrawParameters {
        depth: Depth {
          test: DepthTest::IfLessOrEqual,
          write: true,
          ..Default::default()
        },

        blend: Blend::alpha_blending(),

        ..Default::default()
      };

      let uniforms = glium::uniform! {
          u_texture: u_texture,
      };

      frame
        .draw(vb, ib, self.program.as_ref().unwrap(), &uniforms, &params)
        .unwrap();
    }
  }
}

#[derive(Copy, Clone, Debug)]
struct Vertex {
  pos: [f32; 2],
  uv: [f32; 2],
}
implement_vertex!(Vertex, pos, uv);
implement_uniform_block!(Vertex, pos);
*/

fn main() -> anyhow::Result<()> {
  env_logger::init();

  let mut main_window = MainWindow::new();
  let event_loop = glium::winit::event_loop::EventLoop::builder().build()?;
  event_loop.run_app(&mut main_window)?;

  Ok(())
}
