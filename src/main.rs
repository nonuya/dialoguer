mod app;
mod live2d;

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
  app: Option<app::App>,
}

impl MainWindow {
  fn new() -> Self {
    MainWindow {
      window: None,
      display: None,
      app: None,
    }
  }
}

impl ApplicationHandler for MainWindow {
  fn resumed(&mut self, event_loop: &glium::winit::event_loop::ActiveEventLoop) {
    let (window, display) = SimpleWindowBuilder::new()
      .with_title("IVAV")
      .with_inner_size(800, 400)
      .build(event_loop);

    match app::App::new(&display) {
      Ok(app) => self.app = Some(app),
      Err(e) => {
        error!("[MainWindow] {e}");
        event_loop.exit();
      }
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
          self.app.as_ref().unwrap().draw(display);
        }
      }
      WindowEvent::CloseRequested => event_loop.exit(),
      _ => {}
    }
  }
}

fn main() -> anyhow::Result<()> {
  env_logger::init();

  let mut main_window = MainWindow::new();
  let event_loop = glium::winit::event_loop::EventLoop::builder().build()?;
  event_loop.run_app(&mut main_window)?;

  Ok(())
}
