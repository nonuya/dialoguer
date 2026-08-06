use std::{collections::HashMap, rc::Rc};

use anyhow::Context;
use dear_imgui_rs::*;
use dear_node_editor::*;
use log::warn;

pub struct Graph {
  nodes: Vec<Node>,
  links: Vec<Link>,
  id_gen: IdGen,
  selected_node_id: Option<NodeId>,
}

#[derive(Debug)]
pub enum Node {
  Conversation {
    id: NodeId,
    name: String,
    input: PinId,
    output: PinId,
  },
  Choicer {
    id: NodeId,
    name: String,
    options: Vec<String>,
    input: PinId,
    outputs: Vec<PinId>,
  },
}

impl Node {
  fn id(&self) -> NodeId {
    match self {
      Node::Conversation { id, .. } => *id,
      Node::Choicer { id, .. } => *id,
    }
  }

  pub fn name(&self) -> &String {
    match self {
      Node::Conversation { name, .. } => name,
      Node::Choicer { name, .. } => name,
    }
  }
}

// FIXME: Cambiar a HashMap
struct Link {
  id: LinkId,
  from: PinId,
  to: PinId,
}

/// Generador de ids simple: NodeId/PinId/LinkId comparten el mismo espacio
/// de enteros en imgui-node-editor, así que un único contador alcanza y
/// garantiza que nunca colisionen entre sí.
struct IdGen(usize);

impl IdGen {
  fn new() -> Self {
    Self(1)
  }

  fn next_node(&mut self) -> NodeId {
    let v = self.0;
    self.0 += 1;
    NodeId::new(v)
  }

  fn next_pin(&mut self) -> PinId {
    let v = self.0;
    self.0 += 1;
    PinId::new(v)
  }

  fn next_link(&mut self) -> LinkId {
    let v = self.0;
    self.0 += 1;
    LinkId::new(v)
  }
}

impl Graph {
  pub fn new(
    dialog_mgr: &core::dialog::DialogManager,
  ) -> (HashMap<NodeId, crate::timeline::Timeline>, Self) {
    use core::dialog::Dialog;

    let mut timelines = HashMap::new();
    let mut nodes = Vec::new();
    let mut links = Vec::new();
    let mut id_gen = IdGen::new();

    let dialog = dialog_mgr.get_dialog_by_id("Phase01.5").unwrap();
    let node = Node::Conversation {
      id: id_gen.next_node(),
      name: "Phase01.5".into(),
      input: id_gen.next_pin(),
      output: id_gen.next_pin(),
    };
    match dialog {
      Dialog::Conversation(dialog_nodes) => {
        timelines.insert(node.id(), crate::timeline::Timeline::new(dialog_nodes));
        nodes.push(node);
      }
      Dialog::Choicer(dialog_nodes) => {
        unimplemented!()
      }
    }

    (
      timelines,
      Self {
        selected_node_id: None,
        nodes,
        links,
        id_gen,
      },
    )
  }

  /*
  pub fn new(
    dialog_mgr: &core::dialog::DialogManager,
  ) -> (HashMap<NodeId, crate::timeline::Timeline>, Self) {
    use core::dialog::{Dialog, Event};
    use std::collections::HashSet;

    let mut timelines = HashMap::new();
    let mut nodes = Vec::new();
    let mut links = Vec::new();
    let mut id_gen = IdGen::new();

    struct NodeInfo {
      input: PinId,
      outputs: Vec<PinId>,
    }
    let mut info_by_id: HashMap<String, NodeInfo> = HashMap::new();

    // --- 1ra pasada: crear Node con posición placeholder (se ajusta después) ---

    // --- 2da pasada: crear los Link a partir de los Jump + adjacency dirigida y no dirigida ---
    // directed: name -> target (para las capas dentro de un grupo)
    // undirected: ambos sentidos (para detectar componentes conexas)
    let mut directed: HashMap<String, Vec<String>> =
      info_by_id.keys().map(|k| (k.clone(), Vec::new())).collect();
    let mut undirected: HashMap<String, Vec<String>> =
      info_by_id.keys().map(|k| (k.clone(), Vec::new())).collect();

    for (name, dialog) in dialog_mgr.get_dialogs() {
      let dialog_nodes: &Vec<core::dialog::DialogNode> = match dialog {
        Dialog::Conversation(dn) => dn,
        Dialog::Choicer(dn) => dn,
      };

      for (i, dialog_node) in dialog_nodes.iter().enumerate() {
        let from = match dialog {
          Dialog::Conversation(_) => info_by_id[name].outputs[0],
          Dialog::Choicer(_) => info_by_id[name].outputs[i],
        };

        for event in &dialog_node.events {
          if let Event::Jump(target_id) = event {
            match info_by_id.get(target_id) {
              Some(target_info) => {
                links.push(Link {
                  id: id_gen.next_link(),
                  from,
                  to: target_info.input,
                });

                if !directed[name].iter().any(|t| t == target_id) {
                  directed.get_mut(name).unwrap().push(target_id.clone());
                }
                if !undirected[name].iter().any(|t| t == target_id) {
                  undirected.get_mut(name).unwrap().push(target_id.clone());
                }
                if !undirected[target_id].iter().any(|t| t == name) {
                  undirected.get_mut(target_id).unwrap().push(name.clone());
                }
              }
              None => {
                warn!("Jump apunta a ID inexistente {target_id}");
              }
            }
          }
        }
      }
    }

    // --- Agrupar por componente conexa (BFS no dirigido), respetando orden de inserción ---
    let insertion_order: Vec<_> = dialog_mgr
      .get_dialogs()
      .iter()
      .map(|(name, _)| *name)
      .collect();

    let mut visited: HashSet<String> = HashSet::new();
    let mut groups: Vec<Vec<String>> = Vec::new();

    for start in insertion_order {
      if visited.contains(start) {
        continue;
      }

      let mut group = Vec::new();
      let mut queue: VecDeque<String> = VecDeque::from([start.clone()]);
      visited.insert(start.clone());

      while let Some(current) = queue.pop_front() {
        group.push(current.clone());
        for neighbor in &undirected[&current] {
          if visited.insert(neighbor.clone()) {
            queue.push_back(neighbor.clone());
          }
        }
      }

      groups.push(group);
    }

    // --- Reordenar el Vec<Node> por grupo, con capas dirigidas dentro de cada grupo ---
    let mut nodes_by_name: HashMap<String, Node> =
      nodes.into_iter().map(|n| (n.name().clone(), n)).collect();

    let mut nodes: Vec<Node> = Vec::with_capacity(nodes_by_name.len());

    let mut cursor_x = 0.0_f32;
    let mut cursor_y = 0.0_f32;
    let mut row_max_height = 0.0_f32;

    for group in &groups {
      let group_set: HashSet<&String> = group.iter().collect();

      let mut in_degree_local: HashMap<&String, usize> = group.iter().map(|n| (n, 0)).collect();
      for name in group {
        for target in &directed[name] {
          if group_set.contains(target) {
            *in_degree_local.get_mut(target).unwrap() += 1;
          }
        }
      }

      let mut layer_by_name: HashMap<&String, i32> = group.iter().map(|n| (n, 0)).collect();
      let mut remaining = in_degree_local.clone();

      let mut local_queue: Vec<&String> =
        group.iter().filter(|n| in_degree_local[*n] == 0).collect();
      local_queue.sort_by_key(|n| group.iter().position(|x| x == *n).unwrap());
      let mut local_queue: VecDeque<&String> = local_queue.into();

      let mut layered_order: Vec<&String> = Vec::with_capacity(group.len());

      while let Some(name) = local_queue.pop_front() {
        layered_order.push(name);
        let current_layer = layer_by_name[name];

        for target in &directed[name] {
          if !group_set.contains(target) {
            continue;
          }
          let deg = remaining.get_mut(target).unwrap();
          *deg -= 1;

          let target_layer = layer_by_name.get_mut(target).unwrap();
          *target_layer = (*target_layer).max(current_layer + 1);

          if *deg == 0 {
            local_queue.push_back(target);
          }
        }
      }

      if layered_order.len() < group.len() {
        for name in group {
          if !layered_order.contains(&name) {
            layered_order.push(name);
          }
        }
      }

      // --- Calcular tamaño del grupo ANTES de decidir el cursor ---
      let mut slot_per_layer: HashMap<i32, f32> = HashMap::new();
      let mut max_layer = 0;
      for name in &layered_order {
        let layer = layer_by_name[*name];
        max_layer = max_layer.max(layer);
        *slot_per_layer.entry(layer).or_insert(0.0) += 1.0;
      }
      let group_width = (max_layer as f32 + 1.0) * NODE_SPACING_X;
      let group_height = slot_per_layer.values().cloned().fold(0.0_f32, f32::max) * NODE_SPACING_Y;

      // --- Salto de fila ANTES de posicionar ---
      if cursor_x > 0.0 && cursor_x + group_width > MAX_ROW_WIDTH {
        cursor_x = 0.0;
        cursor_y += row_max_height + GROUP_SPACING_Y;
        row_max_height = 0.0;
      }

      // --- Ahora sí, posicionar cada nodo con el cursor ya corregido ---
      let mut slot_per_layer: HashMap<i32, f32> = HashMap::new();
      for name in &layered_order {
        let Some(mut node) = nodes_by_name.remove(*name) else {
          continue;
        };

        let layer = layer_by_name[*name];
        let slot = slot_per_layer.entry(layer).or_insert(0.0);
        let y = *slot;
        *slot += 1.0;

        node.set_position([
          cursor_x + layer as f32 * NODE_SPACING_X,
          cursor_y + y * NODE_SPACING_Y,
        ]);
        nodes.push(node);
      }

      cursor_x += group_width + GROUP_SPACING_X;
      row_max_height = row_max_height.max(group_height);
    }

    (
      timelines,
      Self {
        first_frame: true,
        selected_node_id: None,
        nodes,
        links,
        id_gen,
      },
    )
  }
  */

  fn add_conversation(&mut self, name: String) -> NodeId {
    let id = self.id_gen.next_node();
    let input = self.id_gen.next_pin();
    let output = self.id_gen.next_pin();
    self.nodes.push(Node::Conversation {
      id,
      name,
      input,
      output,
    });
    id
  }

  fn add_choicer(&mut self, name: String) -> NodeId {
    let id = self.id_gen.next_node();
    let input = self.id_gen.next_pin();
    let options = Vec::new();
    let outputs = options.iter().map(|_| self.id_gen.next_pin()).collect();
    self.nodes.push(Node::Choicer {
      id,
      name,
      options,
      input,
      outputs,
    });
    id
  }

  /// Agrega una opción nueva a un Choicer existente, con su pin de salida.
  fn add_choicer_option(&mut self, node_id: NodeId, label: String) {
    let pin = self.id_gen.next_pin();
    if let Some(Node::Choicer {
      options, outputs, ..
    }) = self.nodes.iter_mut().find(|n| n.id() == node_id)
    {
      options.push(label);
      outputs.push(pin);
    }
  }

  pub fn draw(
    &mut self,
    ui: &Ui,
    node_editor: &EditorContext,
    on_add_node: impl FnOnce(NodeId) -> (),
    on_selected_node: impl FnOnce(Option<NodeId>) -> (),
    on_deleted_node: impl FnOnce(NodeId) -> (),
  ) {
    if ui.button("Add Conversation (Z)") || (ui.io().key_alt() && ui.is_key_pressed(Key::Z)) {
      on_add_node(self.add_conversation(format!("Conversation {}", self.id_gen.0)));
    }
    ui.same_line();
    if ui.button("Add Choicer (X)") || (ui.io().key_alt() && ui.is_key_pressed(Key::X)) {
      self.add_choicer(format!("Choicer {}", self.id_gen.0));
    }

    let mut pending_deleting = None;

    if let Some(id) = self.selected_node_id {
      ui.same_line();

      if ui.button("(D)elete") || (ui.io().key_alt() && ui.is_key_pressed(Key::D)) {
        if let Some(index) = self.nodes.iter().position(|n| n.id() == id) {
          let pins: Vec<PinId> = match &self.nodes[index] {
            Node::Conversation { input, output, .. } => {
              vec![*input, *output]
            }
            Node::Choicer { input, outputs, .. } => {
              let mut pins = Vec::with_capacity(outputs.len() + 1);
              pins.push(*input);
              pins.extend(outputs.iter().copied());
              pins
            }
          };

          self
            .links
            .retain(|link| !pins.contains(&link.from) && !pins.contains(&link.to));

          pending_deleting = Some(index);
        }
      }
    }

    self.draw_graph(
      ui,
      &node_editor,
      pending_deleting,
      on_selected_node,
      on_deleted_node,
    );
  }

  pub fn draw_graph(
    &mut self,
    ui: &Ui,
    node_editor: &EditorContext,
    mut pending_deleting: Option<usize>,
    on_selected_node: impl FnOnce(Option<NodeId>) -> (),
    on_deleted_node: impl FnOnce(NodeId) -> (),
  ) {
    let editor = ui.node_editor(node_editor, "DialogueGraph", [0.0, 0.0]);
    let mut pending_new_option: Option<NodeId> = None;
    let mut pending_delete_option: Option<(NodeId, usize)> = None;

    for node in &mut self.nodes {
      match node {
        Node::Conversation {
          id,
          name,
          input,
          output,
        } => {
          editor.node(*id, |node| {
            node.pin(*input, PinKind::Input, |_pin| {
              ui.text(" * ");
            });
            ui.same_line();
            ui.text(name);
            ui.same_line();
            node.pin(*output, PinKind::Output, |_pin| {
              ui.text(" * ");
            });
          });
        }
        Node::Choicer {
          id,
          name,
          options,
          input,
          outputs,
        } => {
          editor.node(*id, |node| {
            node.pin(*input, PinKind::Input, |_pin| {
              ui.text(" * ");
            });
            ui.same_line();
            ui.text(name);

            for (idx, (opt, pin_id)) in options.iter_mut().zip(outputs.iter()).enumerate() {
              let _id = ui.push_id(pin_id.0);
              if ui.button("X") {
                pending_delete_option = Some((*id, idx));
              }
              ui.same_line();
              ui.set_next_item_width(300.0);
              ui.input_text("##option", opt).build();
              ui.same_line();
              node.pin(*pin_id, PinKind::Output, |_pin| {
                ui.text(" * ");
              });
            }

            let _id = ui.push_id(format!("##add_{}", id.0).as_str());
            if ui.button_with_size("+", [50.0, 0.0]) {
              pending_new_option = Some(*id);
            }
          });
        }
      }
    }

    // --- Dibujar links existentes ---
    for link in &self.links {
      editor.link_colored(link.id, link.from, link.to, [0.37, 0.72, 0.95, 1.0], 2.5);
    }

    // --- Eliminar opción pendiente (botón X) ---
    if let Some((node_id, option_idx)) = pending_delete_option {
      if let Some(Node::Choicer {
        options, outputs, ..
      }) = self.nodes.iter_mut().find(|n| n.id() == node_id)
      {
        let pin = outputs.remove(option_idx);
        options.remove(option_idx);
        self.links.retain(|link| link.from != pin && link.to != pin);
      }
    }

    // --- Crear links: máximo uno por pin de salida ---
    if let Some(create) = editor.begin_create([0.30, 0.85, 0.45, 1.0], 2.0) {
      if let Some((a, b)) = create.query_new_link() {
        if let Some((from, to)) = self.normalize_link(a, b) {
          if create.accept_new_item() {
            // Un pin de salida solo puede tener un link activo: si ya
            // existía uno desde ese `from`, lo reemplaza.
            self.links.retain(|link| link.from != from);
            self.links.push(Link {
              id: self.id_gen.next_link(),
              from,
              to,
            });
          }
        } else {
          create.reject_new_item();
        }
      }
    }

    // --- Eliminar links con click derecho / tecla Delete sobre el link ---
    if let Some(delete) = editor.begin_delete() {
      while let Some((link_id, _, _)) = delete.query_deleted_link() {
        if delete.accept_deleted_item(true) {
          self.links.retain(|link| link.id != link_id);
        }
      }
      while delete.query_deleted_node().is_some() {
        delete.reject_deleted_item();
      }
    }

    // --- Aplicar el "+" pendiente después de terminar el frame del editor ---
    if let Some(node_id) = pending_new_option {
      self.add_choicer_option(node_id, "Text".into());
    }

    if let Some(index) = pending_deleting.take() {
      on_deleted_node(self.nodes[index].id());
      self.nodes.remove(index);
      self.selected_node_id = None;
      editor.clear_selection();
    }

    let current_selection = match editor.selected_nodes().as_slice() {
      [id] => Some(*id),
      _ => None,
    };

    if self.selected_node_id != current_selection {
      self.selected_node_id = current_selection;
      on_selected_node(current_selection);
    }

    editor.end();
  }

  pub fn get_node_by_id(&self, id: NodeId) -> Option<&Node> {
    self.nodes.iter().find(|n| n.id() == id)
  }

  pub fn get_mut_node_by_id(&mut self, id: NodeId) -> Option<&mut Node> {
    self.nodes.iter_mut().find(|n| n.id() == id)
  }

  /// Determina cuál de los dos pines involucrados en un drag es el de
  /// salida y cuál el de entrada, buscando en todos los nodos. Devuelve
  /// `None` si el par no es válido (ambos del mismo tipo, o no encontrados).
  fn normalize_link(&self, a: PinId, b: PinId) -> Option<(PinId, PinId)> {
    let kind_of = |pin: PinId| -> Option<PinKind> {
      for node in &self.nodes {
        match node {
          Node::Conversation { input, output, .. } => {
            if *input == pin {
              return Some(PinKind::Input);
            }
            if *output == pin {
              return Some(PinKind::Output);
            }
          }
          Node::Choicer { input, outputs, .. } => {
            if *input == pin {
              return Some(PinKind::Input);
            }
            if outputs.contains(&pin) {
              return Some(PinKind::Output);
            }
          }
        }
      }
      None
    };

    match (kind_of(a), kind_of(b)) {
      (Some(PinKind::Output), Some(PinKind::Input)) => Some((a, b)),
      (Some(PinKind::Input), Some(PinKind::Output)) => Some((b, a)),
      _ => None, // mismo tipo en ambos extremos, o pin desconocido: link inválido
    }
  }

  pub fn export_to_dialog(
    &self,
    id: NodeId,
    timelines: &HashMap<NodeId, crate::timeline::Timeline>,
  ) -> anyhow::Result<Vec<(Rc<str>, core::dialog::Dialog)>> {
    use core::dialog::{Dialog, DialogNode, Event};
    use std::collections::VecDeque;

    let mut queue = VecDeque::new();
    let node = self
      .get_node_by_id(id)
      .context("Failed to get Node by Id")?;
    let mut dialogs: Vec<(Rc<str>, Dialog)> = Vec::new();
    queue.push_back(node);

    while let Some(node) = queue.pop_front() {
      match node {
        Node::Conversation {
          id,
          name,
          input,
          output,
        } => {
          let timeline = timelines.get(id).context("Failed to get Timeline")?;
          let mut dialog = timeline.export_to_dialog();
          if let Some(next) = self.follow(*output) {
            queue.push_back(next);
            match &mut dialog {
              Dialog::Conversation(nodes) => {
                nodes.push(DialogNode {
                  label: "Player".into(),
                  events: vec![Event::Jump(Rc::from(next.name().as_str()))],
                });
              }
              Dialog::Choicer(_) => {
                unreachable!();
              }
            }
          }

          dialogs.push((name.as_str().into(), dialog));
        }
        Node::Choicer {
          id,
          name,
          options,
          input,
          outputs,
        } => {
          let mut valid_choices: Vec<DialogNode> = Vec::new();
          for (option, output) in options.iter().zip(outputs) {
            match self.follow(*output) {
              Some(next) => {
                valid_choices.push(DialogNode {
                  label: Rc::from(option.as_str()),
                  events: vec![Event::Jump(Rc::from(next.name().as_str()))],
                });
                queue.push_back(next);
              }
              None => {
                warn!("Skipping '{}' because it doesnt have an output node.", name);
              }
            }
          }
          dialogs.push((name.as_str().into(), Dialog::Choicer(valid_choices)));
        }
      }
    }

    Ok(dialogs)
  }

  fn follow(&self, output: PinId) -> Option<&Node> {
    let to = self.links.iter().find(|link| link.from == output)?.to;

    self.nodes.iter().find(|node| match node {
      Node::Conversation { input, .. } => *input == to,
      Node::Choicer { input, .. } => *input == to,
    })
  }
}
