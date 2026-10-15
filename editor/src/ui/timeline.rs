use crate::{
  project::ProjectAssets,
  ui::{
    UiState,
    editor::{UiCommand, UiOutput},
  },
};
use dear_imgui_rs::{DrawListMut, FocusedFlags, Font, MouseButton, WindowFlags};
use std::rc::Rc;

const BLOCK_WIDTH: f32 = 50.0;
const BLOCK_HEIGHT: f32 = 40.0;
const HEADER_HEIGHT: f32 = 22.0;
const CONTAINER_PADDING: f32 = 6.0;
const SELECTED_COLOR: [f32; 4] = [1.0, 1.0, 0.0, 1.0];
const CONTAINER_SPACING: f32 = 6.0;

#[derive(Clone)]
pub enum TimelineBlockType {
  Set(TimelineSetBlockType),
  Text(String),
  Wait(f32),
  Next,
}

impl TimelineBlock {
  fn draw(
    &self,
    font: &Font,
    font_size: f32,
    draw_list: &DrawListMut,
    p_min: [f32; 2],
    p_max: [f32; 2],
    border_color: [f32; 4],
  ) {
    let (name, color) = match self.value {
      TimelineBlockType::Set { .. } => (
        "Set",
        [0.20, 0.45, 0.78, 1.0], // Azul
      ),
      TimelineBlockType::Text(..) => (
        "Text",
        [0.22, 0.58, 0.32, 1.0], // Verde
      ),
      TimelineBlockType::Wait(..) => (
        "Wait",
        [0.74, 0.54, 0.18, 1.0], // Ámbar
      ),
      TimelineBlockType::Next => (
        "Next",
        [0.55, 0.35, 0.70, 1.0], // Morado
      ),
    };
    draw_list
      .add_rect(p_min, p_max, color)
      .filled(true)
      .rounding(4.0)
      .build();

    draw_list
      .add_rect(p_min, p_max, border_color)
      .rounding(4.0)
      .build();

    let text_size = font.calc_text_size(font_size, f32::MAX, 0.0, &name);
    let text_pos = [
      p_min[0] + ((BLOCK_WIDTH - 2.0) - text_size[0]) * 0.5,
      p_min[1] + (BLOCK_HEIGHT - text_size[1]) * 0.5,
    ];

    draw_list.add_text(text_pos, [1.0, 1.0, 1.0, 1.0], name);
  }
}

#[derive(Default, Clone)]
pub struct TimelineSetBlockType {
  parameters: Vec<(Rc<str>, Rc<str>)>, // EnumName ValueName
  anim: (Rc<str>, bool),
  view: Rc<str>,
}

impl TimelineSetBlockType {
  fn is_empty(&self) -> bool {
    self.parameters.is_empty() && self.view.is_empty() && self.anim.0.is_empty()
  }
}

#[derive(Clone)]
pub struct TimelineBlock {
  pub id: usize,
  pub value: TimelineBlockType,
}

#[derive(Clone)]
pub struct Container {
  id: usize,
  name: String,
  blocks: Vec<TimelineBlock>,
}

impl Container {
  pub fn export_to_dialog_node(&self) -> core::dialog::DialogNode {
    use core::dialog::Event;

    let mut events = Vec::new();

    for b in &self.blocks {
      match &b.value {
        TimelineBlockType::Wait(seconds) => events.push(Event::Wait(*seconds)),
        TimelineBlockType::Text(text) => events.push(Event::Text(text.as_str().into())),
        TimelineBlockType::Set(TimelineSetBlockType {
          parameters,
          anim,
          view,
        }) => {
          if !view.is_empty() {
            events.push(Event::SetView(view.clone()));
          }

          if !anim.0.is_empty() {
            events.push(Event::PlayAnimation(anim.0.clone(), anim.1));
          }

          for p in parameters {
            if p.1.as_ref() == "NonControl" {
              events.push(Event::RemoveEnum(p.0.clone()));
            } else {
              events.push(Event::SetParameter(p.0.clone(), p.1.clone()));
            }
          }
        }
        TimelineBlockType::Next => events.push(Event::Next),
      }
    }

    core::dialog::DialogNode {
      label: self.name.as_str().into(),
      events,
    }
  }
}

#[derive(Clone)]
struct BlockDragState {
  container_idx: usize,
  block_idx: usize,
  offset_x: f32,
  current_x: f32,
}

#[derive(Clone)]
struct ContainerDragState {
  container_idx: usize,
  offset_x: f32,
  current_x: f32,
  offset_y: f32,
  current_y: f32,
}

pub enum Selection {
  Block {
    container_index: usize,
    container_id: usize,
    block_index: usize,
  },
  Container {
    index: usize,
  },
}

pub struct UiTimelineState {
  containers: Vec<Container>,
  drag: Option<BlockDragState>,
  container_drag: Option<ContainerDragState>,
  selected_container_id: Option<usize>,
  // (container_id, block_id) — estable aunque se reordenen los Vec
  selected_block: Option<(usize, usize)>,
}

impl UiTimelineState {
  pub fn new(dialog_nodes: &Vec<core::dialog::DialogNode>) -> Self {
    use core::dialog::Event;

    let mut containers = Vec::new();
    for (id, node) in dialog_nodes.iter().enumerate() {
      let mut blocks = Vec::new();
      let mut pending_set = TimelineSetBlockType::default();
      let mut block_id: usize = 0;

      let mut add_block = |value| {
        blocks.push(TimelineBlock {
          id: block_id,
          value,
        });

        block_id += 1;
      };

      for event in &node.events {
        match event {
          Event::SetParameter(name, value) => {
            pending_set.parameters.push((name.clone(), value.clone()))
          }
          Event::PlayAnimation(name, looped) => {
            pending_set.anim = (name.clone(), *looped);
          }
          Event::RemoveEnum(name) => pending_set
            .parameters
            .push((name.clone(), "NonControl".into())),
          Event::SetView(name) => pending_set.view = name.clone(),
          Event::SetMainChoicer(_) => {
            // Managed by Graph
          }
          other => {
            if !pending_set.is_empty() {
              add_block(TimelineBlockType::Set(std::mem::take(&mut pending_set)));
            }

            let value = match other {
              Event::Text(text) => TimelineBlockType::Text(text.to_string()),
              Event::Wait(seconds) => TimelineBlockType::Wait(*seconds),
              Event::Next => TimelineBlockType::Next,
              Event::Jump(..) => continue,
              _ => unreachable!("{:#?}", other),
            };

            add_block(value);
          }
        }
      }

      // Si solo es un Nodo con puros Sets
      if !pending_set.is_empty() {
        add_block(TimelineBlockType::Set(std::mem::take(&mut pending_set)));
      }

      if !blocks.is_empty() {
        containers.push(Container {
          id,
          name: node.label.to_string(),
          blocks,
        });
      }
    }

    Self {
      containers,
      drag: None,
      container_drag: None,
      selected_container_id: None,
      selected_block: None,
    }
  }

  pub fn get_selected(&self) -> Option<Selection> {
    if let Some((container_id, block_id)) = self.selected_block {
      let container_index = self.containers.iter().position(|c| c.id == container_id)?;
      let block_index = self.containers[container_index]
        .blocks
        .iter()
        .position(|b| b.id == block_id)?;

      return Some(Selection::Block {
        container_index,
        container_id,
        block_index,
        // block: &mut self.containers[container_index].blocks[block_idx],
      });
    } else if let Some(container_id) = self.selected_container_id {
      let index = self.containers.iter().position(|c| c.id == container_id)?;
      return Some(Selection::Container {
        index,
        // container: &mut self.containers[index],
      });
    } else {
      return None;
    }
  }

  pub fn export_to_dialog(&self) -> core::dialog::Dialog {
    let dialogs = self
      .containers
      .iter()
      .map(Container::export_to_dialog_node)
      .collect();

    core::dialog::Dialog::Conversation(dialogs)
  }
}

pub enum UiTimelineCommand {
  RemoveBlock {
    container_id: usize,
    block_id: usize,
  },
  RemoveContainer {
    container_id: usize,
  },
  AddContainer(TimelineBlockType),
  MoveBlock {
    container_id: usize,
    block_id: usize,
    target_index: usize,
  },
  MoveContainer {
    container_id: usize,
    target_index: usize,
  },
}

pub fn draw_timeline(
  ui: &dear_imgui_rs::Ui,
  is_dialog_playing: bool,
  current_container_index: Option<usize>,
  state: &mut UiTimelineState,
) -> UiOutput {
  let mut output = UiOutput::default();

  // ==========================
  // Toolbar
  // ==========================
  if is_dialog_playing {
    if ui.button("Stop Conversation") {
      output.commands.push(UiCommand::StopDialog);
    }
  } else {
    if ui.button("Play Conversation") {
      output.commands.push(UiCommand::PlayDialog { skip: 0 });
    }
  }

  ui.same_line();

  if ui.button("(A)dd Container") {
    // ui.open_popup("Add Container");
  }

  if let Some(selected) = state.get_selected() {
    ui.same_line();
    ui.separator_vertical();
    ui.same_line();

    // Selected
    if ui.button("Play Container") {
      match selected {
        Selection::Block {
          container_index, ..
        } => {
          output.commands.push(UiCommand::PlayDialog {
            skip: container_index,
          });
        }
        Selection::Container { index, .. } => {
          output.commands.push(UiCommand::PlayDialog { skip: index });
        }
      }
    }

    ui.same_line();

    if ui.button("(D)elete") {
      match &selected {
        Selection::Block {
          container_id,
          container_index,
          block_index,
          ..
        } => {
          let block = &state.containers[*container_index].blocks[*block_index];

          output
            .commands
            .push(UiCommand::Timeline(UiTimelineCommand::RemoveBlock {
              container_id: *container_id,
              block_id: block.id,
            }));
        }
        Selection::Container { index } => {
          let container = &state.containers[*index];
          output
            .commands
            .push(UiCommand::Timeline(UiTimelineCommand::RemoveContainer {
              container_id: container.id,
            }));
        }
      }
    }

    match selected {
      Selection::Block { .. } => {}
      Selection::Container { .. } => {
        let mut add_effect = |t| {
          output
            .commands
            .push(UiCommand::Timeline(UiTimelineCommand::AddContainer(t)))
        };

        ui.same_line();

        ui.separator_vertical();

        ui.same_line();

        ui.text("Blocks:");

        ui.same_line();
        if ui.button("(T)ext") {
          add_effect(TimelineBlockType::Text(String::new()));
        }

        ui.same_line();
        if ui.button("(W)ait") {
          add_effect(TimelineBlockType::Wait(0.0));
        }

        ui.same_line();

        if ui.button("(S)et") {
          add_effect(TimelineBlockType::Set(TimelineSetBlockType::default()));
        }

        ui.same_line();

        if ui.button("(N)ext") {
          add_effect(TimelineBlockType::Next);
        }
      }
    }
  }

  ui.separator();

  // =========================
  // Viewport
  // ==========================
  ui.child_window("##timeline_canvas")
    .size([0.0, 0.0])
    .flags(WindowFlags::HORIZONTAL_SCROLLBAR)
    .build(ui, || {
      let origin = ui.cursor_screen_pos();
      let draw_list = ui.get_window_draw_list();

      let mouse_pos = ui.io().mouse_pos();
      let mouse_dragging = ui.is_mouse_dragging(MouseButton::Left);
      let mouse_released = ui.is_mouse_released(MouseButton::Left);

      let row_height = HEADER_HEIGHT + BLOCK_HEIGHT + CONTAINER_PADDING * 2.0;
      let row_stride = row_height + CONTAINER_SPACING; // alto de fila + separación vertical

      let avail_width = ui.content_region_avail()[0];

      // widths sigue viniendo de compute_layout (ancho "natural" de cada container).
      let (_normal_lefts, widths) = compute_layout(&state.containers, origin[0]);

      // --- Layout con wrap: asigna (row, left) a cada container ---
      let mut rows: Vec<usize> = Vec::with_capacity(widths.len());
      let mut lefts: Vec<f32> = Vec::with_capacity(widths.len());
      {
        let mut cur_row = 0usize;
        let mut cur_x = origin[0];
        for (i, &w) in widths.iter().enumerate() {
          // Si no es el primero de la fila y no entra, salta de fila.
          if i > 0 && cur_x > origin[0] && cur_x + w > origin[0] + avail_width {
            cur_row += 1;
            cur_x = origin[0];
          }
          rows.push(cur_row);
          lefts.push(cur_x);
          cur_x += w + CONTAINER_SPACING;
        }
      }
      let max_row = rows.iter().copied().max().unwrap_or(0);

      for container_idx in 0..state.containers.len() {
        let container_id = state.containers[container_idx].id;
        let container_width = widths[container_idx];

        let is_dragging_container = matches!(
          &state.container_drag,
          Some(d) if d.container_idx == container_idx
        );

        let is_container_highlighted = state.selected_block.is_none()
          && matches!(&state.selected_container_id, Some(id) if *id == container_id);

        let (container_left, container_top) = if is_dragging_container {
          let d = state.container_drag.as_ref().unwrap();
          (d.current_x, d.current_y)
        } else {
          (
            lefts[container_idx],
            origin[1] + rows[container_idx] as f32 * row_stride,
          )
        };

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
        } else if is_container_highlighted {
          SELECTED_COLOR
        } else if current_container_index.is_some_and(|idx| container_idx == idx) {
          [1.0, 0.0, 0.0, 1.0]
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

        // --- Header ---
        let header_button_id = format!("##container_header_{}", container_id);
        ui.set_cursor_screen_pos([container_left, container_top]);
        ui.invisible_button(&header_button_id, [container_width, HEADER_HEIGHT]);

        let header_active = ui.is_item_active();
        let header_hovered = ui.is_item_hovered();

        if ui.is_item_clicked() {
          state.selected_container_id = Some(container_id);
          state.selected_block = None;
        }

        if header_active && state.container_drag.is_none() && state.drag.is_none() {
          state.container_drag = Some(ContainerDragState {
            container_idx,
            offset_x: mouse_pos[0] - container_left,
            current_x: container_left,
            offset_y: mouse_pos[1] - container_top,
            current_y: container_top,
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
          &state.containers[container_idx].name,
        );

        let track_x = container_left + CONTAINER_PADDING;
        let track_y = container_top + HEADER_HEIGHT + CONTAINER_PADDING;

        // --- Bloques dentro del container: siguen en línea horizontal, sin cambios ---
        let block_count = state.containers[container_idx].blocks.len();
        for block_idx in 0..block_count {
          let block_id = state.containers[container_idx].blocks[block_idx].id;
          let slot_x = track_x + block_idx as f32 * BLOCK_WIDTH;

          let is_dragging_block = matches!(
            &state.drag,
            Some(d) if d.container_idx == container_idx && d.block_idx == block_idx
          );

          let is_block_selected = matches!(
            &state.selected_block,
            Some((cid, bid)) if *cid == container_id && *bid == block_id
          );

          let block_screen_pos = if is_dragging_block {
            [state.drag.as_ref().unwrap().current_x, track_y]
          } else {
            [slot_x, track_y]
          };

          let block = &state.containers[container_idx].blocks[block_idx];
          let button_id = format!("##{}_{}", container_id, block.id);

          ui.set_cursor_screen_pos(block_screen_pos);
          ui.invisible_button(&button_id, [BLOCK_WIDTH - 2.0, BLOCK_HEIGHT]);

          let is_active = ui.is_item_active();

          if ui.is_item_clicked() {
            state.selected_container_id = Some(container_id);
            state.selected_block = Some((container_id, block_id));
          }

          if is_active && state.drag.is_none() && state.container_drag.is_none() {
            state.drag = Some(BlockDragState {
              container_idx,
              block_idx,
              offset_x: mouse_pos[0] - slot_x,
              current_x: slot_x,
            });
          }

          let p_min = block_screen_pos;
          let p_max = [p_min[0] + BLOCK_WIDTH - 2.0, p_min[1] + BLOCK_HEIGHT];

          let block_border_color = if is_block_selected {
            SELECTED_COLOR
          } else {
            [0.05, 0.05, 0.05, 1.0]
          };

          block.draw(
            ui.current_font(),
            ui.current_font_size(),
            &draw_list,
            p_min,
            p_max,
            block_border_color,
          );
        }
      }

      // --- Actualizar posición del BLOQUE en drag (sin cambios) ---
      if let Some(drag) = &mut state.drag {
        if mouse_dragging {
          let track_x = lefts[drag.container_idx] + CONTAINER_PADDING;
          let container = &state.containers[drag.container_idx];
          let track_max_x = track_x + (container.blocks.len() as f32 - 1.0) * BLOCK_WIDTH;
          drag.current_x = (mouse_pos[0] - drag.offset_x).clamp(track_x, track_max_x);
        }
      }

      // --- Actualizar posición del CONTAINER en drag: ahora libre en X e Y ---
      if let Some(drag) = &mut state.container_drag {
        if mouse_dragging {
          let width = widths[drag.container_idx];
          let min_x = origin[0];
          let max_x = origin[0] + avail_width - width;
          drag.current_x = (mouse_pos[0] - drag.offset_x).clamp(min_x, max_x);

          let min_y = origin[1];
          let max_y = origin[1] + max_row as f32 * row_stride;
          drag.current_y = (mouse_pos[1] - drag.offset_y).clamp(min_y, max_y);
        }
      }

      // --- Soltar BLOQUE: sin cambios ---
      if mouse_released {
        if let Some(drag) = state.drag.take() {
          let track_x = lefts[drag.container_idx] + CONTAINER_PADDING;
          let container = &state.containers[drag.container_idx];
          let relative_x = drag.current_x - track_x;

          let mut target_index = (relative_x / BLOCK_WIDTH).round() as isize;
          target_index = target_index.clamp(0, container.blocks.len() as isize - 1);

          if target_index as usize != drag.block_idx {
            output
              .commands
              .push(UiCommand::Timeline(UiTimelineCommand::MoveBlock {
                container_id: container.id,
                block_id: container.blocks[drag.block_idx].id,
                target_index: target_index as usize,
              }));
          }
        }
      }

      // --- Soltar CONTAINER: reordenar considerando fila (Y) y columna (X) ---
      if mouse_released {
        if let Some(drag) = state.container_drag.take() {
          let dragged_row = ((drag.current_y - origin[1]) / row_stride)
            .round()
            .clamp(0.0, max_row as f32) as usize;
          let trigger_x = drag.current_x + widths[drag.container_idx] * 0.5;

          let mut target_index = 0;
          for i in 0..state.containers.len() {
            if i == drag.container_idx {
              continue;
            }
            let other_center_x = lefts[i] + widths[i] * 0.5;
            let goes_before =
              rows[i] < dragged_row || (rows[i] == dragged_row && other_center_x < trigger_x);
            if goes_before {
              target_index += 1;
            }
          }

          if target_index != drag.container_idx {
            output
              .commands
              .push(UiCommand::Timeline(UiTimelineCommand::MoveContainer {
                container_id: state.containers[drag.container_idx].id,
                target_index,
              }));
          }
        }
      }
    });

  output
}

fn compute_layout(containers: &[Container], origin_x: f32) -> (Vec<f32>, Vec<f32>) {
  let widths: Vec<f32> = containers.iter().map(container_width).collect();
  let mut lefts = Vec::with_capacity(widths.len());
  let mut acc = origin_x;
  for w in &widths {
    lefts.push(acc);
    acc += w + CONTAINER_SPACING;
  }
  (lefts, widths)
}

fn container_width(container: &Container) -> f32 {
  container.blocks.len().max(1) as f32 * BLOCK_WIDTH + CONTAINER_PADDING * 2.0
}

pub enum UiPropertiesCommand {
  ChangeText {
    container_id: usize,
    block_id: usize,
    value: String,
  },
  ChangeWait {
    container_id: usize,
    block_id: usize,
    value: f32,
  },
  ChangeAnimation {
    container_id: usize,
    block_id: usize,
    value: Rc<str>,
  },
  ChangeLoop {
    container_id: usize,
    block_id: usize,
    value: bool,
  },
  ChangeView {
    container_id: usize,
    block_id: usize,
    value: Rc<str>,
  },
  ChangeParameter {
    container_id: usize,
    block_id: usize,
    parameter_index: usize,
    value: Rc<str>,
  },
  AddParameter {
    container_id: usize,
    block_id: usize,
    index: usize,
    value: (Rc<str>, Rc<str>),
  },
  RemoveParameter {
    container_id: usize,
    block_id: usize,
    index: usize,
  },
  RenameContainer {
    container_id: usize,
    value: String,
  },
}

pub fn draw_properties(
  ui: &dear_imgui_rs::Ui,
  selected: Selection,
  unchanged_str: Rc<str>,
  empty_str: Rc<str>,
  noncontrol_str: Rc<str>,
  assets: &ProjectAssets,
  state: &UiTimelineState,
) -> UiOutput {
  let mut output = UiOutput::default();

  match selected {
    Selection::Block {
      container_index,
      container_id,
      block_index,
    } => {
      let block = &state.containers[container_index].blocks[block_index];

      match &block.value {
        TimelineBlockType::Text(text) => {
          ui.text("Text:");
          ui.same_line();
          ui.set_next_item_width(-1.0);
          let mut value = text.clone();
          if ui
            .input_text(format!("##{}_{}_text", container_id, block.id), &mut value)
            .build()
          {
            output
              .commands
              .push(UiCommand::Properties(UiPropertiesCommand::ChangeText {
                container_id,
                block_id: block.id,
                value,
              }));
          }
        }
        TimelineBlockType::Wait(seconds) => {
          ui.text("Wait:");
          ui.same_line();

          let mut value = *seconds;
          if ui
            .input_float_config("##wait_seconds")
            .step(0.10)
            .build(&mut value)
          {
            output
              .commands
              .push(UiCommand::Properties(UiPropertiesCommand::ChangeWait {
                container_id,
                block_id: block.id,
                value,
              }));
          }

          ui.same_line();
          ui.text("seconds");
        }
        TimelineBlockType::Set(TimelineSetBlockType {
          parameters,
          anim,
          view,
        }) => {
          let block_id = block.id;

          // ANIMATION
          ui.text("Animation");
          ui.separator();
          let mut looped = anim.1;
          if ui.checkbox("Loop?", &mut looped) {
            output
              .commands
              .push(UiCommand::Properties(UiPropertiesCommand::ChangeLoop {
                container_id,
                block_id,
                value: looped,
              }));
          }
          ui.same_line();

          let mut animation = anim.0.clone();
          if combo_box(
            ui,
            &mut animation,
            "##anim_combo",
            assets.motion_mgr.names().as_slice(),
            &unchanged_str,
            empty_str.clone(),
          ) {
            output.commands.push(UiCommand::Properties(
              UiPropertiesCommand::ChangeAnimation {
                container_id,
                block_id,
                value: animation,
              },
            ));
          }

          ui.text("View");
          ui.separator();

          let keys: Vec<_> = assets.model_def.views.keys().collect();
          // VIEW
          let mut selected_view = view.clone();
          if combo_box(
            ui,
            &mut selected_view,
            "##view_combo",
            keys.as_slice(),
            &unchanged_str,
            empty_str.clone(),
          ) {
            output
              .commands
              .push(UiCommand::Properties(UiPropertiesCommand::ChangeView {
                container_id,
                block_id,
                value: selected_view,
              }));
          }

          ui.spacing();
          ui.text("Parameters");
          ui.separator();

          let mut remove_index: Option<usize> = None;

          for (idx, entry) in parameters.iter().enumerate() {
            let _id = ui.push_id(idx as i32);

            if ui.button("X") {
              remove_index = Some(idx);
            }

            ui.same_line();

            ui.text(&entry.0);
            ui.same_line();
            ui.set_next_item_width(-10.0);
            match assets.model_def.enums.get(&entry.0) {
              Some(enumtype) => {
                if let Some(_) = ui.begin_combo(format!("##{}_{}", &entry.0, &entry.1), &entry.1) {
                  let mut values: Vec<_> = enumtype.values.iter().collect();
                  values.sort_unstable_by_key(|&p| p.0);

                  for value in values
                    .iter()
                    .map(|v| v.0)
                    .chain(std::iter::once(&noncontrol_str))
                  {
                    let selected = entry.1 == *value;
                    if ui.selectable_config(value).selected(selected).build() {
                      output.commands.push(UiCommand::Properties(
                        UiPropertiesCommand::ChangeParameter {
                          container_id,
                          block_id,
                          parameter_index: idx,
                          value: value.clone(),
                        },
                      ));
                    }
                  }
                }
              }
              None => ui.text("Unknown Enum"),
            }
          }

          if let Some(idx) = remove_index {
            output.commands.push(UiCommand::Properties(
              UiPropertiesCommand::RemoveParameter {
                container_id,
                block_id,
                index: idx,
              },
            ));
          }

          ui.spacing();
          // --- Botón "+" al final: abre un combobox para elegir qué agregar ---
          if ui.button_with_size("+", [ui.content_region_avail()[0], 0.0]) {
            ui.open_popup("##add_entry_popup");
          }

          if let Some(_) = ui.begin_popup("##add_entry_popup") {
            let mut params: Vec<_> = assets.model_def.enums.iter().collect();
            params.sort_unstable_by_key(|&p| p.0);
            for (prop_name, prop_value) in params {
              if ui.selectable_config(prop_name).size([0.0, 0.0]).build() {
                let value = (
                  prop_name.clone(),
                  prop_value.values.iter().next().unwrap().0.clone(),
                );
                let index = parameters.len();
                output
                  .commands
                  .push(UiCommand::Properties(UiPropertiesCommand::AddParameter {
                    container_id,
                    block_id,
                    index,
                    value,
                  }));
                ui.close_current_popup();
              }
            }
          }
        }
        TimelineBlockType::Next => {}
      }
    }
    Selection::Container { index } => {
      let container = &state.containers[index];
      ui.text("Who?");
      ui.same_line();
      let mut name = container.name.clone();
      if ui
        .input_text(format!("##container_{}", container.id), &mut name)
        .build()
      {
        output.commands.push(UiCommand::Properties(
          UiPropertiesCommand::RenameContainer {
            container_id: container.id,
            value: name,
          },
        ));
      }
    }
  }

  output
}

fn combo_box(
  ui: &dear_imgui_rs::Ui,
  target: &mut Rc<str>,
  id: &str,
  options: &[&Rc<str>],
  default_value: &Rc<str>,
  empty_value: Rc<str>,
) -> bool {
  let mut changed = false;
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
      changed = true;
    }

    let mut options: Vec<_> = options.into_iter().collect();
    options.sort_unstable_by(|a, b| natord::compare(a.as_ref(), b.as_ref()));

    for opt in options {
      let selected = target.as_ref() == opt.as_ref();
      if ui.selectable_config(&opt).selected(selected).build() {
        *target = Rc::clone(opt);
        changed = true;
      }
    }
  }

  changed
}

pub fn handle_ui_timeline_command(cmd: UiTimelineCommand, state: &mut UiState) {
  let Some(timeline_state) = state.graph_state.get_mut_selected_timeline() else {
    return;
  };

  match cmd {
    UiTimelineCommand::RemoveBlock {
      container_id,
      block_id,
    } => {
      if let Some(container) = timeline_state
        .containers
        .iter_mut()
        .find(|container| container.id == container_id)
      {
        container.blocks.retain(|block| block.id != block_id);
      }

      if timeline_state.selected_block == Some((container_id, block_id)) {
        timeline_state.selected_block = None;
      }
    }

    UiTimelineCommand::RemoveContainer { container_id } => {
      timeline_state
        .containers
        .retain(|container| container.id != container_id);

      if timeline_state.selected_container_id == Some(container_id) {
        timeline_state.selected_container_id = None;
      }

      if timeline_state
        .selected_block
        .is_some_and(|(id, _)| id == container_id)
      {
        timeline_state.selected_block = None;
      }
    }

    UiTimelineCommand::AddContainer(block_type) => {
      let container_id = timeline_state
        .containers
        .iter()
        .map(|container| container.id)
        .max()
        .map_or(0, |id| id + 1);

      timeline_state.containers.push(Container {
        id: container_id,
        name: format!("Container {container_id}"),
        blocks: vec![TimelineBlock {
          id: 0,
          value: block_type,
        }],
      });
    }

    UiTimelineCommand::MoveBlock {
      container_id,
      block_id,
      target_index,
    } => {
      let Some(container) = timeline_state
        .containers
        .iter_mut()
        .find(|container| container.id == container_id)
      else {
        return;
      };

      let Some(current_index) = container
        .blocks
        .iter()
        .position(|block| block.id == block_id)
      else {
        return;
      };

      if container.blocks.is_empty() || current_index == target_index {
        return;
      }

      let target_index = target_index.min(container.blocks.len() - 1);
      let block = container.blocks.remove(current_index);
      container.blocks.insert(target_index, block);
    }

    UiTimelineCommand::MoveContainer {
      container_id,
      target_index,
    } => {
      let Some(current_index) = timeline_state
        .containers
        .iter()
        .position(|container| container.id == container_id)
      else {
        return;
      };

      if timeline_state.containers.is_empty() || current_index == target_index {
        return;
      }

      let target_index = target_index.min(timeline_state.containers.len() - 1);
      let container = timeline_state.containers.remove(current_index);
      timeline_state.containers.insert(target_index, container);
    }
  }
}

pub fn handle_ui_properties_command(cmd: UiPropertiesCommand, state: &mut UiState) {
  let Some(timeline) = state.graph_state.get_mut_selected_timeline() else {
    return;
  };

  match cmd {
    UiPropertiesCommand::ChangeText {
      container_id,
      block_id,
      value,
    } => {
      if let Some(TimelineBlockType::Text(text)) =
        find_block_value_mut(timeline, container_id, block_id)
      {
        *text = value;
      }
    }
    UiPropertiesCommand::ChangeWait {
      container_id,
      block_id,
      value,
    } => {
      if let Some(TimelineBlockType::Wait(seconds)) =
        find_block_value_mut(timeline, container_id, block_id)
      {
        *seconds = value;
      }
    }
    UiPropertiesCommand::ChangeAnimation {
      container_id,
      block_id,
      value,
    } => {
      if let Some(TimelineBlockType::Set(set)) =
        find_block_value_mut(timeline, container_id, block_id)
      {
        set.anim.0 = value;
      }
    }
    UiPropertiesCommand::ChangeLoop {
      container_id,
      block_id,
      value,
    } => {
      if let Some(TimelineBlockType::Set(set)) =
        find_block_value_mut(timeline, container_id, block_id)
      {
        set.anim.1 = value;
      }
    }
    UiPropertiesCommand::ChangeView {
      container_id,
      block_id,
      value,
    } => {
      if let Some(TimelineBlockType::Set(set)) =
        find_block_value_mut(timeline, container_id, block_id)
      {
        set.view = value;
      }
    }
    UiPropertiesCommand::ChangeParameter {
      container_id,
      block_id,
      parameter_index,
      value,
    } => {
      if let Some(TimelineBlockType::Set(set)) =
        find_block_value_mut(timeline, container_id, block_id)
        && let Some((_, current)) = set.parameters.get_mut(parameter_index)
      {
        *current = value;
      }
    }
    UiPropertiesCommand::AddParameter {
      container_id,
      block_id,
      index,
      value,
    } => {
      if let Some(TimelineBlockType::Set(set)) =
        find_block_value_mut(timeline, container_id, block_id)
        && index <= set.parameters.len()
      {
        set.parameters.insert(index, value);
      }
    }
    UiPropertiesCommand::RemoveParameter {
      container_id,
      block_id,
      index,
    } => {
      if let Some(TimelineBlockType::Set(set)) =
        find_block_value_mut(timeline, container_id, block_id)
        && index < set.parameters.len()
      {
        set.parameters.remove(index);
      }
    }
    UiPropertiesCommand::RenameContainer {
      container_id,
      value,
    } => {
      if let Some(container) = timeline
        .containers
        .iter_mut()
        .find(|c| c.id == container_id)
      {
        container.name = value;
      }
    }
  }
}

fn find_block_value_mut(
  state: &mut UiTimelineState,
  container_id: usize,
  block_id: usize,
) -> Option<&mut TimelineBlockType> {
  let container = state.containers.iter_mut().find(|c| c.id == container_id)?;
  let block = container.blocks.iter_mut().find(|b| b.id == block_id)?;
  Some(&mut block.value)
}
