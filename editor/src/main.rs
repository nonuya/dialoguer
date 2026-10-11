mod app;
mod project;
mod runtime;
mod ui;

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
use std::path::PathBuf;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes};

fn main() -> anyhow::Result<()> {
  env_logger::init();

  let project_path = std::env::args().nth(1).expect("expected project path");
  let window_attributes =
    Window::default_attributes().with_title(format!("Dialoguer - {project_path}"));
  let display_builder = DisplayBuilder::new().with_window_attributes(Some(window_attributes));
  let event_loop = EventLoop::new()?;
  let template = ConfigTemplateBuilder::new().with_alpha_size(8);

  event_loop.run_app(&mut MainWindow::new(
    template,
    display_builder,
    project_path,
  ))?;

  Ok(())
}

struct MainWindow {
  template: ConfigTemplateBuilder,
  project_path: String,
  app: Option<App>,
  imgui: Option<ImGuiState>,
  state: Option<AppState>,
  gl_context: Option<PossiblyCurrentContext>,
  display: GlDisplayCreationState,
}

struct ImGuiState {
  context: dear_imgui_rs::Context,
  platform: dear_imgui_winit::WinitPlatform,
  renderer: dear_imgui_glow::GlowRenderer,
}

struct AppState {
  surface: Surface<WindowSurface>,
  window: Window,
}

impl ApplicationHandler for MainWindow {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    let (window, config) = match &self.display {
      GlDisplayCreationState::Builder(builder) => {
        let (window, config) =
          match builder
            .clone()
            .build(event_loop, self.template.clone(), gl_config_picker)
          {
            Ok((Some(window), config)) => (window, config),
            Ok((None, _)) => {
              error!("[DisplayBuilder] failed to create a window");
              event_loop.exit();
              return;
            }
            Err(err) => {
              error!("[DisplayBuilder] {err:#}");
              event_loop.exit();
              return;
            }
          };

        self.display = GlDisplayCreationState::Init;
        self.gl_context = Some(create_gl_context(&window, &config).treat_as_possibly_current());
        (window, config)
      }
      GlDisplayCreationState::Init => {
        debug!("Recreating window in `resumed`");
        let config = self.gl_context().config();
        match glutin_winit::finalize_window(event_loop, WindowAttributes::default(), &config) {
          Ok(window) => (window, config),
          Err(err) => {
            error!("[Window] {err:#}");
            event_loop.exit();
            return;
          }
        }
      }
    };

    let attrs = window
      .build_surface_attributes(Default::default())
      .expect("failed to build surface attributes");
    let surface = unsafe {
      config
        .display()
        .create_window_surface(&config, &attrs)
        .expect("failed to create window surface")
    };
    self
      .gl_context()
      .make_current(&surface)
      .expect("failed to make GL context current");

    if self.app.is_none() {
      if let Err(err) = self.initialize_app(&window, event_loop) {
        error!("[Application] {err:#}");
        event_loop.exit();
        return;
      }
    }

    if let Err(err) = surface.set_swap_interval(
      self.gl_context(),
      SwapInterval::Wait(NonZeroU32::new(1).expect("one is non-zero")),
    ) {
      error!("failed to enable vsync: {err:?}");
    }

    self.state = Some(AppState { surface, window });
  }

  fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
    debug!("Window suspended");
    self.state = None;
    self.gl_context = Some(
      self
        .gl_context
        .take()
        .expect("GL context is missing")
        .make_not_current()
        .expect("failed to release GL context")
        .treat_as_possibly_current(),
    );
  }

  fn window_event(
    &mut self,
    event_loop: &ActiveEventLoop,
    _window_id: winit::window::WindowId,
    event: WindowEvent,
  ) {
    if let (Some(imgui), Some(state)) = (self.imgui.as_mut(), self.state.as_ref()) {
      imgui
        .platform
        .handle_window_event(&mut imgui.context, &state.window, &event);
    }

    match event {
      WindowEvent::Resized(size) if size.width > 0 && size.height > 0 => {
        if let Some(state) = self.state.as_ref() {
          state.surface.resize(
            self.gl_context(),
            NonZeroU32::new(size.width).expect("width is non-zero"),
            NonZeroU32::new(size.height).expect("height is non-zero"),
          );
        }
        if let Some(app) = self.app.as_mut() {
          app.resize(size.width, size.height);
        }
      }
      WindowEvent::CloseRequested => event_loop.exit(),
      WindowEvent::KeyboardInput { event, .. } => {
        if let Some(app) = self.app.as_mut() {
          app.keyboard(event);
        }
      }
      _ => {}
    }
  }

  fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
    let _display = self
      .gl_context
      .take()
      .expect("GL context is missing")
      .display();
    self.state = None;
  }

  fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
    let Some(state) = self.state.as_ref() else {
      return;
    };

    let window = &state.window;
    let surface = &state.surface;
    let gl_context = self.gl_context.as_ref().expect("GL context is missing");
    let app = self.app.as_mut().expect("application is missing");
    let imgui = self.imgui.as_mut().expect("ImGui state is missing");

    imgui.platform.prepare_frame(window, &mut imgui.context);
    let ui = imgui.context.frame();
    app.update(ui.io().delta_time());
    app.draw(ui);
    imgui.platform.prepare_render_with_ui(&ui, window);

    let draw_data = imgui.context.render();
    imgui
      .renderer
      .new_frame()
      .expect("failed to start renderer frame");
    imgui
      .renderer
      .render(draw_data)
      .expect("failed to render ImGui frame");

    window.request_redraw();
    surface
      .swap_buffers(gl_context)
      .expect("failed to swap buffers");
  }
}

impl MainWindow {
  fn new(
    template: ConfigTemplateBuilder,
    display_builder: DisplayBuilder,
    project_path: String,
  ) -> Self {
    Self {
      template,
      project_path,
      app: None,
      imgui: None,
      state: None,
      gl_context: None,
      display: GlDisplayCreationState::Builder(Box::new(display_builder)),
    }
  }

  fn gl_context(&self) -> &PossiblyCurrentContext {
    self.gl_context.as_ref().expect("GL context is missing")
  }

  fn initialize_app(
    &mut self,
    window: &Window,
    _event_loop: &ActiveEventLoop,
  ) -> anyhow::Result<()> {
    use dear_imgui_rs::ConfigFlags;

    let mut context = dear_imgui_rs::Context::create();
    context
      .io_mut()
      .set_config_flags(ConfigFlags::DOCKING_ENABLE);
    context.set_ini_filename(None::<String>)?;
    context
      .fonts()
      .add_font(&[dear_imgui_rs::FontSource::TtfData {
        data: include_bytes!("NotoSansJP-Regular.ttf"),
        size_pixels: None,
        config: None,
      }]);

    let mut platform = dear_imgui_winit::WinitPlatform::new(&mut context);
    platform.attach_window(window, dear_imgui_winit::HiDpiMode::Default, &mut context);

    let gl = unsafe {
      glow::Context::from_loader_function_cstr(|name| {
        self.gl_context().display().get_proc_address(name).cast()
      })
    };
    let mut renderer = dear_imgui_glow::GlowRenderer::new(gl, &mut context)?;
    renderer.set_framebuffer_srgb_enabled(false);
    renderer.new_frame()?;

    let gl = renderer
      .gl_context()
      .ok_or_else(|| anyhow::anyhow!("renderer has no GL context"))?
      .clone();
    let app = App::new(
      PathBuf::from(&self.project_path),
      gl,
      &context,
      &mut renderer,
    )?;
    self.app = Some(app);
    self.imgui = Some(ImGuiState {
      context,
      platform,
      renderer,
    });
    Ok(())
  }
}

fn create_gl_context(window: &Window, config: &Config) -> NotCurrentContext {
  let raw_window_handle = window.window_handle().ok().map(|handle| handle.as_raw());
  let gl_display = config.display();

  let attributes = ContextAttributesBuilder::new().build(raw_window_handle);
  let gles_attributes = ContextAttributesBuilder::new()
    .with_context_api(ContextApi::Gles(None))
    .build(raw_window_handle);
  let legacy_attributes = ContextAttributesBuilder::new()
    .with_context_api(ContextApi::OpenGl(Some(Version::new(2, 1))))
    .build(raw_window_handle);

  unsafe {
    gl_display
      .create_context(config, &attributes)
      .or_else(|_| gl_display.create_context(config, &gles_attributes))
      .or_else(|_| gl_display.create_context(config, &legacy_attributes))
      .expect("failed to create an OpenGL context")
  }
}

enum GlDisplayCreationState {
  Builder(Box<DisplayBuilder>),
  Init,
}

fn gl_config_picker(configs: Box<dyn Iterator<Item = Config> + '_>) -> Config {
  configs
    .reduce(|best, config| {
      let transparency = config.supports_transparency().unwrap_or(false)
        && !best.supports_transparency().unwrap_or(false);
      if transparency || config.num_samples() > best.num_samples() {
        config
      } else {
        best
      }
    })
    .expect("no compatible OpenGL configuration found")
}
