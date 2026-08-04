use dear_imgui_rs::*;

use crate::timeline::{Selection, Timeline, TimelineBlockType};

pub struct EditorContext<'a> {
  pub model: &'a mut live2d::Model,
  pub animator: &'a mut live2d::animator::Animator,
  pub enummap: &'a mut live2d::animator::EnumMap,
}

pub enum Layout {
  Dialog {
    layout_initialized: bool,
    new_container_name: String,
    timeline: Timeline,
  },
  Enum {
    selected_enum: Option<(String, String)>,
    enumlog: Vec<(String, String)>,
    layout_initialized: bool,
  },
}

impl Layout {
  fn modal(ui: &Ui, id: &str, output: &mut String) -> bool {
    if let Some(_) = ui.begin_modal_popup(id) {
      ui.input_text("##container_name", output).build();

      if ui.button("Create") {
        if !output.is_empty() {
          ui.close_current_popup();
          return true;
        }
      }

      ui.same_line();

      if ui.button("Cancel") {
        output.clear();
        ui.close_current_popup();
        return false;
      }
    }

    return false;
  }

  pub fn draw(&mut self, ui: &mut Ui, ctx: EditorContext) {
    match self {
      Layout::Dialog {
        new_container_name,
        timeline,
        layout_initialized,
      } => {
        let viewport = ui.main_viewport();
        ui.set_next_window_viewport(viewport.id());
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

            if !*layout_initialized {
              *layout_initialized = true;
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

              if ui.button("Add Container") {
                ui.open_popup("Add Container");
              }

              if Self::modal(ui, "Add Container", new_container_name) {
                timeline.push_back_container(new_container_name.clone());
                new_container_name.clear();
              }

              enum DeleteCommand {
                Container(usize),
                Block(usize, usize),
              }

              let mut delete_command = None;

              if let Some(selected) = timeline.get_selected() {
                ui.same_line();
                ui.separator_vertical();
                ui.same_line();
                match selected {
                  Selection::Block(container_id, block) => {
                    if ui.button("Delete") {
                      delete_command = Some(DeleteCommand::Block(container_id, block.id));
                    }
                  }
                  Selection::Container(container) => {
                    if ui.button("Play Container") {}

                    ui.same_line();

                    ui.separator_vertical();

                    ui.same_line();

                    ui.text("Blocks:");

                    ui.same_line();
                    if ui.button("Text") {
                      container.add_block(TimelineBlockType::Text(String::new()));
                    }

                    ui.same_line();
                    if ui.button("Wait") {
                      container.add_block(TimelineBlockType::Wait(0.0));
                    }

                    ui.same_line();

                    ui.separator_vertical();

                    ui.same_line();

                    if ui.button("Delete") {
                      delete_command = Some(DeleteCommand::Container(container.id));
                    }
                  }
                }
              }

              if let Some(cmd) = delete_command.take() {
                match cmd {
                  DeleteCommand::Container(id) => timeline.delete_container(id),
                  DeleteCommand::Block(container_id, block_id) => timeline.delete_block(container_id, block_id),
                }
              }

              ui.separator();

              ui.child_window("##timeline_canvas")
                .size([0.0, 0.0])
                .flags(WindowFlags::HORIZONTAL_SCROLLBAR)
                .build(ui, || {
                  timeline.draw(ui);
                });
            });

            ui.window("Properties").build(|| {
              if let Some(selected) = timeline.get_selected() {
                match selected {
                  Selection::Container(container) => {
                    ui.text("Who?");
                    ui.same_line();
                    ui.input_text("##container_name", &mut container.name)
                      .build();
                  }
                  Selection::Block(container_id, block) => {}
                }
              }
            });
          });
      }
      Layout::Enum {
        selected_enum,
        enumlog,
        layout_initialized,
      } => {
        let viewport = ui.main_viewport();
        ui.set_next_window_viewport(viewport.id());
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

            if !*layout_initialized {
              *layout_initialized = true;
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
              | Self::draw_float_property(
                ui,
                e.id,
                e.value,
                e.min_value,
                e.max_value,
                e.default_value,
              )
          });

          if changed {
            ctx.model.update_parameters();
          }
        });

        if let Some((enum_name, value_name)) = selected_enum {
          ui.window("Inspector").build(|| {
            use live2d::animator::Value;

            // Listc
            if let Some(r#enum) = ctx.enummap.enums.get_mut(enum_name)
              && let Some(params) = r#enum.values.get_mut(value_name)
            {
              // =================
              // Toolbar
              // =================
              // Preview
              if ui.button("Preview") {
                enumlog.push((enum_name.clone(), value_name.clone()));
                for p in &*params {
                  ctx.animator.set_parameter(&p.name, p.value);
                }
              }

              if ui.button("Clear") {
                ctx.model.load_parameters();
                enumlog.clear();
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
                let selected = selected_enum
                  .as_ref()
                  .is_some_and(|(name, value)| enum_name == name && value_name == value);

                if ui.selectable_config(value_name).selected(selected).build() {
                  *selected_enum = Some((enum_name.clone(), value_name.clone()));
                }
              }
            }
          }
        });
        ui.window("Enum Log").build(|| {
          for l in enumlog {
            ui.text(format!("{}/{}", l.0, l.1));
          }
        });
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

    ui.same_line();
    ui.text(format!("{:.2}", value));

    changed
  }
}
