use core::dialog::DialogNode;
use dear_imgui_rs::*;
use dear_node_editor::*;

pub struct Graph {
  nodes: Vec<Node>,
  links: Vec<Link>,
  id_gen: IdGen,
}

enum Node {
  Conversation {
    id: NodeId,
    dialog: DialogNode,
    input: PinId,
    output: PinId,
  },
  Choicer {
    id: NodeId,
    options: Vec<String>, // TODO: Change to DialogNode?
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
}

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
  pub fn new() -> Self {
    let mut id_gen = IdGen::new();

    let conv1 = Node::Conversation {
      id: id_gen.next_node(),
      dialog: DialogNode {
        label: "Hello!".into(),
        events: vec![],
      },
      input: id_gen.next_pin(),
      output: id_gen.next_pin(),
    };

    let choice = Node::Choicer {
      id: id_gen.next_node(),
      input: id_gen.next_pin(),
      outputs: vec![id_gen.next_pin(), id_gen.next_pin()],
      options: vec!["Yes".into(), "No".into()],
    };

    let conv2 = Node::Conversation {
      id: id_gen.next_node(),
      dialog: DialogNode {
        label: "Great!".into(),
        events: vec![],
      },
      input: id_gen.next_pin(),
      output: id_gen.next_pin(),
    };

    let conv3 = Node::Conversation {
      id: id_gen.next_node(),
      dialog: DialogNode {
        label: "Too bad.".into(),
        events: vec![],
      },
      input: id_gen.next_pin(),
      output: id_gen.next_pin(),
    };

    let links = vec![
      Link {
        id: id_gen.next_link(),
        from: match &conv1 {
          Node::Conversation { output, .. } => *output,
          _ => unreachable!(),
        },
        to: match &choice {
          Node::Choicer { input, .. } => *input,
          _ => unreachable!(),
        },
      },
      Link {
        id: id_gen.next_link(),
        from: match &choice {
          Node::Choicer { outputs, .. } => outputs[0],
          _ => unreachable!(),
        },
        to: match &conv2 {
          Node::Conversation { input, .. } => *input,
          _ => unreachable!(),
        },
      },
      Link {
        id: id_gen.next_link(),
        from: match &choice {
          Node::Choicer { outputs, .. } => outputs[1],
          _ => unreachable!(),
        },
        to: match &conv3 {
          Node::Conversation { input, .. } => *input,
          _ => unreachable!(),
        },
      },
    ];

    Self {
      nodes: vec![conv1, choice, conv2, conv3],
      links,
      id_gen,
    }
  }

  fn add_conversation(&mut self, dialog: DialogNode) -> NodeId {
    let id = self.id_gen.next_node();
    let input = self.id_gen.next_pin();
    let output = self.id_gen.next_pin();
    self.nodes.push(Node::Conversation {
      id,
      dialog,
      input,
      output,
    });
    id
  }

  fn add_choicer(&mut self, options: Vec<String>) -> NodeId {
    let id = self.id_gen.next_node();
    let input = self.id_gen.next_pin();
    let outputs = options.iter().map(|_| self.id_gen.next_pin()).collect();
    self.nodes.push(Node::Choicer {
      id,
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

  pub fn draw(&mut self, ui: &Ui, node_editor: &EditorContext) {
    let available = ui.content_region_avail();
    let right_panel_width = 300.0;
    let left_panel_width = (available[0] - right_panel_width).max(100.0);

    // --- Panel izquierdo: el graph ---
    ui.child_window("##graph_panel")
      .size([left_panel_width, available[1]])
      .border(true)
      .build(ui, || {
        self.draw_graph(ui, node_editor);
      });

    ui.same_line();

    // --- Panel derecho: inspector / detalles ---
    ui.child_window("##inspector_panel")
      .size([right_panel_width, available[1]])
      .border(true)
      .build(ui, || {
        ui.text("Inspector");
        ui.separator();
      });
  }

  pub fn draw_graph(&mut self, ui: &Ui, node_editor: &EditorContext) {
    let editor = ui.node_editor(node_editor, "DialogueGraph", [0.0, 0.0]);
    let mut pending_new_option: Option<NodeId> = None;
    let mut pending_delete_option: Option<(NodeId, usize)> = None;

    for node in &self.nodes {
      match node {
        Node::Conversation {
          id,
          dialog,
          input,
          output,
        } => {
          editor.node(*id, |node| {
            node.pin(*input, PinKind::Input, |_pin| {
              ui.text(" * ");
            });
            ui.same_line();
            ui.text("Conversation");
            ui.same_line();
            node.pin(*output, PinKind::Output, |_pin| {
              ui.text(" * ");
            });
          });
        }
        Node::Choicer {
          id,
          options,
          input,
          outputs,
        } => {
          editor.node(*id, |node| {
            node.pin(*input, PinKind::Input, |_pin| {
              ui.text(" * ");
            });
            ui.same_line();
            ui.text("Choicer");

            for (idx, (opt, pin_id)) in options.iter().zip(outputs.iter()).enumerate() {
              let _id = ui.push_id(pin_id.0);
              if ui.button("X") {
                pending_delete_option = Some((*id, idx));
              }
              ui.same_line();
              ui.text(opt);
              ui.same_line();
              node.pin(*pin_id, PinKind::Output, |_pin| {
                ui.text(" * ");
              });
            }

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

    editor.end();

    // --- Aplicar el "+" pendiente después de terminar el frame del editor ---
    if let Some(node_id) = pending_new_option {
      let next_idx = self
        .nodes
        .iter()
        .find(|n| n.id() == node_id)
        .map(|n| match n {
          Node::Choicer { options, .. } => options.len() + 1,
          _ => 0,
        })
        .unwrap_or(0);
      self.add_choicer_option(node_id, format!("Choice {}", next_idx));
    }
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
}
