use std::collections::HashMap;

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
  ) -> anyhow::Result<Vec<(String, core::dialog::Dialog)>> {
    use core::dialog::{Dialog, DialogNode, Event};
    use std::collections::VecDeque;

    let mut queue = VecDeque::new();
    let node = self
      .get_node_by_id(id)
      .context("Failed to get Node by Id")?;
    let mut dialogs: Vec<(String, Dialog)> = Vec::new();
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
                  events: vec![Event::Jump(next.name().clone())],
                });
              }
              Dialog::Choicer(_) => {
                unreachable!();
              }
            }
          }

          dialogs.push((name.clone(), dialog));
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
                  label: option.clone(),
                  events: vec![Event::Jump(next.name().clone())],
                });
                queue.push_back(next);
              }
              None => {
                warn!("Skipping '{}' because it doesnt have an output node.", name);
              }
            }
          }
          dialogs.push((name.clone(), Dialog::Choicer(valid_choices)));
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
