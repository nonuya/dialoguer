use core::dialog::{DialogManager, DialogPlayer};

use dear_imgui_rs::{Condition, DockBuilder, SplitDirection, StyleVar, TextureId, WindowFlags};

use crate::{project::ProjectAssets, runtime::EditorRuntime};

pub struct UiState {
  layout_initialized: bool,
  texture_id: TextureId,
}

impl UiState {
  pub fn new(
    runtime: &EditorRuntime,
    imgui_context: &dear_imgui_rs::Context,
    imgui_renderer: &mut dear_imgui_glow::GlowRenderer,
  ) -> Self {
    // Adding Model Texture to TexturePool
    let texture_id = TextureId::new(1001);
    imgui_renderer
      .texture_map_mut()
      .set(texture_id, runtime.model.texture);

    Self {
      texture_id,
      layout_initialized: false,
    }
  }
}

pub fn draw_editor(ui: &dear_imgui_rs::Ui, assets: &ProjectAssets, ui_state: &mut UiState) {
  let viewport = ui.main_viewport();
  ui.set_next_window_viewport(viewport.id());
  let style_vars = [
    ui.push_style_var(StyleVar::WindowRounding(0.0)),
    ui.push_style_var(StyleVar::WindowBorderSize(0.0)),
    ui.push_style_var(StyleVar::WindowPadding([0.0, 0.0])),
  ];

  let window_flags = WindowFlags::NO_COLLAPSE
    | WindowFlags::NO_RESIZE
    | WindowFlags::NO_MOVE
    | WindowFlags::NO_BRING_TO_FRONT_ON_FOCUS
    | WindowFlags::NO_NAV_FOCUS
    | WindowFlags::NO_DOCKING
    | WindowFlags::NO_BACKGROUND;

  ui.window("Dialog Editor")
    .position(viewport.pos(), Condition::Always)
    .size(viewport.size(), Condition::Always)
    .flags(window_flags)
    .build(|| {
      for style_var in style_vars {
        style_var.pop();
      }

      let dockspace_id = ui.get_id("MainDockSpace");
      ui.dock_space(dockspace_id, [0.0, 0.0]);

      if !ui_state.layout_initialized {
        ui_state.layout_initialized = true;
        DockBuilder::remove_node(ui, dockspace_id);
        DockBuilder::add_node(
          ui,
          dockspace_id,
          dear_imgui_rs::DockNodeFlags::PASSTHRU_CENTRAL_NODE,
        );
        DockBuilder::set_node_size(ui, dockspace_id, viewport.size());

        let (dock_preview, dock_bottom) =
          DockBuilder::split_node(ui, dockspace_id, SplitDirection::Up, 0.65);

        let (dock_timeline, dock_properties) =
          DockBuilder::split_node(ui, dock_bottom, SplitDirection::Left, 0.70);

        let (dock_preview_left, dock_graph) =
          DockBuilder::split_node(ui, dock_preview, SplitDirection::Left, 0.60);

        DockBuilder::dock_window(ui, "Preview", dock_preview_left);
        DockBuilder::dock_window(ui, "Graph", dock_graph);

        DockBuilder::dock_window(ui, "Timeline", dock_timeline);
        DockBuilder::dock_window(ui, "Properties", dock_properties);

        DockBuilder::finish(ui, dockspace_id);
      }

      ui.window("Graph").build(|| {
        // use crate::graph::Node;

        let avail = ui.content_region_avail();

        let inspector_height = 100.0;
        let graph_height = avail[1] - inspector_height - ui.clone_style().item_spacing()[1];

        ui.child_window("##graph")
          .size([avail[0], graph_height])
          .border(true)
          .build(ui, || {
            /*
            let result = self.graph.build(ui, &self.node_editor, false, None);

            if let Some(cmd) = result.playback_cmd {
              use crate::ui::graph;
              match cmd {
                graph::PlaybackCmd::Play {
                  dialog_mgr,
                  dialog_player,
                } => {
                  playback_cmd = Some(PlaybackCmd::Play {
                    dialog_mgr,
                    dialog_player,
                  });
                }
                graph::PlaybackCmd::Stop => {
                  playback_cmd = Some(PlaybackCmd::Stop);
                }
              }
            }*/
          });

        // Inspector
        ui.child_window("##inspector")
          .size([avail[0], inspector_height])
          .border(true)
          .build(ui, || {
            /*
            ui.text("Inspector");
            ui.separator();

            if let Some(node) = self.graph.get_mut_selected_node() {
              let name = match node {
                Node::Conversation { name, .. } => name,
                Node::Choicer { name, .. } => name,
              };

              ui.text("Name:");
              ui.same_line();
              ui.input_text("##graph_inspector_input", name).build();
            }*/
          });
      });

      ui.window("Timeline").build(|| {
        /*
        if !self
          .graph
          .get_selected_node_id()
          .is_some_and(|id| self.graph.has_node_a_timeline(id))
        {
          ui.text("Select a Conversation Node");
          return;
        }

        let result = self.graph.get_mut_selected_timeline().unwrap().build(
          ui,
          self.copied_block.as_ref(),
          self.copied_container.as_ref(),
          None,
          false,
        );

        if let Some(cmd) = result.playback_cmd {
          use crate::ui::timeline;

          match cmd {
            timeline::PlaybackCmd::Play { skip } => {
              let node = self.graph.get_selected_node().unwrap();
              let dialog = self
                .graph
                .get_selected_timeline()
                .unwrap()
                .export_to_dialog();
              let dialog_mgr =
                DialogManager::from_entries(vec![(node.name().as_str().into(), dialog)]);
              let dialog_player = {
                let entry_point = dialog_mgr.build(node.name().as_str().into()).unwrap();
                DialogPlayer::new(entry_point)
              };

              playback_cmd = Some(PlaybackCmd::Play {
                dialog_mgr,
                dialog_player,
              });
            }
            timeline::PlaybackCmd::Stop => {
              playback_cmd = Some(PlaybackCmd::Stop);
            }
          }
        }
        */
      });

      ui.window("Properties").build(|| {
        // self.draw_properties(ui, &ctx);
      });
    });

  // Draw Model
  ui.window("Preview").build(|| {
    /*let available = ui.content_region_avail();

    if available[0] <= 0.0 || available[1] <= 0.0 {
      return;
    }

    let aspect_ratio = core.layout.width / core.layout.height;

    let draw_size = if available[0] / available[1] > aspect_ratio {
      [available[1] * aspect_ratio, available[1]]
    } else {
      [available[0], available[0] / aspect_ratio]
    };

    let cursor = ui.cursor_pos();

    ui.set_cursor_pos([
      cursor[0] + (available[0] - draw_size[0]) * 0.5,
      cursor[1] + (available[1] - draw_size[1]) * 0.5,
    ]);

    dear_imgui_rs::Image::new(ui, self.texture_id, draw_size)
      .uv0([0.0, 1.0])
      .uv1([1.0, 0.0])
      .build();*/
  });
}

/*
pub struct EditorUI {
  graph: Graph,
  node_editor: dear_node_editor::EditorContext,
  copied_block: Option<TimelineBlock>,
  copied_container: Option<Container>,
}

pub enum PlaybackCmd {
  Play {
    dialog_mgr: DialogManager,
    dialog_player: DialogPlayer,
  },
  Stop,
}

pub struct BuildResult {
  pub playback_cmd: Option<PlaybackCmd>,
}

impl EditorUI {
  pub fn new(
    core: &EditorCore,
  ) -> Self {
    let graph = Graph::new(&core.dialog_mgr);
    let node_editor = dear_node_editor::EditorContext::create(imgui_context);

    Self {
      texture_id,
      graph,
      node_editor,
      copied_block: None,
      copied_container: None,
    }
  }

  pub fn build(&mut self, ui: &dear_imgui_rs::Ui, core: &EditorCore) -> BuildResult {
    let mut playback_cmd = None;



    BuildResult { playback_cmd }
  }
}*/
