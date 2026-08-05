use dear_imgui_rs::*;

use crate::timeline::{Selection, Timeline, TimelineBlockType};

pub struct EditorContext<'a> {
  pub model: &'a mut core::live2d::Model,
  pub animator: &'a mut core::live2d::animator::Animator,
  pub enummap: &'a mut core::live2d::animator::EnumMap,
}

pub struct Layout {
  layout_initialized: bool,
  new_modal_string: String,
  open_modal: bool,
  timeline: Timeline,
  is_main_layout: bool,
  selected_enum: Option<(String, String)>,
  enumlog: Vec<(String, String)>,
}

impl Layout {
  pub fn new() -> Self {
    Self {
      layout_initialized: false,
      selected_enum: None,
      enumlog: Vec::new(),
      is_main_layout: true,
      open_modal: false,
      timeline: Timeline::new(),
      new_modal_string: String::new(),
    }
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

  pub fn draw(&mut self, ui: &mut Ui, ctx: EditorContext) {
    if ui.io().key_alt() && ui.is_key_pressed(Key::Key1) {
      self.is_main_layout = true;
      self.layout_initialized = false;
    }

    if ui.io().key_alt() && ui.is_key_pressed(Key::Key2) {
      self.is_main_layout = false;
      self.layout_initialized = false;
    }

    if let Some(_) = ui.begin_main_menu_bar() {
      ui.menu("View", || {
        if ui.menu_item("Dialog") {
          self.is_main_layout = true;
          self.layout_initialized = false;
        }

        if ui.menu_item("Enum") {
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

          // Inspector arriba, nueva ventana abajo
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
      let mut parameters: Vec<_> = ctx.model.get_parameters_iter().collect();
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

        // Listc
        if let Some(r#enum) = ctx.enummap.enums.get_mut(enum_name)
          && let Some(params) = r#enum.values.get_mut(value_name)
        {
          // =================
          // Toolbar
          // =================
          // Preview
          if ui.button("Preview") {
            self.enumlog.push((enum_name.clone(), value_name.clone()));
            for p in &*params {
              ctx.animator.set_parameter(&p.name, p.value);
            }
          }

          if ui.button("Clear") {
            ctx.model.load_parameters();
            self.enumlog.clear();
            ctx.animator.clear_parameters();
          }

          ui.separator();

          for p in params {
            let _scope = ui.push_id(&p.name);

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

  fn draw_dialogue_layout(&mut self, ui: &mut Ui, ctx: EditorContext) {
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

          DockBuilder::dock_window(ui, "Preview", dock_preview);
          DockBuilder::dock_window(ui, "Timeline", dock_timeline);
          DockBuilder::dock_window(ui, "Properties", dock_properties);

          DockBuilder::finish(ui, dockspace_id);
        }

        ui.window("Timeline").build(|| {
          if ui.button("Play Conversation") {}

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
            self
              .timeline
              .push_back_container(self.new_modal_string.clone());
            self.new_modal_string.clear();
          }

          enum DeleteCommand {
            Container(usize),
            Block(usize, usize),
          }

          let mut delete_command = None;

          if let Some(selected) = self.timeline.get_selected() {
            ui.same_line();
            ui.separator_vertical();
            ui.same_line();
            match selected {
              Selection::Block(container_id, block) => {
                if ui.button("(D)elete") || (ui.io().key_alt() && ui.is_key_pressed(Key::D)) {
                  delete_command = Some(DeleteCommand::Block(container_id, block.id));
                }
              }
              Selection::Container(container) => {
                if ui.button("Play Container") {
                }

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
                  container.add_block(TimelineBlockType::empty_set());
                }

                ui.same_line();

                ui.separator_vertical();

                ui.same_line();

                if ui.button("(D)elete") || (ui.io().key_alt() && ui.is_key_pressed(Key::D)) {
                  delete_command = Some(DeleteCommand::Container(container.id));
                }
              }
            }
          }

          if let Some(cmd) = delete_command.take() {
            match cmd {
              DeleteCommand::Container(id) => self.timeline.delete_container(id),
              DeleteCommand::Block(container_id, block_id) => {
                self.timeline.delete_block(container_id, block_id)
              }
            }
          }

          ui.separator();

          ui.child_window("##timeline_canvas")
            .size([0.0, 0.0])
            .flags(WindowFlags::HORIZONTAL_SCROLLBAR)
            .build(ui, || {
              self.timeline.draw(ui);
            });
        });

        ui.window("Properties").build(|| {
          if let Some(selected) = self.timeline.get_selected() {
            match selected {
              Selection::Container(container) => {
                ui.text("Who?");
                ui.same_line();
                ui.input_text("##container_name", &mut container.name)
                  .build();
              }
              Selection::Block(container_id, block) => match &mut block.value {
                TimelineBlockType::Text(text) => {
                  ui.text("Text:");
                  ui.same_line();
                  ui.input_text("##container_name", text).build();
                }
                TimelineBlockType::Wait(seconds) => {
                  ui.text("Wait:");
                  ui.same_line();

                  ui.input_float_config("##wait_seconds")
                    .step(0.1)
                    .build(seconds);

                  ui.same_line();
                  ui.text("seconds");
                }
                TimelineBlockType::Set { parameters, anim } => {
                  let mut remove_index: Option<usize> = None;

                  ui.text("Parameters");
                  ui.separator();

                  for (idx, entry) in parameters.iter_mut().enumerate() {
                    let _ = ui.push_id(idx as i32);

                    // --- Texto (nombre de la entrada) ---
                    ui.text(&entry.0);
                    ui.same_line();

                    // --- Combobox (tipo de valor) ---
                    ui.set_next_item_width(100.0);
                    match ctx.enummap.enums.get(&entry.0) {
                      Some(enumtype) => {
                        if let Some(_) =
                          ui.begin_combo(format!("##{}_{}", &entry.0, &entry.1), &entry.1)
                        {
                          for value in &enumtype.values {
                            let selected = entry.1 == *value.0;
                            if ui.selectable_config(value.0).selected(selected).build() {
                              entry.1 = value.0.clone();
                            }
                          }
                        }
                      }
                      None => ui.text("Unknown Enum"),
                    }
                    ui.same_line();

                    // --- Botón "x" (eliminar fila) ---
                    if ui.button("x") {
                      remove_index = Some(idx);
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
                    for prop_name in ctx.enummap.enums.keys() {
                      if ui.selectable_config(prop_name).size([0.0, 0.0]).build() {
                        parameters.push((prop_name.clone(), String::new()));
                        ui.close_current_popup();
                      }
                    }
                  }

                  ui.spacing();
                  ui.spacing();

                  ui.text("Animation");
                  ui.separator();

                  let preview = if anim.is_empty() {
                    "(Unchanged)"
                  } else {
                    anim.as_str()
                  };

                  ui.set_next_item_width(ui.content_region_avail()[0]);
                  if let Some(_) = ui.begin_combo("##anim_combo", preview) {
                    // Opción vacía = sin cambios
                    let unchanged_selected = anim.is_empty();
                    if ui
                      .selectable_config("(Unchanged)")
                      .selected(unchanged_selected)
                      .build()
                    {
                      anim.clear();
                    }

                    /*
                    for anim_name in &ctx.animations {
                      let selected = anim == anim_name;
                      if ui.selectable_config(anim_name).selected(selected).build() {
                        *anim = anim_name.clone();
                      }
                    }*/
                  }
                }
                _ => {}
              },
            }
          }
        });
      });
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

    ui.same_line();
    ui.text(format!("{:.2}", value));

    changed
  }
}
