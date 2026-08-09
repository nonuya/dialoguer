use std::{
  collections::{HashMap, HashSet, VecDeque},
  rc::Rc,
};

use anyhow::Context;
use dear_imgui_rs::*;
use dear_node_editor::*;
use log::{debug, warn};
use undoredo::UndoRedo;

use crate::timeline::Timeline;

#[derive(Clone)]
pub enum Command {
  AddNodeConversation(Node, Timeline),
  AddNodeChoicer(Node),
  RemoveNode {
    index: usize,
    node: Node,
    flow_links: Vec<Link>,
    links: Vec<Link>,
    timeline: Option<Timeline>,
  },
  AddLink {
    link: Link,
    replaced: Option<Link>,
  },
  AddFlowLink {
    link: Link,
    replaced: Option<Link>,
  },
  RemoveLink {
    index: usize,
    link: Link,
  },
  RemoveFlowLink {
    index: usize,
    link: Link,
  },
}
pub struct Graph {
  nodes: Vec<Node>,
  timelines: HashMap<NodeId, Timeline>,
  links: Vec<Link>,
  flow_links: Vec<Link>,
  flow_link_editing: bool,
  id_gen: IdGen,
  selected_node_id: Option<NodeId>,
  first_frame: bool,
  history: UndoRedo<(), Command>,
}

#[derive(Debug, Clone)]
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
  pub fn id(&self) -> NodeId {
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
#[derive(Clone)]
pub struct Link {
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
  pub fn new(dialog_mgr: &core::dialog::DialogManager) -> Self {
    let mut id_gen = IdGen::new();
    let (timelines, mut nodes, info_by_id) = Self::create_nodes(dialog_mgr, &mut id_gen);
    let (links, flow_links) = Self::create_links(dialog_mgr, &mut id_gen, info_by_id);

    let pin_to_node = Self::pin_to_node(&nodes);
    let groups = Self::connected_components(&nodes, &links, &pin_to_node);

    let mut cursor = [50.0, 50.0];

    for group in &groups {
      let order = Self::order_group(group, &links, &pin_to_node);
      cursor = Self::layout_group(&order, &mut nodes, cursor);
    }

    Self {
      selected_node_id: None,
      flow_link_editing: false,
      nodes,
      flow_links,
      history: UndoRedo::new(),
      timelines,
      links,
      id_gen,
      first_frame: true,
    }
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

    nodes.sort_unstable_by(|a, b| natord::compare(a.name(), b.name()));

    (timelines, nodes, info_by_id)
  }

  fn create_links(
    dialog_mgr: &core::dialog::DialogManager,
    id_gen: &mut IdGen,
    info_by_id: HashMap<Rc<str>, NodeInfo>,
  ) -> (Vec<Link>, Vec<Link>) {
    use core::dialog::{Dialog, Event};

    let mut links = Vec::new();
    let mut flow_links = Vec::new();

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
          match event {
            Event::Jump(target_id) => match info_by_id.get(target_id) {
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
            },
            Event::SetMainChoicer(target_id) => match info_by_id.get(target_id) {
              Some(target_info) => {
                flow_links.push(Link {
                  id: id_gen.next_link(),
                  from,
                  to: target_info.input,
                });
              }
              None => {
                warn!("SetMainChoicer apunta a ID inexistente {target_id}");
              }
            },
            _ => {}
          }
        }
      }
    }

    (links, flow_links)
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
    const GRAPH_GAP: f32 = 30.0;

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
        Node::Choicer { options, .. } => options.len() as f32 * 50.0,
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

  pub fn undo(&mut self) {
    if let Some(cmd) = self.history.undo_command() {
      match cmd {
        Command::AddNodeConversation(node, _) => {
          debug!("[UndoHistory] Undoing Add Node");
          self.timelines.remove(&node.id());
          self.nodes.pop();
        }
        Command::AddNodeChoicer(_) => {
          self.nodes.pop();
        }
        Command::RemoveNode {
          index,
          node,
          links,
          flow_links,
          timeline,
        } => {
          if let Some(timeline) = timeline {
            self.timelines.insert(node.id(), timeline);
          }
          self.nodes.insert(index, node);
          self.links.extend(links);
          self.flow_links.extend(flow_links);
        }
        Command::AddLink { link, replaced } => {
          self.links.retain(|l| l.id != link.id);
          if let Some(replaced) = replaced {
            self.links.push(replaced);
          }
        }
        Command::AddFlowLink { link, replaced } => {
          self.flow_links.retain(|l| l.id != link.id);
          if let Some(replaced) = replaced {
            self.flow_links.push(replaced);
          }
        }
        Command::RemoveLink { index, link } => {
          let index = index.min(self.links.len());
          self.links.insert(index, link);
        }
        Command::RemoveFlowLink { index, link } => {
          let index = index.min(self.flow_links.len());
          self.flow_links.insert(index, link);
        }
      }
    }
  }
  pub fn redo(&mut self) {
    if let Some(cmd) = self.history.redo_command() {
      match cmd {
        Command::AddNodeConversation(node, timeline) => {
          debug!("[UndoHistory] Redoing Add Node");
          assert!(self.timelines.insert(node.id(), timeline).is_none());
          self.nodes.push(node);
        }
        Command::AddNodeChoicer(node) => {
          self.nodes.push(node);
        }
        Command::RemoveNode {
          index,
          links,
          flow_links,
          timeline,
          node,
        } => {
          if timeline.is_some() {
            self.timelines.remove(&node.id());
          }

          self.nodes.remove(index);
          self.links.retain(|link| {
            !links
              .iter()
              .any(|removed| removed.from == link.from && removed.to == link.to)
          });
          self.flow_links.retain(|link| {
            !flow_links
              .iter()
              .any(|removed| removed.from == link.from && removed.to == link.to)
          });
        }
        Command::AddLink { link, replaced } => {
          if let Some(replaced) = replaced {
            self.links.retain(|l| l.id != replaced.id);
          }
          self.links.push(link);
        }
        Command::AddFlowLink { link, replaced } => {
          if let Some(replaced) = replaced {
            self.flow_links.retain(|l| l.id != replaced.id);
          }
          self.flow_links.push(link);
        }
        Command::RemoveLink { link, .. } => {
          self.links.retain(|l| l.id != link.id);
        }
        Command::RemoveFlowLink { link, .. } => {
          self.flow_links.retain(|l| l.id != link.id);
        }
      }
    }
  }
  fn add_conversation(&mut self, name: String) -> NodeId {
    let id = self.id_gen.next_node(); // Ignoring this because IDs have usize type so you have to put a lot of nodes
    let input = self.id_gen.next_pin();
    let output = self.id_gen.next_pin();

    let node = Node::Conversation {
      id,
      name,
      input,
      output,
      position: [0.0, 0.0],
    };

    assert!(
      self
        .timelines
        .insert(node.id(), Timeline::new_empty())
        .is_none()
    );
    self.nodes.push(node.clone());
    self
      .history
      .command(Command::AddNodeConversation(node, Timeline::new_empty()));

    id
  }

  fn add_choicer(&mut self, name: String) -> NodeId {
    let id = self.id_gen.next_node();
    let input = self.id_gen.next_pin();
    let options = Vec::new();
    let outputs = options.iter().map(|_| self.id_gen.next_pin()).collect();
    let node = Node::Choicer {
      id,
      name,
      options,
      input,
      outputs,
      position: [0.0, 0.0],
    };

    self.history.command(Command::AddNodeChoicer(node.clone()));
    self.nodes.push(node);

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
    current_dialog_name: Option<&Rc<str>>,
  ) {
    let mut pending_deleting = false;

    if !self.flow_link_editing {
      if ui.button("Add Conversation") {
        self.add_conversation(format!("Conversation {}", self.id_gen.0));
      }
      ui.same_line();
      if ui.button("Add Choicer") {
        self.add_choicer(format!("Choicer {}", self.id_gen.0));
      }

      if let Some(id) = self.selected_node_id {
        ui.same_line();

        if ui.button("(D)elete") {
          if let Some(index) = self.nodes.iter().position(|n| n.id() == id) {
            self.delete_node(index);
            pending_deleting = true;
          }
        }
      }

      ui.same_line();
      ui.separator_vertical();
    }

    ui.same_line();
    if ui.button(if !self.flow_link_editing {
      "Toggle Flow Link"
    } else {
      "Toggle Link"
    }) {
      self.flow_link_editing = !self.flow_link_editing;
    }

    self.draw_graph(ui, &node_editor, current_dialog_name, pending_deleting);
  }

  fn delete_node(&mut self, index: usize) {
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
    let node = self.nodes[index].clone();

    let links: Vec<Link> = self
      .links
      .iter()
      .filter(|link| pins.contains(&link.from) || pins.contains(&link.to))
      .cloned()
      .collect();

    let flow_links: Vec<Link> = self
      .flow_links
      .iter()
      .filter(|link| pins.contains(&link.from) || pins.contains(&link.to))
      .cloned()
      .collect();

    let timeline = self.timelines.get(&node.id()).cloned();

    self.timelines.remove(&node.id());
    self
      .links
      .retain(|link| !pins.contains(&link.from) && !pins.contains(&link.to));
    self
      .flow_links
      .retain(|link| !pins.contains(&link.from) && !pins.contains(&link.to));
    self.nodes.remove(index);

    self.history.command(Command::RemoveNode {
      index,
      node,
      links,
      flow_links,
      timeline,
    });
  }
  pub fn draw_graph(
    &mut self,
    ui: &Ui,
    node_editor: &EditorContext,
    current_dialog_name: Option<&Rc<str>>,
    pending_deleting: bool,
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
          ..
        } => {
          editor.node(*id, |node| {
            node.pin(*input, PinKind::Input, |_pin| {
              ui.text(" * ");
            });
            ui.same_line();
            if current_dialog_name.is_some_and(|current| current.as_ref() == name) {
              ui.text_colored([1.0, 0.0, 0.0, 1.0], name);
            } else {
              ui.text(name);
            }
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
          ..
        } => {
          editor.node(*id, |node| {
            node.pin(*input, PinKind::Input, |_pin| {
              ui.text(" * ");
            });
            ui.same_line();
            if current_dialog_name.is_some_and(|current| current.as_ref() == name) {
              ui.text_colored([1.0, 0.0, 0.0, 1.0], name);
            } else {
              ui.text(name);
            }

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
      let alpha = if self.flow_link_editing { 0.2 } else { 1.0 };
      editor.link_colored(link.id, link.from, link.to, [0.37, 0.72, 0.95, alpha], 2.5);
    }

    // --- Dibujar flow links existentes ---
    for link in &self.flow_links {
      let alpha = if !self.flow_link_editing { 0.3 } else { 1.0 };
      editor.link_colored(link.id, link.from, link.to, [1.0, 0.0, 0.0, alpha], 2.5);
    }

    // --- Eliminar opción pendiente (botón X) ---
    if let Some((node_id, option_idx)) = pending_delete_option {
      if let Some(Node::Choicer {
        options, outputs, ..
      }) = self.nodes.iter_mut().find(|n| n.id() == node_id)
      {
        let pin = outputs.remove(option_idx);
        options.remove(option_idx);
        if self.flow_link_editing {
          self
            .flow_links
            .retain(|link| link.from != pin && link.to != pin);
        } else {
          self.links.retain(|link| link.from != pin && link.to != pin);
        }
      }
    }

    // --- Crear links: máximo uno por pin de salida ---
    if let Some(create) = editor.begin_create([0.30, 0.85, 0.45, 1.0], 2.0) {
      if let Some((a, b)) = create.query_new_link() {
        if let Some((from, to)) = self.normalize_link(a, b) {
          if create.accept_new_item() {
            if self.flow_link_editing {
              let replaced = self
                .flow_links
                .iter()
                .position(|link| link.from == from)
                .map(|i| self.flow_links.remove(i));

              let link = Link {
                id: self.id_gen.next_link(),
                from,
                to,
              };
              self.flow_links.push(link.clone());
              self
                .history
                .command(Command::AddFlowLink { link, replaced });
            } else {
              let replaced = self
                .links
                .iter()
                .position(|link| link.from == from)
                .map(|i| self.links.remove(i));

              let link = Link {
                id: self.id_gen.next_link(),
                from,
                to,
              };
              self.links.push(link.clone());
              self.history.command(Command::AddLink { link, replaced });
            }
          }
        } else {
          create.reject_new_item();
        }
      }
    }

    if let Some(delete) = editor.begin_delete() {
      while let Some((link_id, _, _)) = delete.query_deleted_link() {
        if delete.accept_deleted_item(true) {
          if self.flow_link_editing {
            if let Some(index) = self.flow_links.iter().position(|l| l.id == link_id) {
              let link = self.flow_links.remove(index);
              self
                .history
                .command(Command::RemoveFlowLink { index, link });
            }
          } else if let Some(index) = self.links.iter().position(|l| l.id == link_id) {
            let link = self.links.remove(index);
            self.history.command(Command::RemoveLink { index, link });
          }
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

    if pending_deleting {
      editor.clear_selection();
    }

    let current_selection = match editor.selected_nodes().as_slice() {
      [id] => Some(*id),
      _ => None,
    };

    if self.selected_node_id != current_selection {
      self.selected_node_id = current_selection;
    }

    editor.end();
  }

  pub fn get_selected_node_id(&self) -> Option<NodeId> {
    self.selected_node_id
  }

  pub fn get_selected_node(&self) -> Option<&Node> {
    self.selected_node_id.and_then(|id| self.get_node_by_id(id))
  }

  pub fn get_mut_selected_node(&mut self) -> Option<&mut Node> {
    self
      .selected_node_id
      .and_then(|id| self.get_mut_node_by_id(id))
  }

  pub fn get_node_by_id(&self, id: NodeId) -> Option<&Node> {
    self.nodes.iter().find(|n| n.id() == id)
  }

  pub fn get_mut_node_by_id(&mut self, id: NodeId) -> Option<&mut Node> {
    self.nodes.iter_mut().find(|n| n.id() == id)
  }

  pub fn has_node_a_timeline(&self, id: NodeId) -> bool {
    self.timelines.contains_key(&id)
  }

  pub fn get_selected_timeline(&self) -> Option<&Timeline> {
    self.selected_node_id.and_then(|id| self.timelines.get(&id))
  }

  pub fn get_mut_selected_timeline(&mut self) -> Option<&mut Timeline> {
    self
      .selected_node_id
      .and_then(|id| self.timelines.get_mut(&id))
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

  pub fn get_node_by_name(&self, name: &Rc<str>) -> Option<&Node> {
    self.nodes.iter().find(|n| n.name() == name.as_ref())
  }

  pub fn export_to_dialog(
    &self,
    id: NodeId,
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
          id, name, output, ..
        } => {
          let timeline = self.timelines.get(id).context("Failed to get Timeline")?;
          let mut dialog = timeline.export_to_dialog();

          // Si tiene un nodo de salida
          if let Some(next) = self.follow(*output) {
            queue.push_back(next);
            match &mut dialog {
              Dialog::Conversation(nodes) => {
                nodes.push(DialogNode {
                  label: "Player".into(),
                  events: vec![Event::Jump(next.name().as_str().into())],
                });
              }
              Dialog::Choicer(_) => {
                unreachable!();
              }
            }
          }

          // Si tiene un flow nodo de salida
          if let Some(next) = self.flow_follow(*output) {
            match &mut dialog {
              Dialog::Conversation(nodes) => {
                nodes
                  .get_mut(0)
                  .context("Empty node")?
                  .events
                  .insert(0, Event::SetMainChoicer(next.name().as_str().into()));
              }
              Dialog::Choicer(_) => {
                unreachable!();
              }
            }
          }

          dialogs.push((name.as_str().into(), dialog));
        }
        Node::Choicer {
          name,
          options,
          outputs,
          ..
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

  fn flow_follow(&self, output: PinId) -> Option<&Node> {
    let to = self.flow_links.iter().find(|link| link.from == output)?.to;

    self.nodes.iter().find(|node| match node {
      Node::Conversation { input, .. } => *input == to,
      Node::Choicer { input, .. } => *input == to,
    })
  }
}
