use std::rc::Rc;

use crate::{
  project::ProjectAssets,
  runtime::EditorRuntime,
  ui::{
    draw_graph, draw_graph_inspector, draw_timeline,
    graph::{UiGraphCommand, UiGraphState, handle_ui_graph_command},
    timeline::{UiPropertiesCommand, UiTimelineCommand, draw_properties},
  },
};
use dear_imgui_rs::{Condition, DockBuilder, SplitDirection, StyleVar, TextureId, WindowFlags};

pub struct IdGen(usize);

impl IdGen {
  fn new() -> Self {
    Self(1)
  }

  pub fn next(&mut self) -> usize {
    let v = self.0;
    self.0 += 1;
    v
  }
}

pub struct UiState {
  layout_initialized: bool,
  texture_id: TextureId,
  unchanged_str: Rc<str>,
  empty_str: Rc<str>,
  noncontrol_str: Rc<str>,
  pub current_dialog_name: Option<Rc<str>>,
  pub current_container_index: Option<usize>,
  pub node_editor: dear_node_editor::EditorContext,
  pub is_dialog_playing: bool,
  pub graph_state: UiGraphState,
  pub id_gen: IdGen,
}

impl UiState {
  pub fn new(
    assets: &ProjectAssets,
    runtime: &EditorRuntime,
    imgui_context: &dear_imgui_rs::Context,
    imgui_renderer: &mut dear_imgui_glow::GlowRenderer,
  ) -> Self {
    // Adding Renderer Texture to TexturePool
    let texture_id = TextureId::new(1001);
    imgui_renderer
      .texture_map_mut()
      .set(texture_id, runtime.model_renderer.texture_target_id());

    let mut id_gen = IdGen::new();
    let graph_state = UiGraphState::new(&assets.dialog_mgr, &mut id_gen);
    let node_editor = dear_node_editor::EditorContext::create(imgui_context);

    Self {
      texture_id,
      layout_initialized: false,
      is_dialog_playing: false,
      // history: UndoRedo::new(),
      graph_state,
      id_gen,
      node_editor,
      current_dialog_name: None,
      current_container_index: None,
      noncontrol_str: "NonControl".into(),
      unchanged_str: "(Unchanged)".into(),
      empty_str: "".into(),
    }
  }
}

pub enum UiCommand {
  PlayDialog { skip: usize },
  StopDialog,
  Graph(UiGraphCommand),
  Timeline(UiTimelineCommand),
  Properties(UiPropertiesCommand),
}

#[derive(Default)]
pub struct UiOutput {
  pub commands: Vec<UiCommand>,
}

pub fn draw_editor(
  ui: &dear_imgui_rs::Ui,
  assets: &ProjectAssets,
  state: &mut UiState,
) -> UiOutput {
  let mut output = UiOutput::default();

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

      if !state.layout_initialized {
        state.layout_initialized = true;
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
            let o = draw_graph(ui, state);

            output.commands.extend(o.commands);
          });

        // Inspector
        ui.child_window("##inspector")
          .size([avail[0], inspector_height])
          .border(true)
          .build(ui, || draw_graph_inspector(ui, state));
      });

      ui.window("Timeline").build(|| {
        let is_dialog_playing = state.is_dialog_playing;
        let current_container_index = state
          .current_dialog_name
          .as_ref()
          .and_then(|name| state.graph_state.get_node_by_name(name))
          .filter(|node| state.graph_state.get_selected_node_id() == Some(node.id()))
          .and(state.current_container_index);

        match state.graph_state.get_mut_selected_timeline() {
          Some(timeline_state) => {
            let o = draw_timeline(
              ui,
              is_dialog_playing,
              current_container_index,
              timeline_state,
            );

            output.commands.extend(o.commands);
          }
          None => {
            ui.text("Select a Conversation Node");
          }
        }
      });

      ui.window("Properties").build(|| {
        let Some(timeline_state) = state.graph_state.get_mut_selected_timeline() else {
          return;
        };

        if let Some(selected) = timeline_state.get_selected() {
          draw_properties(
            ui,
            selected,
            state.unchanged_str.clone(),
            state.empty_str.clone(),
            state.noncontrol_str.clone(),
            assets,
            timeline_state,
          );
        }
      });
    });

  // Draw Model
  ui.window("Preview").build(|| {
    let available = ui.content_region_avail();

    if available[0] <= 0.0 || available[1] <= 0.0 {
      return;
    }

    let aspect_ratio = assets.layout.width / assets.layout.height;

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

    dear_imgui_rs::Image::new(ui, state.texture_id, draw_size)
      .uv0([0.0, 1.0])
      .uv1([1.0, 0.0])
      .build();
  });

  output
}

pub fn handle_ui_command(cmd: UiCommand, state: &mut UiState) {
  match cmd {
    UiCommand::Graph(graph_cmd) => {
      handle_ui_graph_command(graph_cmd, state);
    }
    _ => {
      unimplemented!("Implement this plz")
    }
  }
}
