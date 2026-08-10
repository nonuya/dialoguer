use dear_imgui_rs::*;
use log::{debug, info, warn};
use std::{path::Path, rc::Rc};

use crate::{graph, timeline};

pub struct EditorContext<'a> {
  pub model: &'a mut core::live2d::Model,
  pub animator: &'a mut core::live2d::animator::Animator,
  pub enummap: &'a mut core::live2d::animator::EnumMap,
  pub dialog_mgr: &'a mut core::dialog::DialogManager,
  pub dialog_player: &'a mut Option<core::dialog::DialogPlayer>,
  pub motion_mgr: &'a core::live2d::animator::MotionManager,
  pub enummap_path: &'a Path,
  pub dialog_path: &'a Path,
}

pub struct Layout {
  layout_initialized: bool,
  new_modal_string: String,
  open_modal: bool,
  node_editor: dear_node_editor::EditorContext,
  graph: graph::Graph,
  is_main_layout: bool,
  selected_enum: Option<(Rc<str>, Rc<str>)>,
  enumlog: Vec<(Rc<str>, Rc<str>)>, // FIXME: Delete this
  copied_block: Option<timeline::TimelineBlock>,
  copied_container: Option<timeline::Container>,
  focused_parameters: Vec<Rc<str>>, // Small list
  non_control: Rc<str>,
  unchanged: Rc<str>,
  empty: Rc<str>,
  enum_parameter_state: core::live2d::animator::ParameterState,
}

impl Layout {
  pub fn new(
    imgui_context: &dear_imgui_rs::Context,
    dialog_mgr: &core::dialog::DialogManager,
  ) -> Self {
    Self {
      layout_initialized: false,
      node_editor: dear_node_editor::EditorContext::create(imgui_context),
      selected_enum: None,
      enumlog: Vec::new(),
      is_main_layout: true,
      open_modal: false,
      graph: graph::Graph::new(dialog_mgr),
      new_modal_string: String::new(),
      copied_block: None,
      copied_container: None,
      focused_parameters: Vec::new(),
      enum_parameter_state: core::live2d::animator::ParameterState::new(),
      non_control: "NonControl".into(),
      unchanged: "(Unchanged)".into(),
      empty: "".into(),
    }
  }

  pub fn is_main_layout(&self) -> bool {
    self.is_main_layout
  }

  pub fn mut_enum_parameter_state(&mut self) -> &mut core::live2d::animator::ParameterState {
    &mut self.enum_parameter_state
  }

  pub fn draw(&mut self, ui: &mut Ui, mut ctx: EditorContext) {
    if ui.io().key_alt() && ui.is_key_pressed(Key::Key1) {
      self.is_main_layout = true;
      self.layout_initialized = false;
    }

    if ui.io().key_alt() && ui.is_key_pressed(Key::Key2) {
      self.is_main_layout = false;
      self.layout_initialized = false;
    }

    if ui.io().key_alt() && ui.is_key_pressed(Key::R) {
      self.reload_enummap(&mut ctx);
    }

    if ui.io().key_ctrl() && ui.is_key_pressed(Key::S) {
      self.save_dialog(&ctx);
    }

    if let Some(_) = ui.begin_main_menu_bar() {
      ui.menu("File", || {
        if ui.menu_item("(R)eset Enum") {
          self.reload_enummap(&mut ctx);
        }
        if ui.menu_item("(S)ave") {
          self.save_dialog(&ctx);
        }
      });

      ui.menu("View", || {
        if ui.menu_item("(1) Dialog") {
          self.is_main_layout = true;
          self.layout_initialized = false;
        }

        if ui.menu_item("(2) Enum") {
          self.is_main_layout = false;
          self.layout_initialized = false;
        }
      });
    }

    if self.is_main_layout {
      self.draw_dialogue_layout(ui, ctx);
    } else {
      self.draw_enum_layout(ui, ctx);
    }
  }

  fn reload_enummap(&self, ctx: &mut EditorContext) {
    debug!("Reloading EnumMap in {}", ctx.enummap_path.display());
    match core::live2d::animator::load_enum_map(ctx.enummap_path) {
      Ok(enummap) => {
        *ctx.enummap = enummap;
        debug!("Reloaded EnumMap!!!");
      },
      Err(e) => {
        warn!("Failed to Reloading EnumMap: {e}");
      }
    }
  }

  fn save_dialog(&self, ctx: &EditorContext) {
    debug!("Exporting Dialog to {}", ctx.dialog_path.display());
    match self.graph.export_to_path(ctx.dialog_path) {
      Ok(()) => debug!("Exported Dialog!!!!!!!!"),
      Err(e) => warn!("Failed to export dialog: {e}")
    }
  }

  fn draw_enum_layout(&mut self, ui: &mut Ui, ctx: EditorContext) {
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

          let (dock_enums, dock_right) =
            DockBuilder::split_node(ui, dock_bottom, SplitDirection::Left, 0.50);

          let (dock_inspector, dock_enumlog) =
            DockBuilder::split_node(ui, dock_right, SplitDirection::Up, 0.70);

          DockBuilder::dock_window(ui, "Preview", dock_preview);
          DockBuilder::dock_window(ui, "Parameters", dock_parameters);
          DockBuilder::dock_window(ui, "Enums", dock_enums);
          DockBuilder::dock_window(ui, "Inspector", dock_inspector);
          DockBuilder::dock_window(ui, "Enum Log", dock_enumlog);

          DockBuilder::finish(ui, dockspace_id);
        }
      });

    ui.window("Parameters").build(|| {
      if !self.focused_parameters.is_empty() {
        if ui.button("Clear Filter") {
          self.focused_parameters.clear();
        }
      }

      let mut parameters: Vec<_> = ctx
        .model
        .get_parameters_iter_mut()
        .filter(|a| {
          self.focused_parameters.is_empty()
            || self.focused_parameters.iter().any(|b| a.id == b.as_ref())
        })
        .collect();
      parameters.sort_unstable_by(|a, b| natord::compare(a.id, b.id));
      let changed = parameters.into_iter().fold(false, |acc, e| {
        acc
          | Self::draw_float_property(ui, e.id, e.value, e.min_value, e.max_value, e.default_value)
      });

      if changed {
        ctx.model.update_parameters();
      }
    });

    if let Some((enum_name, value_name)) = &self.selected_enum {
      ui.window("Inspector").build(|| {
        use core::live2d::animator::Value;

        ui.text(format!("{}/{}", enum_name, value_name));
        ui.separator_horizontal();

        if let Some(r#enum) = ctx.enummap.enums.get_mut(enum_name)
          && let Some(params) = r#enum.values.get_mut(value_name)
        {
          // =================
          // Toolbar
          // =================
          if ui.button("Preview") {
            self.enumlog.push((enum_name.clone(), value_name.clone()));
            self.enum_parameter_state.set_parameter_change(
              core::live2d::animator::ParameterChange::from_params(
                enum_name.clone(),
                value_name.clone(),
                params,
                |lhs, rhs| {
                  ctx.animator.get_parameter_state().is_enum_active(lhs, rhs)
                    || self.enum_parameter_state.is_enum_active(lhs, rhs)
                },
              ),
            );
          }

          if ui.button("Clear") {
            ctx.model.load_saved_parameters();
            self.enumlog.clear();
            self.enum_parameter_state.reset();
          }

          if ui.button("Focus") {
            self.focused_parameters = params.iter().map(|p| p.name.clone()).collect();
          }

          ui.separator();

          for p in params {
            let _scope = ui.push_id(p.name.as_ref());

            ui.separator();
            ui.text(&p.name);

            match &mut p.value {
              Value::Fixed(value) => {
                ui.input_float("##value", value);
              }

              Value::Smooth {
                actual: _,
                target,
                step,
              } => {
                ui.input_float("Target", target);
                ui.input_float("Step", step);
              }
            }

            if let Some(modification) = &p.modification {
              ui.separator();
              ui.text("Modification");

              ui.bullet_text(format!("If {} == {}", modification.lhs, modification.rhs));
              ui.bullet_text(format!("Then = {:.2}", modification.then));
            }
          }
        }
      });
    }

    ui.window("Enums").build(|| {
      let mut enums: Vec<_> = ctx.enummap.enums.iter().collect();
      enums.sort_unstable_by_key(|v| v.0);

      for (enum_name, enum_data) in enums {
        if let Some(_) = ui
          .tree_node_config(enum_name)
          .framed(true)
          .span_avail_width(true)
          .push()
        {
          for (value_name, _) in &enum_data.values {
            let selected = self
              .selected_enum
              .as_ref()
              .is_some_and(|(name, value)| enum_name == name && value_name == value);

            if ui.selectable_config(value_name).selected(selected).build() {
              self.selected_enum = Some((enum_name.clone(), value_name.clone()));
            }
          }
        }
      }
    });
    ui.window("Enum Log").build(|| {
      for l in &self.enumlog {
        ui.text(format!("{}/{}", l.0, l.1));
      }
    });
  }

  fn draw_dialogue_layout(&mut self, ui: &mut Ui, mut ctx: EditorContext) {
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
          use crate::graph::Node;

          let avail = ui.content_region_avail();

          let inspector_height = 100.0;
          let graph_height = avail[1] - inspector_height - ui.clone_style().item_spacing()[1];

          ui.child_window("##graph")
            .size([avail[0], graph_height])
            .border(true)
            .build(ui, || {
              if ui.is_window_focused() {
                if ui.io().key_ctrl() && ui.is_key_pressed(Key::Z) {
                  self.graph.undo();
                }
                if ui.io().key_ctrl() && ui.is_key_pressed(Key::Y) {
                  self.graph.redo();
                }
              }

              if let Some(node) = self.graph.get_selected_node() {
                if ui.button("Play") {
                  match self.graph.export_to_dialog(node.id()) {
                    Ok(dialogs) => {
                      let name = self.graph.get_node_by_id(node.id()).unwrap().name();
                      Self::play(&mut ctx, Some((name.clone(), dialogs)));
                    }
                    Err(e) => warn!("[Graph] Failed to Play Node: {e}"),
                  }
                }

                ui.same_line();
                if ctx.dialog_player.as_ref().is_some_and(|p| p.is_playing()) {
                  ui.separator_vertical();
                  ui.same_line();
                  if ui.button("Stop") {
                    Self::play(&mut ctx, None);
                  }
                }

                ui.same_line();
                ui.separator_vertical();
                ui.same_line();
              }

              self.graph.draw(
                ui,
                &self.node_editor,
                ctx
                  .dialog_player
                  .as_ref()
                  .and_then(|p| p.current_dialog_name()),
              );
            });

          // Inspector
          ui.child_window("##inspector")
            .size([avail[0], inspector_height])
            .border(true)
            .build(ui, || {
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
              }
            });
        });

        ui.window("Timeline").build(|| {
          self.draw_timeline(ui, &mut ctx);
        });

        ui.window("Properties").build(|| {
          self.draw_properties(ui, &ctx);
        });
      });
  }

  fn draw_properties(&mut self, ui: &Ui, ctx: &EditorContext) {
    if let Some(selected) = self
      .graph
      .get_mut_selected_timeline()
      .and_then(|t| t.get_selected())
    {
      use timeline::{Selection, TimelineBlockType, TimelineSetBlockType};

      match selected {
        Selection::Container { container, .. } => {
          ui.text("Who?");
          ui.same_line();
          ui.input_text(format!("##container_{}", container.id), &mut container.name)
            .build();
        }
        Selection::Block {
          block,
          container_id,
          ..
        } => match &mut block.value {
          TimelineBlockType::Text(text) => {
            ui.text("Text:");
            ui.same_line();
            ui.set_next_item_width(-1.0);
            ui.input_text(format!("##{}_{}_text", container_id, block.id), text)
              .build();
          }
          TimelineBlockType::Wait(seconds) => {
            ui.text("Wait:");
            ui.same_line();

            ui.input_float_config("##wait_seconds")
              .step(0.10)
              .build(seconds);

            ui.same_line();
            ui.text("seconds");
          }
          TimelineBlockType::Set(TimelineSetBlockType {
            parameters,
            anim,
            view,
          }) => {
            let mut remove_index: Option<usize> = None;
            let mut move_index: Option<(usize, isize)> = None; // (idx, offset: -1 sube, +1 baja)

            // ANIMATION
            ui.text("Animation");
            ui.separator();
            ui.checkbox("Loop?", &mut anim.1);
            ui.same_line();

            Self::combo_box(
              ui,
              &mut anim.0,
              "##anim_combo",
              ctx.motion_mgr.names(),
              &self.unchanged,
              self.empty.clone(),
            );

            ui.text("View");
            ui.separator();

            // VIEW
            Self::combo_box(
              ui,
              view,
              "##view_combo",
              ctx.enummap.views.keys(),
              &self.unchanged,
              self.empty.clone(),
            );

            ui.spacing();
            ui.text("Parameters");
            ui.separator();
            for (idx, entry) in parameters.iter_mut().enumerate() {
              let _id = ui.push_id(idx as i32);

              if ui.button("X") {
                remove_index = Some(idx);
              }

              ui.same_line();

              if ui.button("^") {
                move_index = Some((idx, -1));
              }
              ui.same_line();

              if ui.button("v") {
                move_index = Some((idx, 1));
              }
              ui.same_line();

              ui.text(&entry.0);
              ui.same_line();
              ui.set_next_item_width(-10.0);
              match ctx.enummap.enums.get(&entry.0) {
                Some(enumtype) => {
                  if let Some(_) = ui.begin_combo(format!("##{}_{}", &entry.0, &entry.1), &entry.1)
                  {
                    for value in enumtype
                      .values
                      .iter()
                      .map(|v| v.0)
                      .chain(std::iter::once(&self.non_control))
                    {
                      let selected = entry.1 == *value;
                      if ui.selectable_config(value).selected(selected).build() {
                        entry.1 = value.clone();
                      }
                    }
                  }
                }
                None => ui.text("Unknown Enum"),
              }
            }

            if let Some((idx, offset)) = move_index {
              let target = idx as isize + offset;
              if target >= 0 && (target as usize) < parameters.len() {
                parameters.swap(idx, target as usize);
              }
            }
            if let Some(idx) = remove_index {
              parameters.remove(idx);
            }

            ui.spacing();
            // --- Botón "+" al final: abre un combobox para elegir qué agregar ---
            if ui.button_with_size("+", [ui.content_region_avail()[0], 0.0]) {
              ui.open_popup("##add_entry_popup");
            }
            if let Some(_) = ui.begin_popup("##add_entry_popup") {
              let mut params: Vec<_> = ctx.enummap.enums.iter().collect();
              params.sort_unstable_by_key(|&p| p.0);
              for (prop_name, prop_value) in params {
                if ui.selectable_config(prop_name).size([0.0, 0.0]).build() {
                  parameters.push((
                    prop_name.clone(),
                    prop_value.values.iter().next().unwrap().0.clone(),
                  ));
                  ui.close_current_popup();
                }
              }
            }
          }
          TimelineBlockType::Next => {}
        },
      }
    }
  }

  fn draw_timeline(&mut self, ui: &Ui, ctx: &mut EditorContext) {
    if !self
      .graph
      .get_selected_node_id()
      .is_some_and(|id| self.graph.has_node_a_timeline(id))
    {
      ui.text("Select a Conversation Node");
      return;
    }

    let is_dialog_playing = ctx
      .dialog_player
      .as_ref()
      .is_some_and(|player| player.is_playing());

    if is_dialog_playing {
      if ui.button("Stop Conversation") {
        Self::play(ctx, None);
      }
    } else {
      if ui.button("Play Conversation") {
        let node = self.graph.get_selected_node().unwrap();
        let timeline = self.graph.get_selected_timeline().unwrap();
        let dialog = timeline.export_to_dialog();
        Self::play(
          ctx,
          Some((
            node.name().as_str().into(),
            vec![(node.name().as_str().into(), dialog)],
          )),
        );
      }
    }

    ui.same_line();

    if ui.button("(A)dd Container") || (ui.io().key_alt() && ui.is_key_pressed(Key::A)) {
      ui.open_popup("Add Container");
    }

    if Self::modal(
      ui,
      "Add Container",
      &mut self.new_modal_string,
      &mut self.open_modal,
    ) {
      let timeline = self.graph.get_mut_selected_timeline().unwrap();
      timeline.push_back_container(self.new_modal_string.clone());
      self.new_modal_string.clear();
    }

    enum DeleteCommand {
      Container(usize),
      Block(usize, usize),
    }

    let mut pending_delete = None;
    let mut pending_paste_block = None;
    let mut pending_paste_container = None;
    let mut pending_play = None;

    if let Some(selected) = self
      .graph
      .get_mut_selected_timeline()
      .unwrap()
      .get_selected()
    {
      ui.same_line();
      ui.separator_vertical();
      ui.same_line();
      use timeline::{Selection, TimelineBlockType, TimelineSetBlockType};

      // Selected
      if ui.button("Play Container") {
        match selected {
          Selection::Block {
            container_index, ..
          } => {
            pending_play = Some(container_index);
          }
          Selection::Container { index, .. } => {
            pending_play = Some(index);
          }
        }
      }

      ui.same_line();

      let is_window_focused = ui.is_window_focused_with_flags(FocusedFlags::ROOT_AND_CHILD_WINDOWS);

      if ui.button("(D)elete")
        || (is_window_focused && ui.io().key_alt() && ui.is_key_pressed(Key::D))
      {
        match &selected {
          Selection::Block {
            container_id,
            block,
            ..
          } => {
            pending_delete = Some(DeleteCommand::Block(*container_id, block.id));
          }
          Selection::Container { container, .. } => {
            pending_delete = Some(DeleteCommand::Container(container.id));
          }
        }
      }

      match selected {
        Selection::Block {
          block,
          container_index,
          ..
        } => {
          if is_window_focused {
            if ui.io().key_ctrl() && ui.is_key_pressed(Key::C) {
              debug!(
                "Copied Block {} from Container {}",
                block.id, container_index
              );
              self.copied_block = Some(block.clone());
            }

            if ui.io().key_ctrl() && ui.is_key_pressed(Key::V) {
              if let Some(copied) = self.copied_block.as_ref() {
                pending_paste_block = Some((container_index, block.id, copied));
              }
            }
          }
        }
        Selection::Container { container, index } => {
          ui.same_line();

          ui.separator_vertical();

          ui.same_line();

          ui.text("Blocks:");

          ui.same_line();
          if ui.button("(T)ext") || (ui.io().key_alt() && ui.is_key_pressed(Key::T)) {
            container.add_block(TimelineBlockType::Text(String::new()));
          }

          ui.same_line();
          if ui.button("(W)ait") || (ui.io().key_alt() && ui.is_key_pressed(Key::W)) {
            container.add_block(TimelineBlockType::Wait(0.0));
          }

          ui.same_line();

          if ui.button("(S)et") || (ui.io().key_alt() && ui.is_key_pressed(Key::S)) {
            container.add_block(TimelineBlockType::Set(TimelineSetBlockType::default()));
          }

          ui.same_line();

          if ui.button("(N)ext") || (ui.io().key_alt() && ui.is_key_pressed(Key::N)) {
            container.add_block(TimelineBlockType::Next);
          }

          if is_window_focused {
            if ui.io().key_ctrl() && ui.is_key_pressed(Key::C) {
              debug!("Copied Container {}", index);
              self.copied_container = Some(container.clone());
            }

            if ui.io().key_ctrl() && ui.is_key_pressed(Key::V) {
              if let Some(copied) = self.copied_container.as_ref() {
                pending_paste_container = Some((index, copied));
              }
            }
          }
        }
      }
    }

    if let Some(cmd) = pending_delete.take() {
      let timeline = self.graph.get_mut_selected_timeline().unwrap();
      match cmd {
        DeleteCommand::Container(id) => timeline.delete_container(id),
        DeleteCommand::Block(container_id, block_id) => {
          timeline.delete_block(container_id, block_id)
        }
      }
    }

    if let Some(cmd) = pending_paste_container.take() {
      debug!("Pasting Container in {}", cmd.0);
      let timeline = self.graph.get_mut_selected_timeline().unwrap();
      timeline.insert_container(cmd.0, cmd.1.clone());
    }

    if let Some(cmd) = pending_paste_block.take() {
      debug!("Pasting Block in Container {}", cmd.0);
      let timeline = self.graph.get_mut_selected_timeline().unwrap();
      let container = timeline.get_mut_container_by_index(cmd.0).unwrap();
      container.insert_block(cmd.1, cmd.2.clone());
    }

    if let Some(idx) = pending_play.take() {
      info!("Skipping to Conversation with Index {idx}...");
      let node = self.graph.get_selected_node().unwrap();
      let timeline = self.graph.get_selected_timeline().unwrap();
      let dialog = timeline.export_to_dialog();
      Self::play(
        ctx,
        Some((
          node.name().as_str().into(),
          vec![(node.name().as_str().into(), dialog)],
        )),
      );
      ctx.dialog_player.as_mut().unwrap().skip(
        idx,
        ctx.animator,
        ctx.dialog_mgr,
        ctx.enummap,
        ctx.motion_mgr,
      );
    }

    ui.separator();

    ui.child_window("##timeline_canvas")
      .size([0.0, 0.0])
      .flags(WindowFlags::HORIZONTAL_SCROLLBAR)
      .build(ui, || {
        let idx = ctx.dialog_player.as_ref().and_then(|p| {
          let dialog_name = p.current_dialog_name()?;
          let current_node = self.graph.get_node_by_name(dialog_name)?;

          if self.graph.get_selected_node_id().unwrap() == current_node.id() {
            p.current_node_index()
          } else {
            None
          }
        });

        let timeline = self.graph.get_mut_selected_timeline().unwrap();
        timeline.draw(ui, idx);
      });
  }

  fn combo_box<Opts, T>(
    ui: &Ui,
    target: &mut Rc<str>,
    id: &str,
    options: Opts,
    default_value: &Rc<str>,
    empty_value: Rc<str>,
  ) where
    Opts: IntoIterator<Item = T>,
    T: AsRef<str>,
  {
    let preview = if target.is_empty() {
      default_value
    } else {
      &target
    };

    ui.set_next_item_width(-1.0);
    if let Some(_) = ui.begin_combo(id, preview) {
      let unchanged_selected = target.is_empty();
      if ui
        .selectable_config("(Unchanged)")
        .selected(unchanged_selected)
        .build()
      {
        *target = empty_value;
      }

      let mut options: Vec<_> = options.into_iter().collect();
      options.sort_unstable_by(|a, b| natord::compare(a.as_ref(), b.as_ref()));

      for opt in options {
        let selected = target.as_ref() == opt.as_ref();
        if ui.selectable_config(&opt).selected(selected).build() {
          *target = Rc::from(opt.as_ref());
        }
      }
    }
  }

  fn play(
    ctx: &mut EditorContext,
    start_point: Option<(String, Vec<(Rc<str>, core::dialog::Dialog)>)>,
  ) {
    match start_point {
      Some((name, entries)) => {
        *ctx.dialog_mgr = core::dialog::DialogManager::new_from_entries(entries);
        let initial_dialog = ctx.dialog_mgr.build(&name).unwrap();
        *ctx.dialog_player = Some(core::dialog::DialogPlayer::new(name.into(), initial_dialog));
        ctx.dialog_player.as_mut().unwrap().play();
        info!("Playing Conversation...");
      }
      None => {
        ctx.model.load_saved_parameters();
        ctx.animator.stop_timer();
        ctx.animator.get_mut_parameter_state().reset();
        ctx.animator.clear_motion();
        *ctx.dialog_player = None;
        info!("Stoping Conversation...");
      }
    }
  }

  fn draw_float_property(
    ui: &Ui,
    name: &str,
    value: &mut f32,
    min: f32,
    max: f32,
    default: f32,
  ) -> bool {
    ui.text(name);

    ui.same_line_with_pos(120.0);

    ui.set_next_item_width(180.0);

    let changed = ui
      .slider_config(format!("##{}", name), min, max)
      .build(value);

    if ui.is_item_hovered() && ui.is_mouse_clicked(MouseButton::Middle) {
      *value = default;
    }

    changed
  }

  fn modal(ui: &Ui, id: &str, output: &mut String, open_modal: &mut bool) -> bool {
    if let Some(_) = ui.begin_modal_popup(id) {
      if !*open_modal {
        ui.set_keyboard_focus_here();
        *open_modal = true;
      }
      ui.input_text("##modal_name", output).build();

      if ui.button("Create") {
        if !output.is_empty() {
          ui.close_current_popup();
          *open_modal = false;
          return true;
        }
      }

      ui.same_line();

      if ui.button("Cancel") {
        output.clear();
        ui.close_current_popup();
        *open_modal = false;
        return false;
      }
    }

    return false;
  }
}
