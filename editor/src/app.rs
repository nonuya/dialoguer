use anyhow::Context;
use dear_imgui_glow::GlowRenderer;
use dear_imgui_rs::*;
use std::{fs::File, path::PathBuf, rc::Rc};
use winit::event::KeyEvent;

const BLOCK_WIDTH: f32 = 80.0;
const BLOCK_HEIGHT: f32 = 40.0;
const HEADER_HEIGHT: f32 = 22.0;
const CONTAINER_PADDING: f32 = 6.0;
const CONTAINER_SPACING: f32 = 16.0;

struct TimelineBlock {
  id: usize,
  name: String,
}

struct Container {
  id: usize,
  name: String,
  blocks: Vec<TimelineBlock>,
}

struct BlockDragState {
  container_idx: usize,
  block_idx: usize,
  offset_x: f32,
  current_x: f32,
}

struct ContainerDragState {
  container_idx: usize,
  offset_x: f32,
  current_x: f32,
}

struct Timeline {
  containers: Vec<Container>,
  drag: Option<BlockDragState>,
  container_drag: Option<ContainerDragState>,
  selected_container: Option<usize>,
}

impl Timeline {
  fn new() -> Self {
    Self {
      containers: vec![
        Container {
          id: 0,
          name: "Who".into(),
          blocks: vec![
            TimelineBlock {
              id: 0,
              name: "block1".into(),
            },
            TimelineBlock {
              id: 1,
              name: "block2".into(),
            },
          ],
        },
        Container {
          id: 1,
          name: "Who2".into(),
          blocks: vec![
            TimelineBlock {
              id: 2,
              name: "block1".into(),
            },
            TimelineBlock {
              id: 3,
              name: "block2".into(),
            },
          ],
        },
        Container {
          id: 1,
          name: "Who2".into(),
          blocks: vec![
            TimelineBlock {
              id: 2,
              name: "block1".into(),
            },
            TimelineBlock {
              id: 3,
              name: "block2".into(),
            },
          ],
        },
      ],
      drag: None,
      container_drag: None,
      selected_container: None,
    }
  }

  fn container_width(container: &Container) -> f32 {
    container.blocks.len().max(1) as f32 * BLOCK_WIDTH + CONTAINER_PADDING * 2.0
  }

  /// Posiciones "normales" (sin drag) de cada container, en orden actual del Vec.
  fn compute_layout(&self, origin_x: f32) -> (Vec<f32>, Vec<f32>) {
    let widths: Vec<f32> = self.containers.iter().map(Self::container_width).collect();
    let mut lefts = Vec::with_capacity(widths.len());
    let mut acc = origin_x;
    for w in &widths {
      lefts.push(acc);
      acc += w + CONTAINER_SPACING;
    }
    (lefts, widths)
  }

  fn draw(&mut self, ui: &Ui) {
    let origin = ui.cursor_screen_pos();
    let draw_list = ui.get_window_draw_list();

    let mouse_pos = ui.io().mouse_pos();
    let mouse_dragging = ui.is_mouse_dragging(MouseButton::Left);
    let mouse_released = ui.is_mouse_released(MouseButton::Left);

    let row_height = HEADER_HEIGHT + BLOCK_HEIGHT + CONTAINER_PADDING * 2.0;

    // Layout "normal" de todos los containers, antes de aplicar overrides de drag.
    let (normal_lefts, widths) = self.compute_layout(origin[0]);
    let total_width: f32 = widths.iter().sum::<f32>()
      + CONTAINER_SPACING * widths.len().max(1) as f32
      - CONTAINER_SPACING;

    for container_idx in 0..self.containers.len() {
      let container_width = widths[container_idx];

      let is_dragging_container = matches!(
        &self.container_drag,
        Some(d) if d.container_idx == container_idx
      );

      let is_selected = matches!(
        &self.selected_container,
        Some(idx) if *idx == self.containers[container_idx].id
      );

      // Si este container se está arrastrando, sigue al mouse; si no, su posición normal.
      let container_left = if is_dragging_container {
        self.container_drag.as_ref().unwrap().current_x
      } else {
        normal_lefts[container_idx]
      };
      let container_top = origin[1];

      // --- Fondo del container ---
      draw_list
        .add_rect(
          [container_left, container_top],
          [container_left + container_width, container_top + row_height],
          [0.13, 0.13, 0.13, 1.0],
        )
        .filled(true)
        .rounding(4.0)
        .build();

      let border_color = if is_dragging_container {
        [0.4, 0.6, 0.95, 1.0]
      } else if is_selected {
        [1.0, 1.0, 0.0, 1.0]
      } else {
        [0.05, 0.05, 0.05, 1.0]
      };

      draw_list
        .add_rect(
          [container_left, container_top],
          [container_left + container_width, container_top + row_height],
          border_color,
        )
        .rounding(4.0)
        .build();

      // --- Header: nombre + hitbox de drag del container ---
      let header_button_id = format!("##container_header_{}", self.containers[container_idx].id);
      ui.set_cursor_screen_pos([container_left, container_top]);
      ui.invisible_button(&header_button_id, [container_width, HEADER_HEIGHT]);

      let header_active = ui.is_item_active();
      let header_hovered = ui.is_item_hovered();

      if ui.is_item_clicked() {
        self.selected_container = Some(self.containers[container_idx].id);
      }

      // Iniciar drag de container: solo si no hay un bloque siendo arrastrado.
      if header_active && self.container_drag.is_none() && self.drag.is_none() {
        self.container_drag = Some(ContainerDragState {
          container_idx,
          offset_x: mouse_pos[0] - container_left,
          current_x: container_left,
        });
      }

      let header_text_color = if header_hovered || is_dragging_container {
        [1.0, 1.0, 1.0, 1.0]
      } else {
        [0.85, 0.85, 0.85, 1.0]
      };
      draw_list.add_text(
        [container_left + 8.0, container_top + 4.0],
        header_text_color,
        &self.containers[container_idx].name,
      );

      let track_x = container_left + CONTAINER_PADDING;
      let track_y = container_top + HEADER_HEIGHT + CONTAINER_PADDING;

      // --- Bloques dentro del container, contiguos ---
      let block_count = self.containers[container_idx].blocks.len();
      for block_idx in 0..block_count {
        let slot_x = track_x + block_idx as f32 * BLOCK_WIDTH;

        let is_dragging_block = matches!(
          &self.drag,
          Some(d) if d.container_idx == container_idx && d.block_idx == block_idx
        );

        let block_screen_pos = if is_dragging_block {
          [self.drag.as_ref().unwrap().current_x, track_y]
        } else {
          [slot_x, track_y]
        };

        let block = &self.containers[container_idx].blocks[block_idx];
        let button_id = format!("##block_{}", block.id);

        ui.set_cursor_screen_pos(block_screen_pos);
        ui.invisible_button(&button_id, [BLOCK_WIDTH - 2.0, BLOCK_HEIGHT]);

        let is_active = ui.is_item_active();
        let is_hovered = ui.is_item_hovered();

        // Iniciar drag de bloque: solo si no hay un container siendo arrastrado.
        if is_active && self.drag.is_none() && self.container_drag.is_none() {
          self.drag = Some(BlockDragState {
            container_idx,
            block_idx,
            offset_x: mouse_pos[0] - slot_x,
            current_x: slot_x,
          });
        }

        let color = if is_dragging_block {
          [0.35, 0.55, 0.9, 1.0]
        } else if is_hovered {
          [0.3, 0.45, 0.7, 1.0]
        } else {
          [0.25, 0.35, 0.55, 1.0]
        };

        let p_min = block_screen_pos;
        let p_max = [p_min[0] + BLOCK_WIDTH - 2.0, p_min[1] + BLOCK_HEIGHT];

        draw_list
          .add_rect(p_min, p_max, color)
          .filled(true)
          .rounding(4.0)
          .build();
        draw_list
          .add_rect(p_min, p_max, [0.05, 0.05, 0.05, 1.0])
          .rounding(4.0)
          .build();

        let text_size =
          ui.current_font()
            .calc_text_size(ui.current_font_size(), f32::MAX, 0.0, &block.name);
        let text_pos = [
          p_min[0] + ((BLOCK_WIDTH - 2.0) - text_size[0]) * 0.5,
          p_min[1] + (BLOCK_HEIGHT - text_size[1]) * 0.5,
        ];
        draw_list.add_text(text_pos, [1.0, 1.0, 1.0, 1.0], &block.name);
      }
    }

    // --- Actualizar posición del BLOQUE en drag ---
    if let Some(drag) = &mut self.drag {
      if mouse_dragging {
        let track_x = normal_lefts[drag.container_idx] + CONTAINER_PADDING;
        let container = &self.containers[drag.container_idx];
        let track_max_x = track_x + (container.blocks.len() as f32 - 1.0) * BLOCK_WIDTH;
        drag.current_x = (mouse_pos[0] - drag.offset_x).clamp(track_x, track_max_x);
      }
    }

    // --- Actualizar posición del CONTAINER en drag ---
    if let Some(drag) = &mut self.container_drag {
      if mouse_dragging {
        let width = widths[drag.container_idx];
        let min_x = origin[0];
        let max_x = origin[0] + total_width - width;
        drag.current_x = (mouse_pos[0] - drag.offset_x).clamp(min_x, max_x);
      }
    }

    // --- Soltar BLOQUE: reordenar dentro del mismo container ---
    if mouse_released {
      if let Some(drag) = self.drag.take() {
        let track_x = normal_lefts[drag.container_idx] + CONTAINER_PADDING;
        let container = &mut self.containers[drag.container_idx];
        let relative_x = drag.current_x - track_x;

        let mut target_index = (relative_x / BLOCK_WIDTH).round() as isize;
        target_index = target_index.clamp(0, container.blocks.len() as isize - 1);

        let moved_block = container.blocks.remove(drag.block_idx);
        container.blocks.insert(target_index as usize, moved_block);
      }
    }

    // --- Soltar CONTAINER: reordenar el Vec de containers según su posición final ---
    if mouse_released {
      if let Some(drag) = self.container_drag.take() {
        let dragged_width = widths[drag.container_idx];
        let dragged_center = drag.current_x + dragged_width * 0.5;

        let mut target_index = 0;
        for i in 0..self.containers.len() {
          if i == drag.container_idx {
            continue;
          }
          let center = normal_lefts[i] + widths[i] * 0.5;
          if center < dragged_center {
            target_index += 1;
          }
        }

        let moved_container = self.containers.remove(drag.container_idx);
        self.containers.insert(target_index, moved_container);
      }
    }

    // ui.dummy([total_width, row_height]);
  }
}
struct EditorContext<'a> {
  model: &'a mut live2d::Model,
  animator: &'a mut live2d::animator::Animator,
  enummap: &'a mut live2d::animator::EnumMap,
}

enum Layout {
  Dialog {
    layout_initialized: bool,
    timeline: Timeline,
  },
  Enum {
    selected_enum: Option<(String, String)>,
    enumlog: Vec<(String, String)>,
    layout_initialized: bool,
  },
}

impl Layout {
  fn draw(&mut self, ui: &mut Ui, ctx: EditorContext) {
    match self {
      Layout::Dialog {
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
              if ui.button("Add") {}
              ui.same_line();

              ui.separator();

              ui.child_window("##timeline_canvas")
                .size([0.0, 0.0])
                .flags(WindowFlags::HORIZONTAL_SCROLLBAR)
                .build(ui, || {
                  timeline.draw(ui);
                });
            });

            ui.window("Properties").build(|| {});
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

pub struct App {
  gl: Rc<glow::Context>,
  texture_id: TextureId,
  model: live2d::Model,
  enummap: live2d::animator::EnumMap,
  mvp: glam::Mat4,
  model_renderer: live2d::Renderer,
  animator: live2d::animator::Animator,
  layout: Layout,
}

impl App {
  pub fn new(model_path: PathBuf, renderer: &mut GlowRenderer) -> anyhow::Result<Self> {
    let gl = renderer.gl_context().unwrap().clone();

    let model_name = model_path
      .file_name()
      .ok_or_else(|| anyhow::anyhow!("Fail to get model name from '{}'", model_path.display()))?;

    let model_file =
      File::open(model_path.join(format!("{}.model3.json", model_name.display()))).context(
        format!("Failed to read .model3.json in '{}'", model_path.display()),
      )?;

    let model3 = cubism::json::model::Model3::from_reader(model_file).context(format!(
      "Failed to parse '{}.model3.json'",
      model_name.display()
    ))?;
    let mut model = live2d::Model::new(gl.clone(), &model_path, &model3)?;
    model.save_parameters();

    let mut enummap_path = model_path.join(model_name);
    enummap_path.set_extension("map");
    let enummap = live2d::animator::load_enum_map(&enummap_path)?;

    let model_renderer = live2d::Renderer::new(
      gl.clone(),
      live2d::config::MODEL_WIDTH,
      live2d::config::MODEL_HEIGHT,
    )
    .context("Failed to create Live2D Renderer")?;

    let texture_id = TextureId::new(1000);
    renderer
      .texture_map_mut()
      .set(texture_id, model_renderer.tex());

    Ok(Self {
      gl,
      texture_id,
      model,
      enummap,
      mvp: glam::Mat4::from_scale(glam::vec3(2.0, 2.0, 1.0)),
      model_renderer,
      animator: live2d::animator::Animator::new(),
      layout: Layout::Dialog {
        timeline: Timeline::new(),
        layout_initialized: false,
      },
    })
  }

  pub fn update(&mut self, deltatime: f32) {
    self.animator.update(deltatime, &mut self.model);
  }

  pub fn draw(&mut self, ui: &mut Ui) {
    self.model_renderer.draw(&self.model, &self.mvp);

    let ctx = EditorContext {
      model: &mut self.model,
      animator: &mut self.animator,
      enummap: &mut self.enummap,
    };
    self.layout.draw(ui, ctx);

    ui.window("Preview").build(|| {
      /*let available = ui.content_region_avail();

      if available[0] <= 0.0 || available[1] <= 0.0 {
        return;
      }

      let mut draw_size = available;

      if available[0] / available[1] > 1.0 {
        draw_size[0] = available[1];
      } else {
        draw_size[1] = available[0];
      }

      let cursor = ui.cursor_pos();

      ui.set_cursor_pos([
        cursor[0] + (available[0] - draw_size[0]) * 0.5,
        cursor[1] + (available[1] - draw_size[1]) * 0.5,
      ]);

      Image::new(ui, self.texture_id, draw_size)
        .uv0([0.0, 1.0])
        .uv1([1.0, 0.0])
        .build();*/
    });
  }

  pub fn resize(&mut self, width: u32, height: u32) {}

  pub fn keyboard(&mut self, event: KeyEvent) {}
}
