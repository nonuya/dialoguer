use std::rc::Rc;

use dear_imgui_rs::*;

const BLOCK_WIDTH: f32 = 50.0;
const BLOCK_HEIGHT: f32 = 40.0;
const HEADER_HEIGHT: f32 = 22.0;
const CONTAINER_PADDING: f32 = 6.0;
const SELECTED_COLOR: [f32; 4] = [1.0, 1.0, 0.0, 1.0];
const CONTAINER_SPACING: f32 = 6.0;

pub enum TimelineBlockType {
  Set(TimelineSetBlockType),
  Text(String),
  Wait(f32),
  Next,
}

#[derive(Default)]
pub struct TimelineSetBlockType {
  pub parameters: Vec<(Rc<str>, Rc<str>)>, // EnumName ValueName
  pub anim: Rc<str>,
  pub view: Rc<str>,
  pub main_choicer: Rc<str>,
}

impl TimelineSetBlockType {
  pub fn is_empty(&self) -> bool {
    self.parameters.is_empty()
      && self.view.is_empty()
      && self.anim.is_empty()
      && self.main_choicer.is_empty()
  }
}

pub struct TimelineBlock {
  pub id: usize,
  pub value: TimelineBlockType,
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

pub struct Container {
  pub id: usize,
  pub name: String,
  pub blocks: Vec<TimelineBlock>,
}

impl Container {
  pub fn add_block(&mut self, value: TimelineBlockType) {
    self.blocks.push(TimelineBlock {
      id: self.blocks.len(),
      value,
    });
  }

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
          main_choicer,
        }) => {
          if !main_choicer.is_empty() {
            events.push(Event::SetMainChoicer(main_choicer.clone()));
          }

          if !view.is_empty() {
            events.push(Event::SetView(view.clone()));
          }

          if !anim.is_empty() {
            events.push(Event::SetAnim(anim.clone()));
          }

          for p in parameters {
            if p.1.as_ref() == "NonControl" {
              events.push(Event::RemoveParamater(p.0.clone()));
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
  offset_y: f32,
  current_y: f32,
}

pub enum Selection<'a> {
  Container(usize, &'a mut Container), // Index, Container
  Block(usize, &'a mut TimelineBlock), // ContainerID, Block
}

pub struct Timeline {
  containers: Vec<Container>,
  drag: Option<BlockDragState>,
  container_drag: Option<ContainerDragState>,
  selected_container_id: Option<usize>,
  // (container_id, block_id) — estable aunque se reordenen los Vec
  selected_block: Option<(usize, usize)>,
}

impl Timeline {
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
          Event::RemoveParamater(name) => pending_set
            .parameters
            .push((name.clone(), "NonControl".into())),
          Event::SetView(name) => pending_set.view = name.clone(),
          Event::SetMainChoicer(name) => pending_set.main_choicer = name.clone(),
          Event::SetAnim(name) => pending_set.anim = name.clone(),
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

      containers.push(Container {
        id,
        name: node.label.to_string(),
        blocks,
      });
    }

    Self {
      containers,
      drag: None,
      container_drag: None,
      selected_container_id: None,
      selected_block: None,
    }
  }

  pub fn new_empty() -> Self {
    Self {
      containers: Vec::new(),
      drag: None,
      container_drag: None,
      selected_container_id: None,
      selected_block: None,
    }
  }

  fn container_width(container: &Container) -> f32 {
    container.blocks.len().max(1) as f32 * BLOCK_WIDTH + CONTAINER_PADDING * 2.0
  }

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

  pub fn draw(&mut self, ui: &Ui, current_container_index: Option<usize>) {
    let origin = ui.cursor_screen_pos();
    let draw_list = ui.get_window_draw_list();

    let mouse_pos = ui.io().mouse_pos();
    let mouse_dragging = ui.is_mouse_dragging(MouseButton::Left);
    let mouse_released = ui.is_mouse_released(MouseButton::Left);

    let row_height = HEADER_HEIGHT + BLOCK_HEIGHT + CONTAINER_PADDING * 2.0;
    let row_stride = row_height + CONTAINER_SPACING; // alto de fila + separación vertical

    let avail_width = ui.content_region_avail()[0];

    // widths sigue viniendo de compute_layout (ancho "natural" de cada container).
    let (_normal_lefts, widths) = self.compute_layout(origin[0]);

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

    for container_idx in 0..self.containers.len() {
      let container_id = self.containers[container_idx].id;
      let container_width = widths[container_idx];

      let is_dragging_container = matches!(
        &self.container_drag,
        Some(d) if d.container_idx == container_idx
      );

      let is_container_highlighted = self.selected_block.is_none()
        && matches!(&self.selected_container_id, Some(id) if *id == container_id);

      let (container_left, container_top) = if is_dragging_container {
        let d = self.container_drag.as_ref().unwrap();
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
        self.selected_container_id = Some(container_id);
        self.selected_block = None;
      }

      if header_active && self.container_drag.is_none() && self.drag.is_none() {
        self.container_drag = Some(ContainerDragState {
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
        &self.containers[container_idx].name,
      );

      let track_x = container_left + CONTAINER_PADDING;
      let track_y = container_top + HEADER_HEIGHT + CONTAINER_PADDING;

      // --- Bloques dentro del container: siguen en línea horizontal, sin cambios ---
      let block_count = self.containers[container_idx].blocks.len();
      for block_idx in 0..block_count {
        let block_id = self.containers[container_idx].blocks[block_idx].id;
        let slot_x = track_x + block_idx as f32 * BLOCK_WIDTH;

        let is_dragging_block = matches!(
          &self.drag,
          Some(d) if d.container_idx == container_idx && d.block_idx == block_idx
        );

        let is_block_selected = matches!(
          &self.selected_block,
          Some((cid, bid)) if *cid == container_id && *bid == block_id
        );

        let block_screen_pos = if is_dragging_block {
          [self.drag.as_ref().unwrap().current_x, track_y]
        } else {
          [slot_x, track_y]
        };

        let block = &self.containers[container_idx].blocks[block_idx];
        let button_id = format!("##{}_{}", container_id, block.id);

        ui.set_cursor_screen_pos(block_screen_pos);
        ui.invisible_button(&button_id, [BLOCK_WIDTH - 2.0, BLOCK_HEIGHT]);

        let is_active = ui.is_item_active();

        if ui.is_item_clicked() {
          self.selected_container_id = Some(container_id);
          self.selected_block = Some((container_id, block_id));
        }

        if is_active && self.drag.is_none() && self.container_drag.is_none() {
          self.drag = Some(BlockDragState {
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
    if let Some(drag) = &mut self.drag {
      if mouse_dragging {
        let track_x = lefts[drag.container_idx] + CONTAINER_PADDING;
        let container = &self.containers[drag.container_idx];
        let track_max_x = track_x + (container.blocks.len() as f32 - 1.0) * BLOCK_WIDTH;
        drag.current_x = (mouse_pos[0] - drag.offset_x).clamp(track_x, track_max_x);
      }
    }

    // --- Actualizar posición del CONTAINER en drag: ahora libre en X e Y ---
    if let Some(drag) = &mut self.container_drag {
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
      if let Some(drag) = self.drag.take() {
        let track_x = lefts[drag.container_idx] + CONTAINER_PADDING;
        let container = &mut self.containers[drag.container_idx];
        let relative_x = drag.current_x - track_x;

        let mut target_index = (relative_x / BLOCK_WIDTH).round() as isize;
        target_index = target_index.clamp(0, container.blocks.len() as isize - 1);

        let moved_block = container.blocks.remove(drag.block_idx);
        container.blocks.insert(target_index as usize, moved_block);
      }
    }

    // --- Soltar CONTAINER: reordenar considerando fila (Y) y columna (X) ---
    if mouse_released {
      if let Some(drag) = self.container_drag.take() {
        let dragged_row = ((drag.current_y - origin[1]) / row_stride)
          .round()
          .clamp(0.0, max_row as f32) as usize;
        let trigger_x = drag.current_x + widths[drag.container_idx] * 0.5;

        let mut target_index = 0;
        for i in 0..self.containers.len() {
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

        let moved_container = self.containers.remove(drag.container_idx);
        self.containers.insert(target_index, moved_container);
      }
    }
  }
  pub fn get_selected(&mut self) -> Option<Selection<'_>> {
    if let Some((container_id, block_id)) = self.selected_block {
      let container_idx = self.containers.iter().position(|c| c.id == container_id)?;
      let block_idx = self.containers[container_idx]
        .blocks
        .iter()
        .position(|b| b.id == block_id)?;

      return Some(Selection::Block(
        container_id,
        &mut self.containers[container_idx].blocks[block_idx],
      ));
    } else if let Some(container_id) = self.selected_container_id {
      let container_idx = self.containers.iter().position(|c| c.id == container_id)?;
      return Some(Selection::Container(
        container_idx,
        &mut self.containers[container_idx],
      ));
    } else {
      return None;
    }
  }

  pub fn push_back_container(&mut self, name: String) {
    self.containers.push(Container {
      id: self.containers.len(),
      name,
      blocks: Vec::new(),
    });
  }

  pub fn delete_container(&mut self, id: usize) {
    if let Some(idx) = self.containers.iter().position(|c| c.id == id) {
      self.containers.remove(idx);
    }
  }

  pub fn delete_block(&mut self, container_id: usize, block_id: usize) {
    if let Some(container_idx) = self.containers.iter().position(|c| c.id == container_id) {
      let container = &mut self.containers[container_idx];

      if let Some(idx) = container.blocks.iter().position(|b| b.id == block_id) {
        container.blocks.remove(idx);
      }
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
