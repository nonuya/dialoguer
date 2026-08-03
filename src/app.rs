use std::{path::PathBuf, rc::Rc, str::FromStr};

use dear_imgui_rs::*;
use winit::event::KeyEvent;

use crate::scene::Scene;

// TODO: Add Speed for Animations

pub struct App {
  scene: Option<Scene>,
  layout_initialized: bool,
}

impl App {
  pub fn new() -> anyhow::Result<Self> {
    Ok(Self {
      scene: None,
      layout_initialized: false,
    })
  }

  pub fn update(&mut self, deltatime: f32) {}

  pub fn draw(&mut self, ui: &mut Ui, gl: Rc<glow::Context>) {
    // FIXME: This is only for debug
    /*
    if self.scene.is_none() {
      let path = PathBuf::from("assets/models/iav_013_2");
      let mut scene = Scene::load_from_model_path(gl, path).unwrap();
      let size = ui.io().display_size();
      scene.resize(size[0] as u32, size[1] as u32);
      self.scene = Some(scene);
    }

    self.scene.as_ref().unwrap().draw();
    */

    let viewport = ui.main_viewport();

    // Forzar que la ventana host ocupe TODO el viewport, siempre
    ui.set_next_window_viewport(viewport.id());

    // Quitar cualquier decoración/estilo que permita moverla o darle padding
    let style_vars = [
      ui.push_style_var(StyleVar::WindowRounding(0.0)),
      ui.push_style_var(StyleVar::WindowBorderSize(0.0)),
      ui.push_style_var(StyleVar::WindowPadding([0.0, 0.0])),
    ];

    let window_flags = WindowFlags::NO_TITLE_BAR
      | WindowFlags::NO_COLLAPSE
      | WindowFlags::NO_RESIZE
      | WindowFlags::NO_MOVE
      | WindowFlags::NO_BRING_TO_FRONT_ON_FOCUS
      | WindowFlags::NO_NAV_FOCUS
      | WindowFlags::NO_DOCKING
      | WindowFlags::NO_BACKGROUND; // opcional, si no quieres que pinte fondo propio

    ui.window("MainDockspaceHost")
      .position(viewport.pos(), Condition::Always)
      .size(viewport.size(), Condition::Always)
      .flags(window_flags)
      .build(|| {
        for style_var in style_vars {
          style_var.pop();
        }

        let dockspace_id = ui.get_id("MainDockSpace");
        ui.dock_space(dockspace_id, [0.0, 0.0]);

        if !self.layout_initialized {
          self.layout_initialized = true;
          DockBuilder::remove_node(ui, dockspace_id);
          DockBuilder::add_node(ui, dockspace_id, DockNodeFlags::PASSTHRU_CENTRAL_NODE);
          DockBuilder::set_node_size(ui, dockspace_id, viewport.size());

          let (dock_top, dock_bottom) =
            DockBuilder::split_node(ui, dockspace_id, SplitDirection::Up, 0.65);
          let (dock_preview, dock_parameters) =
            DockBuilder::split_node(ui, dock_top, SplitDirection::Left, 0.65);
          let (dock_timeline, dock_inspector) =
            DockBuilder::split_node(ui, dock_bottom, SplitDirection::Left, 0.50);

          DockBuilder::dock_window(ui, "Preview", dock_preview);
          DockBuilder::dock_window(ui, "Parameters", dock_parameters);
          DockBuilder::dock_window(ui, "Timeline", dock_timeline);
          DockBuilder::dock_window(ui, "Inspector", dock_inspector);

          DockBuilder::finish(ui, dockspace_id);
        }
      });
    ui.window("Parameters").build(|| {
      ui.text("Parameters panel");
      if ui.button("Add parameter") {
        println!("Add parameter");
      }
    });
    ui.window("Inspector").build(|| {
      ui.text("Inspector panel");
      ui.text("Selected object:");
      ui.text("None");
    });
    ui.window("Preview").build(|| {
      ui.text("Game preview");
      let available = ui.content_region_avail();
      ui.text(format!("Size: {:.0} x {:.0}", available[0], available[1]));
    });
    ui.window("Timeline").build(|| {
      ui.text("Timeline");
      ui.separator();
      for i in 0..10 {
        ui.text(format!("Track {}", i));
      }
    });
  }

  pub fn resize(&mut self, width: u32, height: u32) {
    if let Some(scene) = self.scene.as_mut() {
      scene.resize(width, height);
    }
  }

  pub fn keyboard(&mut self, event: KeyEvent) {}
}
