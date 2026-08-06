use std::{
  collections::{HashMap, HashSet, VecDeque},
  rc::Rc,
};

use anyhow::Context;
use dear_imgui_rs::*;
use dear_node_editor::*;
use log::warn;

pub struct Graph {
  nodes: Vec<Node>,
  links: Vec<Link>,
  id_gen: IdGen,
  selected_node_id: Option<NodeId>,
  first_frame: bool,
}

#[derive(Debug)]
pub enum Node {
  Conversation {
    id: NodeId,
    name: String,
    input: PinId,
    output: PinId,
    position: [f32; 2],
  },
  Choicer {
    id: NodeId,
    name: String,
    options: Vec<String>,
    input: PinId,
    outputs: Vec<PinId>,
    position: [f32; 2],
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

  fn set_position(&mut self, pos: [f32; 2]) {
    match self {
      Node::Conversation { position, .. } => *position = pos,
      Node::Choicer { position, .. } => *position = pos,
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

struct NodeInfo {
  input: PinId,
  outputs: Vec<PinId>,
}

struct GraphGroup {
  nodes: Vec<NodeId>,
}

struct OrderedNode {
  id: NodeId,
  depth: usize,
}

impl Graph {
  pub fn new(
    dialog_mgr: &core::dialog::DialogManager,
  ) -> (HashMap<NodeId, crate::timeline::Timeline>, Self) {
    let mut id_gen = IdGen::new();
    let (timelines, mut nodes, info_by_id) = Self::create_nodes(dialog_mgr, &mut id_gen);
    let links = Self::create_links(dialog_mgr, &mut id_gen, info_by_id);

    let pin_to_node = Self::pin_to_node(&nodes);
    let groups = Self::connected_components(&nodes, &links, &pin_to_node);

    let mut cursor = [50.0, 50.0];

    for group in &groups {
      let order = Self::order_group(group, &links, &pin_to_node);
      cursor = Self::layout_group(&order, &mut nodes, cursor);
    }
    (
      timelines,
      Self {
        selected_node_id: None,
        nodes,
        links,
        id_gen,
        first_frame: true,
      },
    )
  }

  fn create_nodes(
    dialog_mgr: &core::dialog::DialogManager,
    id_gen: &mut IdGen,
  ) -> (
    HashMap<NodeId, crate::timeline::Timeline>,
    Vec<Node>,
    HashMap<Rc<str>, NodeInfo>,
  ) {
    use core::dialog::Dialog;
    let mut nodes = Vec::new();
    let mut timelines = HashMap::new();
    let mut info_by_id = HashMap::new();

    for (name, dialog) in dialog_mgr.get_dialogs() {
      match dialog {
        Dialog::Conversation(dialog_nodes) => {
          let node = Node::Conversation {
            id: id_gen.next_node(),
            name: name.to_string(),
            input: id_gen.next_pin(),
            output: id_gen.next_pin(),
            position: [0.0, 0.0],
          };

          let (input, output) = match &node {
            Node::Conversation { input, output, .. } => (*input, *output),
            _ => unreachable!(),
          };

          timelines.insert(node.id(), crate::timeline::Timeline::new(dialog_nodes));

          info_by_id.insert(
            name.clone(),
            NodeInfo {
              input,
              outputs: vec![output],
            },
          );
          nodes.push(node);
        }
        Dialog::Choicer(dialog_nodes) => {
          let input = id_gen.next_pin();
          let options: Vec<String> = dialog_nodes.iter().map(|n| n.label.to_string()).collect();
          let outputs: Vec<PinId> = dialog_nodes.iter().map(|_| id_gen.next_pin()).collect();

          let node = Node::Choicer {
            id: id_gen.next_node(),
            name: name.to_string(),
            options,
            input,
            outputs: outputs.clone(),
            position: [0.0, 0.0],
          };

          info_by_id.insert(name.clone(), NodeInfo { input, outputs });
          nodes.push(node);
        }
      }
    }

    (timelines, nodes, info_by_id)
  }

  fn create_links(
    dialog_mgr: &core::dialog::DialogManager,
    id_gen: &mut IdGen,
    info_by_id: HashMap<Rc<str>, NodeInfo>,
  ) -> Vec<Link> {
    use core::dialog::{Dialog, Event};

    let mut links = Vec::new();

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
              }
              None => {
                warn!("Jump apunta a ID inexistente {target_id}");
              }
            }
          }
        }
      }
    }

    links
  }

  fn pin_to_node(nodes: &Vec<Node>) -> HashMap<PinId, NodeId> {
    nodes
      .iter()
      .flat_map(|n| {
        let id = n.id();

        let mut pins = Vec::new();

        match n {
          Node::Conversation { input, output, .. } => {
            pins.push((*input, id));
            pins.push((*output, id));
          }

          Node::Choicer { input, outputs, .. } => {
            pins.push((*input, id));

            for p in outputs {
              pins.push((*p, id));
            }
          }
        }

        pins
      })
      .collect()
  }

  fn connected_components(
    nodes: &Vec<Node>,
    links: &Vec<Link>,
    pin_to_node: &HashMap<PinId, NodeId>,
  ) -> Vec<GraphGroup> {
    let mut adjacency: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

    for link in links {
      let from = pin_to_node[&link.from];
      let to = pin_to_node[&link.to];

      adjacency.entry(from).or_default().push(to);
      adjacency.entry(to).or_default().push(from);
    }

    let mut visited = HashSet::new();
    let mut groups = Vec::new();

    for node in nodes {
      let start = node.id();

      if !visited.insert(start) {
        continue;
      }

      let mut queue = VecDeque::from([start]);
      let mut group = Vec::new();

      while let Some(id) = queue.pop_front() {
        group.push(id);

        for &next in adjacency.get(&id).into_iter().flatten() {
          if visited.insert(next) {
            queue.push_back(next);
          }
        }
      }

      groups.push(GraphGroup { nodes: group });
    }

    groups
  }

  fn order_group(
    group: &GraphGroup,
    links: &[Link],
    pin_to_node: &HashMap<PinId, NodeId>,
  ) -> Vec<OrderedNode> {
    let node_set: HashSet<_> = group.nodes.iter().copied().collect();

    let mut outgoing: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    let mut indegree: HashMap<NodeId, usize> = HashMap::new();

    for &id in &group.nodes {
      indegree.insert(id, 0);
    }

    for link in links {
      let from = pin_to_node[&link.from];
      let to = pin_to_node[&link.to];

      if node_set.contains(&from) && node_set.contains(&to) {
        outgoing.entry(from).or_default().push(to);
        *indegree.get_mut(&to).unwrap() += 1;
      }
    }

    let root = group
      .nodes
      .iter()
      .find(|id| indegree[id] == 0)
      .copied()
      .unwrap_or(group.nodes[0]);

    let mut visited = HashSet::new();
    let mut order = Vec::new();

    fn dfs(
      node: NodeId,
      depth: usize,
      outgoing: &HashMap<NodeId, Vec<NodeId>>,
      visited: &mut HashSet<NodeId>,
      order: &mut Vec<OrderedNode>,
    ) {
      if !visited.insert(node) {
        return;
      }

      order.push(OrderedNode { id: node, depth });

      if let Some(children) = outgoing.get(&node) {
        for &child in children {
          dfs(child, depth + 1, outgoing, visited, order);
        }
      }
    }

    dfs(root, 0, &outgoing, &mut visited, &mut order);

    for &id in &group.nodes {
      if !visited.contains(&id) {
        dfs(id, 0, &outgoing, &mut visited, &mut order);
      }
    }

    order
  }

  fn layout_group(order: &[OrderedNode], nodes: &mut [Node], origin: [f32; 2]) -> [f32; 2] {
    const DX: f32 = 400.0;
    const DY: f32 = 40.0;
    const MAX_DEPTH_PER_BAND: usize = 10;
    const BAND_GAP: f32 = 30.0;
    const GRAPH_GAP: f32 = 100.0;

    let mut height_per_local_depth = HashMap::<usize, f32>::new();
    let mut max_band = 0;
    let mut max_offset = 0.0;

    for OrderedNode { id, depth } in order {
      let band = *depth / MAX_DEPTH_PER_BAND;
      let local_depth = *depth % MAX_DEPTH_PER_BAND;

      max_band = max_band.max(band);

      let y_offset = height_per_local_depth.entry(local_depth).or_default();

      let node = nodes.iter_mut().find(|n| n.id() == *id).unwrap();

      let extra_y = match node {
        Node::Choicer { options, .. } => {
          options.len() as f32 * 30.0
        }
        _ => 0.0,
      };

      let pos = [
        origin[0] + local_depth as f32 * DX,
        origin[1] + band as f32 * BAND_GAP + *y_offset,
      ];

      node.set_position(pos);

      *y_offset += DY + extra_y;
      max_offset = y_offset.max(max_offset);
    }

    [
      origin[0],
      origin[1] + (max_band + 1) as f32 * BAND_GAP + GRAPH_GAP + max_offset,
    ]
  }

  fn add_conversation(&mut self, name: String) -> NodeId {
    let id = self.id_gen.next_node();
    let input = self.id_gen.next_pin();
    let output = self.id_gen.next_pin();
    self.nodes.push(Node::Conversation {
      id,
      name,
      input,
      output,
      position: [0.0, 0.0],
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
      position: [0.0, 0.0],
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

    if self.first_frame {
      self.first_frame = false;

      for node in &self.nodes {
        match node {
          Node::Conversation { id, position, .. } => {
            editor.set_node_position(*id, *position);
          }
          Node::Choicer { id, position, .. } => {
            editor.set_node_position(*id, *position);
          }
        }
      }
    }

    for node in &mut self.nodes {
      match node {
        Node::Conversation {
          id,
          name,
          input,
          output,
          position,
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
          position,
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

  pub fn get_node_names(&self) -> Vec<&String> {
    self.nodes.iter().map(|n| n.name()).collect()
  }

  pub fn export_to_dialog(
    &self,
    id: NodeId,
    timelines: &HashMap<NodeId, crate::timeline::Timeline>,
  ) -> anyhow::Result<Vec<(Rc<str>, core::dialog::Dialog)>> {
    use core::dialog::{Dialog, DialogNode, Event};
    use std::collections::{HashSet, VecDeque};

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let node = self
      .get_node_by_id(id)
      .context("Failed to get Node by Id")?;
    let mut dialogs: Vec<(Rc<str>, Dialog)> = Vec::new();
    queue.push_back(node);

    while let Some(node) = queue.pop_front() {
      if visited.contains(&node.id()) {
        continue;
      }
      visited.insert(node.id());

      match node {
        Node::Conversation {
          id,
          name,
          input,
          output,
          position,
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
          position,
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
