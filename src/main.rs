use std::{fs::File, ops::Index, path::PathBuf};

use glium::{
  Display, Frame, IndexBuffer, Program, Surface, VertexBuffer,
  backend::glutin::SimpleWindowBuilder,
  glutin::surface::WindowSurface,
  implement_uniform_block, implement_vertex,
  winit::{application::ApplicationHandler, window::Window},
};

use log::debug;

static VERTEX_SHADER_SRC: &str = r#"
    #version 330 core

    in vec2 position;

    uniform mat4 transform;

    void main() {
        gl_Position = transform * vec4(position, 0.0, 1.0);
    }
"#;

static FRAGMENT_SHADER_SRC: &str = r#"
    #version 330 core

    out vec4 color;

    void main() {
        color = vec4(1.0, 1.0, 1.0, 1.0);
    }
"#;

struct App {
  window: Option<Window>,
  display: Option<Display<WindowSurface>>,
  model: cubism::model::UserModel,
  vertex_buffers: Vec<VertexBuffer<Vertex>>,
  indices_buffers: Vec<IndexBuffer<u16>>,
  program: Option<Program>,
}

impl App {
  fn new(model: cubism::model::UserModel) -> Self {
    App {
      window: None,
      program: None,
      display: None,
      model,
      vertex_buffers: Vec::new(),
      indices_buffers: Vec::new(),
    }
  }
}

impl ApplicationHandler for App {
  fn resumed(&mut self, event_loop: &glium::winit::event_loop::ActiveEventLoop) {
    let (window, display) = SimpleWindowBuilder::new()
      .with_title("IVAV")
      .with_inner_size(400, 400)
      .build(event_loop);

    self.display = Some(display);
    self.window = Some(window);

    self.init();
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

          self.draw(&mut frame);

          frame.finish().unwrap();
        }
      }
      WindowEvent::CloseRequested => event_loop.exit(),
      _ => {}
    }
  }
}

impl App {
  fn init(&mut self) {
    let display = self.display.as_ref().unwrap();

    self.program =
      Some(Program::from_source(display, VERTEX_SHADER_SRC, FRAGMENT_SHADER_SRC, None).unwrap());

    for cubism::core::Drawable {
      vertex_positions,
      indices,
      index,
      ..
    } in self.model.drawables()
    {
      assert_eq!(index, self.vertex_buffers.len());

      let buffer: Vec<_> = vertex_positions
        .iter()
        .map(|&p| Vertex { position: p })
        .collect();

      let vb = glium::VertexBuffer::dynamic(display, &buffer).unwrap();
      let vi = glium::IndexBuffer::new(
        display,
        glium::index::PrimitiveType::TrianglesList,
        &indices,
      )
      .unwrap();
      self.vertex_buffers.push(vb);
      self.indices_buffers.push(vi);
    }
  }

  fn draw(&self, frame: &mut Frame) {
    use cubism::core::DynamicFlags;

    frame.clear_color(0.0, 0.0, 0.0, 1.0);

    let mut drawables: Vec<_> = self.model.drawables().collect();
    drawables.sort_unstable_by_key(|d| d.render_order);

    for drawable in &drawables {
      let dflags = drawable.dynamic_flags;
      if drawable.opacity <= 0.0 || !dflags.intersects(DynamicFlags::IS_VISIBLE) {
        continue;
      }

      let vb = &self.vertex_buffers[drawable.index];
      let ib = &self.indices_buffers[drawable.index];

      if dflags.intersects(DynamicFlags::VERTEX_POSITIONS_CHANGED) {
        let vtx_buffer: Vec<_> = drawable
          .vertex_positions
          .iter()
          .map(|&p| Vertex { position: p })
          .collect();
        vb.write(&vtx_buffer);
      }

      let transform = [
        [1.0f32, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
      ];

      let params = glium::DrawParameters {
        blend: glium::Blend::alpha_blending(),
        polygon_mode: glium::draw_parameters::PolygonMode::Line,
        backface_culling: glium::draw_parameters::BackfaceCullingMode::CullingDisabled,
        ..Default::default()
      };

      let uniforms = glium::uniform! {
          transform: transform,
      };

      frame
        .draw(vb, ib, self.program.as_ref().unwrap(), &uniforms, &params)
        .unwrap();
    }
  }
}

#[derive(Copy, Clone, Debug)]
struct Vertex {
  position: [f32; 2],
}
implement_vertex!(Vertex, position);
implement_uniform_block!(Vertex, position);

fn main() -> anyhow::Result<()> {
  env_logger::init();

  let model_path = PathBuf::from("assets/models/iav_013_2/");
  let model_file = File::open(model_path.join("iav_013_2.model3.json"))?;
  let model_json = cubism::json::model::Model3::from_reader(model_file)?;
  let model = cubism::model::UserModel::from_model3(&model_path, &model_json)?;

  let mut app = App::new(model);
  let event_loop = glium::winit::event_loop::EventLoop::builder().build()?;
  event_loop.run_app(&mut app)?;

  Ok(())
}
