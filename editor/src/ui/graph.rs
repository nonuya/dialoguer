use anyhow::Context;
use dear_node_editor::*;
use std::{
  collections::{HashMap, HashSet, VecDeque},
  rc::Rc,
};

use crate::ui::{
  UiState,
  editor::{IdGen, UiCommand, UiOutput},
  timeline::UiTimelineState,
};

impl IdGen {
  fn next_node(&mut self) -> NodeId {
    NodeId::new(self.next())
  }

  fn next_pin(&mut self) -> PinId {
    PinId::new(self.next())
  }

  fn next_link(&mut self) -> LinkId {
    LinkId::new(self.next())
  }
}

#[derive(Debug, Clone)]
pub enum Node {
  Conversation {
    id: NodeId,
    name: String,
    input: PinId,
    output: PinId,
    position: [f32; 2],
    dirty: bool,
  },
  Choicer {
    id: NodeId,
    name: String,
    options: Vec<String>,
    input: PinId,
    outputs: Vec<PinId>,
    position: [f32; 2],
    dirty: bool,
  },
}

impl Node {
  pub fn id(&self) -> NodeId {
    match self {
      Node::Conversation { id, .. } => *id,
      Node::Choicer { id, .. } => *id,
    }
  }

  pub fn dirty(&self) -> bool {
    match self {
      Node::Conversation { dirty, .. } => *dirty,
      Node::Choicer { dirty, .. } => *dirty,
    }
  }

  pub fn name(&self) -> &String {
    match self {
      Node::Conversation { name, .. } => name,
      Node::Choicer { name, .. } => name,
    }
  }

  pub fn set_position(&mut self, pos: [f32; 2]) {
    match self {
      Node::Conversation { position, .. } => *position = pos,
      Node::Choicer { position, .. } => *position = pos,
    }
  }
}

#[derive(Clone)]
pub struct Link {
  id: LinkId,
  from: PinId,
  to: PinId,
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

pub struct UiGraphState {
  nodes: Vec<Node>,
  selected_node_id: Option<NodeId>,
  timeline_states: HashMap<NodeId, UiTimelineState>,
  links: Vec<Link>,
  flow_links: Vec<Link>,
  flow_link_editing: bool,
}

impl UiGraphState {
  pub fn new(dialog_mgr: &core::dialog::DialogManager, id_gen: &mut IdGen) -> Self {
    let (timeline_states, mut nodes, info_by_id) = Self::create_nodes(dialog_mgr, id_gen);
    let (links, flow_links) = Self::create_links(dialog_mgr, id_gen, info_by_id);

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
      links,
      flow_links,
      timeline_states,
    }
  }

  fn create_nodes(
    dialog_mgr: &core::dialog::DialogManager,
    id_gen: &mut IdGen,
  ) -> (
    HashMap<NodeId, UiTimelineState>,
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
            dirty: true,
          };

          let (input, output) = match &node {
            Node::Conversation { input, output, .. } => (*input, *output),
            _ => unreachable!(),
          };

          timelines.insert(node.id(), UiTimelineState::new(dialog_nodes));

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
            dirty: true,
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
                log::warn!("Jump apunta a ID inexistente {target_id}");
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
                log::warn!("SetMainChoicer apunta a ID inexistente {target_id}");
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

  pub fn get_selected_node(&self) -> Option<&Node> {
    self.selected_node_id.and_then(|id| self.get_node_by_id(id))
  }

  pub fn get_selected_node_id(&self) -> Option<NodeId> {
    self.selected_node_id
  }

  fn get_node_by_id(&self, id: NodeId) -> Option<&Node> {
    self.nodes.iter().find(|n| n.id() == id)
  }

  pub fn get_node_by_name(&self, name: &Rc<str>) -> Option<&Node> {
    self.nodes.iter().find(|n| n.name() == name.as_ref())
  }

  fn get_mut_selected_node(&mut self) -> Option<&mut Node> {
    self
      .selected_node_id
      .and_then(|id| self.get_mut_node_by_id(id))
  }

  fn get_mut_node_by_id(&mut self, id: NodeId) -> Option<&mut Node> {
    self.nodes.iter_mut().find(|n| n.id() == id)
  }

  pub fn get_mut_selected_timeline(&mut self) -> Option<&mut UiTimelineState> {
    self
      .selected_node_id
      .and_then(|id| self.timeline_states.get_mut(&id))
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
          let timeline = self
            .timeline_states
            .get(id)
            .context("Failed to get Timeline")?;
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
                log::warn!(
                  "[Export] Skipping '{}' because it doesnt have an output node.",
                  name
                );
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

pub enum UiGraphCommand {
  AddConversation { name: String, position: [f32; 2] },
  AddChoicer { name: String, position: [f32; 2] },
  RemoveNode { nodes_idx: usize },
  ToggleLinkEdit,
  RemoveChoicerOption { node_id: NodeId, option_idx: usize },
  AddChoicerOption { node_id: NodeId },
  AddLink { from: PinId, to: PinId },
  AddFlowLink { from: PinId, to: PinId },
  RemoveLink { links_idx: usize },
  RemoveFlowLink { flow_links_idx: usize },
}

pub fn draw_graph(ui: &dear_imgui_rs::Ui, state: &mut UiState) -> UiOutput {
  let mut output = UiOutput::default();

  // ==========================
  // Toolbar
  // ==========================
  if let Some(node) = state.graph_state.get_selected_node() {
    if ui.button("Play") {
      output.commands.push(UiCommand::PlayDialog { skip: 0 });

      /*
      match self.export_to_dialog(node.id()) {
        Ok(dialogs) => {
          let name = self.get_node_by_id(node.id()).unwrap().name();
          let dialog_mgr = DialogManager::from_entries(dialogs);
          let dialog_player = {
            let initial_dialog = dialog_mgr.build(name).unwrap();
            DialogPlayer::new(initial_dialog)
          };
          playback_cmd = Some(PlaybackCmd::Play {
            dialog_mgr,
            dialog_player,
          });
        }
        Err(e) => warn!("[Graph] Failed to Play Node: {e}"),
      }*/
    }

    ui.same_line();
    if state.is_dialog_playing {
      ui.separator_vertical();
      ui.same_line();
      if ui.button("Stop") {
        output.commands.push(UiCommand::StopDialog);
      }
    }

    ui.same_line();
    ui.separator_vertical();
    ui.same_line();
  }

  let mut add_effect = |cmd| {
    output.commands.push(UiCommand::Graph(cmd));
  };

  if !state.graph_state.flow_link_editing {
    if ui.button("Add Conversation") {
      let name = format!("Conversation {}", state.id_gen.next());
      add_effect(UiGraphCommand::AddConversation {
        name,
        position: ui.mouse_pos(),
      });
    }
    ui.same_line();
    if ui.button("Add Choicer") {
      let name = format!("Choicer {}", state.id_gen.next());
      add_effect(UiGraphCommand::AddChoicer {
        name,
        position: ui.mouse_pos(),
      });
    }

    if let Some(id) = state.graph_state.selected_node_id {
      ui.same_line();

      if ui.button("Delete") {
        if let Some(nodes_idx) = state.graph_state.nodes.iter().position(|n| n.id() == id) {
          add_effect(UiGraphCommand::RemoveNode { nodes_idx });
        }
      }
    }

    ui.same_line();
    ui.separator_vertical();
  }

  ui.same_line();
  if ui.button(if !state.graph_state.flow_link_editing {
    "Toggle Flow Link"
  } else {
    "Toggle Link"
  }) {
    add_effect(UiGraphCommand::ToggleLinkEdit);
  }

  // ==========================
  // Viewport
  // ==========================
  let editor = ui.node_editor(&state.node_editor, "DialogueGraph", [0.0, 0.0]);

  for node in &mut state.graph_state.nodes {
    if !node.dirty() {
      continue;
    }

    match node {
      Node::Conversation {
        id,
        position,
        dirty,
        ..
      } => {
        *dirty = false;
        editor.set_node_position(*id, *position);
      }
      Node::Choicer {
        id,
        position,
        dirty,
        ..
      } => {
        *dirty = false;
        editor.set_node_position(*id, *position);
      }
    }
  }

  // Mutable just for ui.input
  for node in &mut state.graph_state.nodes {
    let am_i_current_dialog = |name: &str| {
      state
        .current_dialog_name
        .as_ref()
        .is_some_and(|current| current.as_ref() == name)
    };

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
          if am_i_current_dialog(name) {
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
          if am_i_current_dialog(name) {
            ui.text_colored([1.0, 0.0, 0.0, 1.0], name);
          } else {
            ui.text(name);
          }

          for (idx, (opt, pin_id)) in options.iter_mut().zip(outputs.iter()).enumerate() {
            let _id = ui.push_id(pin_id.0);
            if ui.button("X") {
              add_effect(UiGraphCommand::RemoveChoicerOption {
                node_id: *id,
                option_idx: idx,
              });
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
            add_effect(UiGraphCommand::AddChoicerOption { node_id: *id });
          }
        });
      }
    }
  }

  // --- Dibujar links existentes ---
  for link in &state.graph_state.links {
    let alpha = if state.graph_state.flow_link_editing {
      0.2
    } else {
      1.0
    };
    editor.link_colored(link.id, link.from, link.to, [0.37, 0.72, 0.95, alpha], 2.5);
  }

  // --- Dibujar flow links existentes ---
  for link in &state.graph_state.flow_links {
    let alpha = if !state.graph_state.flow_link_editing {
      0.3
    } else {
      1.0
    };
    editor.link_colored(link.id, link.from, link.to, [1.0, 0.0, 0.0, alpha], 2.5);
  }

  if let Some(create) = editor.begin_create([0.30, 0.85, 0.45, 1.0], 2.0) {
    if let Some((a, b)) = create.query_new_link() {
      if let Some((from, to)) = normalize_link(&state.graph_state.nodes, a, b) {
        if create.accept_new_item() {
          if state.graph_state.flow_link_editing {
            add_effect(UiGraphCommand::AddFlowLink { from, to });
          } else {
            add_effect(UiGraphCommand::AddLink { from, to });
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
        if state.graph_state.flow_link_editing {
          if let Some(flow_links_idx) = state
            .graph_state
            .flow_links
            .iter()
            .position(|l| l.id == link_id)
          {
            add_effect(UiGraphCommand::RemoveFlowLink { flow_links_idx });
          }
        } else if let Some(links_idx) = state.graph_state.links.iter().position(|l| l.id == link_id)
        {
          add_effect(UiGraphCommand::RemoveLink { links_idx });
        }
      }
    }
    while delete.query_deleted_node().is_some() {
      delete.reject_deleted_item();
    }
  }

  let current_selection = match editor.selected_nodes().as_slice() {
    [id] => Some(*id),
    _ => None,
  };

  if state.graph_state.selected_node_id != current_selection {
    state.graph_state.selected_node_id = current_selection;
  }

  editor.end();

  output
}

pub fn draw_graph_inspector(ui: &dear_imgui_rs::Ui, state: &mut UiState) {
  ui.text("Inspector");
  ui.separator();

  if let Some(node) = state.graph_state.get_mut_selected_node() {
    let name = match node {
      Node::Conversation { name, .. } => name,
      Node::Choicer { name, .. } => name,
    };

    ui.text("Name:");
    ui.same_line();
    ui.input_text("##graph_inspector_input", name).build();
  }
}

/// Determina cuál de los dos pines involucrados en un drag es el de
/// salida y cuál el de entrada, buscando en todos los nodos. Devuelve
/// `None` si el par no es válido (ambos del mismo tipo, o no encontrados).
fn normalize_link(nodes: &[Node], a: PinId, b: PinId) -> Option<(PinId, PinId)> {
  let kind_of = |pin: PinId| -> Option<PinKind> {
    for node in nodes {
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

pub fn handle_ui_graph_command(cmd: UiGraphCommand, state: &mut UiState) {
  match cmd {
    UiGraphCommand::AddConversation { name, position } => {
      let node = Node::Conversation {
        id: state.id_gen.next_node(),
        name,
        input: state.id_gen.next_pin(),
        output: state.id_gen.next_pin(),
        position,
        dirty: true,
      };

      state.graph_state.nodes.push(node);
    }
    UiGraphCommand::AddChoicer { name, position } => {
      let node = Node::Choicer {
        id: state.id_gen.next_node(),
        name,
        options: vec![],
        outputs: vec![],
        input: state.id_gen.next_pin(),
        position,
        dirty: true,
      };

      state.graph_state.nodes.push(node);
    }
    UiGraphCommand::AddChoicerOption { node_id } => {
      if let Some(Node::Choicer {
        options, outputs, ..
      }) = state
        .graph_state
        .nodes
        .iter_mut()
        .find(|n| n.id() == node_id)
      {
        options.push("Text".into());
        outputs.push(state.id_gen.next_pin());
      }
    }
    UiGraphCommand::RemoveNode { nodes_idx } => {
      let pins: Vec<PinId> = match &state.graph_state.nodes[nodes_idx] {
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

      let node = &state.graph_state.nodes[nodes_idx];

      state.graph_state.timeline_states.remove(&node.id());
      state
        .graph_state
        .links
        .retain(|link| !pins.contains(&link.from) && !pins.contains(&link.to));
      state
        .graph_state
        .flow_links
        .retain(|link| !pins.contains(&link.from) && !pins.contains(&link.to));
      state.graph_state.nodes.remove(nodes_idx);
    }
    UiGraphCommand::ToggleLinkEdit => {
      state.graph_state.flow_link_editing = !state.graph_state.flow_link_editing;
    }
    UiGraphCommand::RemoveChoicerOption {
      node_id,
      option_idx,
    } => {
      if let Some(Node::Choicer {
        options, outputs, ..
      }) = state
        .graph_state
        .nodes
        .iter_mut()
        .find(|n| n.id() == node_id)
      {
        let pin = outputs.remove(option_idx);
        options.remove(option_idx);
        if state.graph_state.flow_link_editing {
          state
            .graph_state
            .flow_links
            .retain(|link| link.from != pin && link.to != pin);
        } else {
          state
            .graph_state
            .links
            .retain(|link| link.from != pin && link.to != pin);
        }
      }
    }
    UiGraphCommand::AddLink { from, to } => {
      let link = Link {
        id: state.id_gen.next_link(),
        from,
        to,
      };
      state.graph_state.links.push(link);
    }
    UiGraphCommand::AddFlowLink { from, to } => {
      let link = Link {
        id: state.id_gen.next_link(),
        from,
        to,
      };
      state.graph_state.flow_links.push(link);
    }
    UiGraphCommand::RemoveLink { links_idx } => {
      state.graph_state.links.remove(links_idx);
    }
    UiGraphCommand::RemoveFlowLink { flow_links_idx } => {
      state.graph_state.flow_links.remove(flow_links_idx);
    }
  }
}
