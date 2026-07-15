mod app;
mod live2d;

use crate::app::App;
use glutin::config::{Config, ConfigTemplateBuilder, GetGlConfig};
use glutin::context::{
  ContextApi, ContextAttributesBuilder, NotCurrentContext, PossiblyCurrentContext, Version,
};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{Surface, SwapInterval, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow};
use log::{debug, error};
use raw_window_handle::HasWindowHandle;
use std::num::NonZeroU32;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowAttributes};

fn window_attributes() -> WindowAttributes {
  Window::default_attributes().with_title("IVAV")
}

fn main() -> anyhow::Result<()> {
  env_logger::init();
  // The template will match only the configurations supporting rendering
  // to windows.
  //
  // XXX We force transparency only on macOS, given that EGL on X11 doesn't
  // have it, but we still want to show window. The macOS situation is like
  // that, because we can query only one config at a time on it, but all
  // normal platforms will return multiple configs, so we can find the config
  // with transparency ourselves inside the `reduce`.
  let template = ConfigTemplateBuilder::new().with_alpha_size(8);

  let display_builder = DisplayBuilder::new().with_window_attributes(Some(window_attributes()));
  let event_loop = EventLoop::new()?;
  let mut main_window = MainWindow::new(template, display_builder);
  event_loop.run_app(&mut main_window)?;

  Ok(())
}

struct MainWindow {
  template: ConfigTemplateBuilder,
  app: Option<App>,
  // NOTE: `AppState` carries the `Window`, thus it should be dropped after everything else.
  state: Option<AppState>,
  gl_context: Option<PossiblyCurrentContext>,
  gl_display: GlDisplayCreationState,
  last_frame: Instant,
}

impl ApplicationHandler for MainWindow {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    let (window, gl_config) = match &self.gl_display {
      // We just created the event loop, so initialize the display, pick the config, and
      // create the context.
      GlDisplayCreationState::Builder(display_builder) => {
        let (window, gl_config) =
          match display_builder
            .clone()
            .build(event_loop, self.template.clone(), gl_config_picker)
          {
            Ok((window, gl_config)) => (window.unwrap(), gl_config),
            Err(err) => {
              error!("[DisplayBuilder] {:#}", err);
              event_loop.exit();
              return;
            }
          };

        // Mark the display as initialized to not recreate it on resume, since the
        // display is valid until we explicitly destroy it.
        self.gl_display = GlDisplayCreationState::Init;

        // Create gl context.
        self.gl_context = Some(create_gl_context(&window, &gl_config).treat_as_possibly_current());

        (window, gl_config)
      }
      GlDisplayCreationState::Init => {
        debug!("Recreating window in `resumed`");
        // Pick the config which we already use for the context.
        let gl_config = self.gl_context.as_ref().unwrap().config();
        match glutin_winit::finalize_window(event_loop, window_attributes(), &gl_config) {
          Ok(window) => (window, gl_config),
          Err(err) => {
            error!("[OsError] {:#}", err);
            event_loop.exit();
            return;
          }
        }
      }
    };

    let attrs = window
      .build_surface_attributes(Default::default())
      .expect("Failed to build surface attributes");
    let gl_surface = unsafe {
      gl_config
        .display()
        .create_window_surface(&gl_config, &attrs)
        .unwrap()
    };

    // The context needs to be current for the Renderer to set up shaders and
    // buffers. It also performs function loading, which needs a current context on
    // WGL.
    let gl_context = self.gl_context.as_ref().unwrap();
    gl_context.make_current(&gl_surface).unwrap();

    if self.app.is_none() {
      match App::new(
        &gl_config.display(),
        window.inner_size().width,
        window.inner_size().height,
      ) {
        Ok(app) => self.app = Some(app),
        Err(err) => {
          error!("[Application] {:#}", err);
          event_loop.exit();
          return;
        }
      }
    }

    // Try setting vsync.
    if let Err(res) =
      gl_surface.set_swap_interval(gl_context, SwapInterval::Wait(NonZeroU32::new(1).unwrap()))
    {
      error!("Error setting vsync: {res:?}");
    }

    assert!(
      self
        .state
        .replace(AppState { gl_surface, window })
        .is_none()
    );
  }

  fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
    // This event is only raised on Android, where the backing NativeWindow for a GL
    // Surface can appear and disappear at any moment.
    debug!("Android window removed");

    // Destroy the GL Surface and un-current the GL Context before ndk-glue releases
    // the window back to the system.
    self.state = None;

    // Make context not current.
    self.gl_context = Some(
      self
        .gl_context
        .take()
        .unwrap()
        .make_not_current()
        .unwrap()
        .treat_as_possibly_current(),
    );
  }

  fn window_event(
    &mut self,
    event_loop: &ActiveEventLoop,
    _window_id: winit::window::WindowId,
    event: WindowEvent,
  ) {
    match event {
      WindowEvent::Resized(size) if size.width != 0 && size.height != 0 => {
        // Some platforms like EGL require resizing GL surface to update the size
        // Notable platforms here are Wayland and macOS, other don't require it
        // and the function is no-op, but it's wise to resize it for portability
        // reasons.
        if let Some(AppState {
          gl_surface,
          window: _,
        }) = self.state.as_ref()
        {
          let gl_context = self.gl_context.as_ref().unwrap();
          gl_surface.resize(
            gl_context,
            NonZeroU32::new(size.width).unwrap(),
            NonZeroU32::new(size.height).unwrap(),
          );

          let app = self.app.as_mut().unwrap();
          app.resize(size.width, size.height);
        }
      }
      WindowEvent::CloseRequested
      | WindowEvent::KeyboardInput {
        event:
          KeyEvent {
            logical_key: Key::Named(NamedKey::Escape),
            ..
          },
        ..
      } => event_loop.exit(),
      _ => (),
    }
  }

  fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
    // NOTE: The handling below is only needed due to nvidia on Wayland to not crash
    // on exit due to nvidia driver touching the Wayland display from on
    // `exit` hook.
    let _gl_display = self.gl_context.take().unwrap().display();

    // Clear the window.
    self.state = None;
  }

  fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
    if let Some(AppState { gl_surface, window }) = self.state.as_ref() {
      let gl_context = self.gl_context.as_ref().unwrap();
      let app = self.app.as_mut().unwrap();
        
      let now = Instant::now();
      let delta_time = now - self.last_frame;
      self.last_frame = now;

      let dt = delta_time.as_secs_f32();

      app.update(dt);
      
      app.draw();
      window.request_redraw();

      gl_surface.swap_buffers(gl_context).unwrap();
    }
  }
}

fn create_gl_context(window: &Window, gl_config: &Config) -> NotCurrentContext {
  let raw_window_handle = window.window_handle().ok().map(|wh| wh.as_raw());

  // The context creation part.
  let context_attributes = ContextAttributesBuilder::new().build(raw_window_handle);

  // Since glutin by default tries to create OpenGL core context, which may not be
  // present we should try gles.
  let fallback_context_attributes = ContextAttributesBuilder::new()
    .with_context_api(ContextApi::Gles(None))
    .build(raw_window_handle);

  // There are also some old devices that support neither modern OpenGL nor GLES.
  // To support these we can try and create a 2.1 context.
  let legacy_context_attributes = ContextAttributesBuilder::new()
    .with_context_api(ContextApi::OpenGl(Some(Version::new(2, 1))))
    .build(raw_window_handle);

  // Reuse the uncurrented context from a suspended() call if it exists, otherwise
  // this is the first time resumed() is called, where the context still
  // has to be created.
  let gl_display = gl_config.display();

  unsafe {
    gl_display
      .create_context(gl_config, &context_attributes)
      .unwrap_or_else(|_| {
        gl_display
          .create_context(gl_config, &fallback_context_attributes)
          .unwrap_or_else(|_| {
            gl_display
              .create_context(gl_config, &legacy_context_attributes)
              .expect("failed to create context")
          })
      })
  }
}

enum GlDisplayCreationState {
  /// The display was not build yet.
  Builder(Box<DisplayBuilder>),
  /// The display was already created for the application.
  Init,
}

impl MainWindow {
  fn new(template: ConfigTemplateBuilder, display_builder: DisplayBuilder) -> Self {
    Self {
      template,
      gl_display: GlDisplayCreationState::Builder(Box::new(display_builder)),
      gl_context: None,
      state: None,
      app: None,
      last_frame: Instant::now(),
    }
  }
}

struct AppState {
  gl_surface: Surface<WindowSurface>,
  // NOTE: Window should be dropped after all resources created using its
  // raw-window-handle.
  window: Window,
}

// Find the config with the maximum number of samples, so our triangle will be
// smooth.
pub fn gl_config_picker(configs: Box<dyn Iterator<Item = Config> + '_>) -> Config {
  configs
    .reduce(|accum, config| {
      let transparency_check = config.supports_transparency().unwrap_or(false)
        & !accum.supports_transparency().unwrap_or(false);

      if transparency_check || config.num_samples() > accum.num_samples() {
        config
      } else {
        accum
      }
    })
    .unwrap()
}
