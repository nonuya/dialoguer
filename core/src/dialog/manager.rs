use std::{collections::{HashMap, VecDeque}, rc::Rc};

use crate::{
  dialog::parser::{Dialog, Event, Token, dialog_parser},
  live2d::animator::{Animator, EnumMap, MotionManager, ParamValue, Value},
};
use anyhow::Context;
use chumsky::Parser;
use log::{debug, warn};

// Esto indicará si qué cosa será nuestro primer dialogo cuando presionemos "Iniciar Dialogo"
pub enum DialogEntryPoint {
  Choicer(Vec<(Rc<str>, DialogIter)>),
  Conversation(DialogIter),
}

pub struct DialogManager {
  dialogs: Vec<Dialog>,
  map: HashMap<Rc<str>, usize>,
}

impl DialogManager {
  pub fn new_from_entries(entries: Vec<(Rc<str>, Dialog)>) -> Self {
    let (dialogs, map) = entries.into_iter().fold(
      (Vec::new(), HashMap::new()),
      |(mut dialogs, mut map), (id, dialog)| {
        let index = dialogs.len();

        dialogs.push(dialog);
        assert!(map.insert(id, index).is_none(), "Existing Dialog!");

        (dialogs, map)
      },
    );

    Self { dialogs, map }
  }

  pub fn new_from_tokens(tokens: Vec<Token>) -> anyhow::Result<Self> {
    let entries = dialog_parser()
      .parse(&tokens)
      .into_result()
      .map_err(|err| anyhow::anyhow!("Dialog Block Parser {:#?}", err))?;

    Ok(Self::new_from_entries(entries))
  }

  pub fn get_dialogs(&self) -> Vec<(&Rc<str>, &Dialog)> {
    self
      .map
      .iter()
      .map(|(k, v)| (k, &self.dialogs[*v]))
      .collect()
  }

  pub fn get_dialog_by_id(&self, id: &str) -> Option<&Dialog> {
    self.map.get(id).and_then(|idx| Some(&self.dialogs[*idx]))
  }

  // Construyes un iterator a partir de un bloque
  pub fn build(&self, id: &str) -> Option<DialogEntryPoint> {
    match self.map.get(id) {
      Some(dialog_idx) => match &self.dialogs[*dialog_idx] {
        Dialog::Choicer(nodes) => {
          let choices = nodes
            .iter()
            .enumerate()
            .map(|(idx, n)| {
              (
                n.label.clone(),
                DialogIter {
                  index: *dialog_idx,
                  queue: VecDeque::from([ConversationIter {
                    idx,
                    events: (0..n.events.len()).collect(),
                  }]),
                },
              )
            })
            .collect();

          Some(DialogEntryPoint::Choicer(choices))
        }
        Dialog::Conversation(nodes) => {
          let queue = nodes
            .iter()
            .enumerate()
            .map(|(idx, n)| ConversationIter {
              idx,
              events: (0..n.events.len()).collect(),
            })
            .collect();

          Some(DialogEntryPoint::Conversation(DialogIter {
            index: *dialog_idx,
            queue,
          }))
        }
      },
      None => None,
    }
  }
}

/*
* Dialogo es este bloque
[Header]
 Player:   <- Esta es una conversación
   ...
 Saya-Chan:
   ...
===
*/
#[derive(Debug, Clone)]
pub struct DialogIter {
  index: usize,                      // Dialogo
  queue: VecDeque<ConversationIter>, // Player, Saya-Chan, Player, ...
}

#[derive(Debug, Clone)]
pub struct ConversationIter {
  idx: usize, // DialogNode
  events: VecDeque<usize>,
}

#[derive(Debug)]
enum PlayerState {
  Running,
  WaitingInput,
  WaitingChoice(Vec<(Rc<str>, DialogIter)>),
  Finished,
}

pub struct DialogPlayer {
  initial_dialog: DialogEntryPoint,
  initial_dialog_name: Rc<str>,
  current_dialog_name: Rc<str>,
  state: PlayerState,
  current_node_idx: usize,
  iter: Option<DialogIter>,
  shown: bool,
}

impl DialogPlayer {
  pub fn new(initial_dialog_name: Rc<str>, initial_dialog: DialogEntryPoint) -> Self {
    Self {
      current_dialog_name: initial_dialog_name.clone(),
      initial_dialog_name,
      initial_dialog,
      iter: None,
      current_node_idx: 0,
      state: PlayerState::Running,
      shown: false,
    }
  }

  pub fn current_node_index(&self) -> Option<usize> {
    if !self.is_playing() {
      return None;
    }

    Some(self.current_node_idx)
  }

  pub fn is_playing(&self) -> bool {
    !matches!(self.state, PlayerState::Finished)
  }

  pub fn current_dialog_name(&self) -> Option<&Rc<str>> {
    if !self.is_playing() {
      return None;
    }

    Some(&self.current_dialog_name)
  }

  pub fn skip(
    &mut self,
    n: usize,
    animator: &mut Animator,
    dialog_mgr: &DialogManager,
    enum_map: &EnumMap,
    motion_mgr: &MotionManager,
  ) {
    for _ in 0..n {
      self.force_next(animator, dialog_mgr, enum_map, motion_mgr);
    }
  }

  pub fn force_next(
    &mut self,
    animator: &mut Animator,
    dialog_mgr: &DialogManager,
    enum_map: &EnumMap,
    motion_mgr: &MotionManager,
  ) {
    let Some(iter) = self.iter.as_mut() else {
      return;
    };

    // Antes de avanzar al siguiente nodo, consumimos los eventos
    // restantes del nodo actual aplicando SetParameter/SetMainChoicer/RemoveParamater.
    if let Some(conversation_iter) = iter.queue.front_mut() {
      let dialog = &dialog_mgr.dialogs[iter.index];
      let nodes = match dialog {
        Dialog::Conversation(nodes) => nodes,
        Dialog::Choicer(nodes) => nodes,
      };
      let conversation = &nodes[conversation_iter.idx];

      while let Some(&idx) = conversation_iter.events.front() {
        match &conversation.events[idx] {
          Event::SetMainChoicer(id) => {
            if let Some(initial_dialog) = dialog_mgr.build(id) {
              self.initial_dialog = initial_dialog;
            }
          }
          Event::SetParameter(enum_type, enum_value) => match enum_map.enums.get(enum_type) {
            Some(myenum) => match myenum.values.get(enum_value) {
              Some(params) => {
                for p in params {
                  animator.set_parameter(p.name.clone(), get_parameter_value(p, enum_map, animator));
                }
              }
              None => warn!(
                "EnumValue '{}' doesn't exists in '{}'",
                enum_value, enum_type
              ),
            },
            None => warn!("EnumType '{}' doesn't exists!", enum_type),
          },
          Event::RemoveParamater(enum_type) => match enum_map.enums.get(enum_type) {
            Some(myenum) => match myenum.values.values().next() {
              Some(params) => {
                for p in params {
                  warn!("Removing '{}'", &p.name);
                  animator.remove_parameter(&p.name);
                }
              }
              None => warn!("EnumType '{}' is empty", enum_type),
            },
            None => warn!("EnumType '{}' doesn't exists!", enum_type),
          },
          Event::SetAnim(name) => match motion_mgr.get(name) {
            Some(motion) => animator.set_motion(motion.clone()),
            None => warn!("Animation '{}' not found", name),
          },
          _ => {}
        }

        conversation_iter.events.pop_front();
      }
    }

    if iter.queue.pop_front().is_none() {
      self.state = PlayerState::Finished;
      self.change_iter(None);
    } else {
      self.state = PlayerState::Running;
    }
  }

  pub fn play(&mut self) {
    self.current_dialog_name = self.initial_dialog_name.clone();
    match &self.initial_dialog {
      DialogEntryPoint::Choicer(choices) => {
        self.state = PlayerState::WaitingChoice(choices.clone());
      }
      DialogEntryPoint::Conversation(iter) => {
        self.change_iter(Some(iter.clone()));
      }
    }
  }

  pub fn next(&mut self) {
    if let Some(iter) = self.iter.as_mut() {
      if matches!(self.state, PlayerState::WaitingInput) {
        if iter.queue.pop_front().is_none() {
          self.state = PlayerState::Finished;
          self.iter = None;
        } else {
          self.state = PlayerState::Running;
        }
      }
    }
  }

  pub fn update(
    &mut self,
    animator: &mut Animator,
    dialog_mgr: &DialogManager,
    enum_map: &EnumMap,
    motion_mgr: &MotionManager,
  ) {
    match &self.state {
      PlayerState::Running => self.consume_dialog(animator, dialog_mgr, enum_map, motion_mgr),
      PlayerState::WaitingChoice(choices) => {
        if !self.shown {
          self.shown = true;
          for c in choices.iter().enumerate() {
            println!("{}) {}", c.0 + 1, c.1.0);
          }
        }
      }
      _ => {}
    }
  }

  pub fn handle_input(&mut self, idx: usize) {
    if let PlayerState::WaitingChoice(choices) = &self.state {
      if let Some(choice) = choices.get(idx) {
        self.shown = false;
        warn!("Selecting {:#?}", choice);
        self.iter = Some(choice.1.clone());
        self.state = PlayerState::Running;
      }
    }
  }

  fn consume_dialog(
    &mut self,
    animator: &mut Animator,
    dialog_mgr: &DialogManager,
    enum_map: &EnumMap,
    motion_mgr: &MotionManager,
  ) {
    if animator.is_timer_playing() {
      return;
    }

    let Some(iter) = self.iter.as_mut() else {
      self.state = PlayerState::Finished;
      return;
    };
    let Some(conversation_iter) = iter.queue.front_mut() else {
      self.state = PlayerState::Finished;
      return;
    };

    let dialog = &dialog_mgr.dialogs[iter.index];
    let nodes = match dialog {
      Dialog::Conversation(nodes) => nodes,
      Dialog::Choicer(nodes) => nodes,
    };

    let conversation = &nodes[conversation_iter.idx];
    let mut next_iter = None;

    loop {
      let Some(idx) = conversation_iter.events.front_mut() else {
        self.state = PlayerState::WaitingInput;
        // Podríamos hacer conversation_iter.pop_front() pero eso haría que avance al siguiente dialogo.
        // Nosotros queremos esperar el input del usuario.
        break;
      };

      self.current_node_idx = conversation_iter.idx;

      match &conversation.events[*idx] {
        Event::SetMainChoicer(id) => {
          if let Some(initial_dialog) = dialog_mgr.build(id) {
            self.initial_dialog = initial_dialog;
          }
        }
        Event::Text(text) => {
          println!("{}: {}", conversation.label, text);
        }
        Event::Wait(seconds) => {
          debug!("[Wait] {} seconds", seconds);
          animator.set_timer(*seconds);
        }
        Event::SetParameter(enum_type, enum_value) => {
          match enum_map.enums.get(enum_type) {
            Some(myenum) => match myenum.values.get(enum_value) {
              Some(params) => {
                for p in params {
                  animator.set_parameter(p.name.clone(), get_parameter_value(p, enum_map, animator));
                }
              }
              None => warn!(
                "[SetParameter] EnumValue '{}' doesn't exists in '{}'",
                enum_value, enum_type
              ),
            },
            None => warn!("[SetParameter] EnumType '{}' doesn't exists!", enum_type),
          }
          conversation_iter.events.pop_front();
          continue;
        }
        Event::Jump(id) => {
          self.current_dialog_name = id.clone();
          match dialog_mgr.build(id) {
            Some(entry_point) => match entry_point {
              DialogEntryPoint::Choicer(choices) => {
                warn!("[Jump] Choicer '{}'", id);
                self.state = PlayerState::WaitingChoice(choices.clone());
                break;
              }
              DialogEntryPoint::Conversation(iter) => {
                warn!("[Jump] Conversation '{}'", id);
                next_iter = Some(iter);
                break;
              }
            },
            None => warn!("[Jump] Failed to jumping. '{}' doesnt exists", id),
          }
        }
        Event::RemoveParamater(enum_type) => {
          match enum_map.enums.get(enum_type) {
            Some(myenum) => {
              let params = myenum
                .values
                .values()
                .next()
                .context("EnumType is empty")
                .unwrap();
              for p in params {
                // FIXME: Remove &'static str
                warn!("[RemoveParameter] Removing '{}'", &p.name);
                animator.remove_parameter(&p.name);
              }
            }
            None => warn!("[RemoveParameter] EnumType '{}' doesn't exists!", enum_type),
          }
          conversation_iter.events.pop_front();
          continue;
        }
        Event::SetAnim(name) => match motion_mgr.get(name) {
          Some(motion) => animator.play_motion(motion.clone()),
          None => warn!("[SetAnim] Animation '{}' not found", name),
        },
        Event::Next => {
          iter.queue.pop_front();
          break;
        }
        ev => {
          debug!("[Unknown Event] {:#?}", ev);
        }
      }

      conversation_iter.events.pop_front();
      break;
    }

    if let Some(next) = next_iter.take() {
      self.change_iter(Some(next));
    }
  }

  fn change_iter(&mut self, opt_iter: Option<DialogIter>) {
    self.current_node_idx = 0;
    match opt_iter {
      Some(iter) => {
        self.iter = Some(iter);
      }
      None => {
        self.iter = None;
      }
    }
  }
}

fn get_parameter_value(p: &ParamValue, enum_map: &EnumMap, animator: &Animator) -> Value {
  let inc = p
    .modification
    .as_ref()
    .map(|m| {
      let res = enum_map
        .enums
        .get(&m.lhs)
        .and_then(|e| e.values.get(&m.rhs))
        .is_some_and(|values| {
          values.iter().all(|v| {
            animator
              .is_parameter_equal_to_value(&v.name, &get_parameter_value(v, enum_map, animator))
          })
        });

      if res { m.then } else { 0.0 }
    })
    .unwrap_or(0.0);

  match p.value {
    Value::Fixed(v) => Value::Fixed(v + inc),
    Value::Smooth { target, step, .. } => Value::Smooth {
      actual: 0.0,
      target: target + inc,
      step,
    },
  }
}
