use std::collections::{HashMap, VecDeque, btree_map::IterMut};

use crate::{
  dialog::parser::{Dialog, DialogNode, Event, Token, dialog_parser},
  live2d::animator::{Animator, EnumMap, MotionManager},
};
use anyhow::Context;
use chumsky::Parser;
use log::{debug, warn};

// Esto indicará si qué cosa será nuestro primer dialogo cuando presionemos "Iniciar Dialogo"
pub enum DialogEntryPoint {
  Choicer(Vec<(String, DialogIter)>),
  Conversation(DialogIter),
}

pub struct DialogManager {
  dialogs: Vec<Dialog>,
  map: HashMap<String, usize>,
}

impl DialogManager {
  pub fn new(tokens: Vec<Token>) -> anyhow::Result<Self> {
    let (dialogs, map) = dialog_parser()
      .parse(&tokens)
      .into_result()
      .map_err(|err| anyhow::anyhow!("Dialog Block Parser {:#?}", err))?
      .into_iter()
      .fold(
        (Vec::new(), HashMap::new()),
        |(mut dialogs, mut map), (id, dialog)| {
          let index = dialogs.len();

          dialogs.push(dialog);
          map.insert(id, index);

          (dialogs, map)
        },
      );

    Ok(Self { dialogs, map })
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

#[derive(Debug, Clone)]
pub struct DialogIter {
  index: usize,                      // Dialogo
  queue: VecDeque<ConversationIter>, // Player, Saya-Chan, Player, ...
}

impl DialogIter {
  fn next(&mut self) {
    let conversation_idx = self.queue.front();
    if conversation_idx.is_none() {
      return;
    }

    let conversation_iter = conversation_idx.unwrap();

    if conversation_iter.events.is_empty() {
      self.queue.pop_front();
    }
  }
}

#[derive(Debug, Clone)]
pub struct ConversationIter {
  idx: usize, // DialogNode
  events: VecDeque<usize>,
}

enum PlayerState {
  Running,
  WaitingChoice(Vec<(String, DialogIter)>),
  Finished,
}

pub struct DialogPlayer {
  initial_dialog: DialogEntryPoint,
  state: PlayerState,
  iter: Option<DialogIter>,
  shown: bool,
}

impl DialogPlayer {
  pub fn new(initial_dialog: DialogEntryPoint) -> Self {
    Self {
      initial_dialog,
      iter: None,
      state: PlayerState::Running,
      shown: false,
    }
  }

  pub fn play(&mut self) {
    match &self.initial_dialog {
      DialogEntryPoint::Choicer(choices) => {
        self.state = PlayerState::WaitingChoice(choices.clone());
      }
      DialogEntryPoint::Conversation(iter) => self.iter = Some(iter.clone()),
    }
  }

  pub fn next(&mut self) {
    if let Some(iter) = self.iter.as_mut() {
      iter.next();

      if iter.queue.is_empty() {
        self.iter = None;
      }
    }
  }

  pub fn update(
    &mut self,
    dialog_mgr: &DialogManager,
    animator: &mut Animator,
    enum_map: &EnumMap,
    motion_mgr: &MotionManager,
  ) {
    match &self.state {
       PlayerState::Running => self.next_iter(dialog_mgr, animator, enum_map, motion_mgr),
       PlayerState::WaitingChoice(choices) => {
         if !self.shown {
           self.shown = true;
           for c in choices.iter().enumerate() {
            println!("{}) {}", c.0+1, c.1.0);
           }
         }
       },
       PlayerState::Finished => {}
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

  fn next_iter(
    &mut self,
    dialog_mgr: &DialogManager,
    animator: &mut Animator,
    enum_map: &EnumMap,
    motion_mgr: &MotionManager,
  ) {
    if self.iter.is_none() || animator.is_timer_playing() {
      return;
    }

    let iter = self.iter.as_mut().unwrap();

    let conversation_idx = iter.queue.front_mut();
    if conversation_idx.is_none() {
      return;
    }
    
    let dialog = &dialog_mgr.dialogs[iter.index];
    let conversation_iter = conversation_idx.unwrap();
    let nodes = match dialog {
      Dialog::Conversation(nodes) => nodes,
      Dialog::Choicer(nodes) => nodes,
    };

    let mut next_iter = None;
    let conversation = &nodes[conversation_iter.idx];

    loop {
      let Some(idx) = conversation_iter.events.front_mut() else {
        break;
      };

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
          debug!("Waiting for {} seconds", seconds);
          animator.set_timer(*seconds);
        }
        Event::SetParameter(enum_type, enum_value) => {
          match enum_map.get(enum_type.as_str()) {
            Some(values) => {
              // FIXME: Delete .0.
              match values.0.get(enum_value.as_str()) {
                Some(params) => {
                  for p in params {
                    animator.set_parameter(p.0, p.1);
                  }
                }
                None => warn!(
                  "EnumValue '{}' doesn't exists in '{}'",
                  enum_type, enum_value
                ),
              }
            }
            None => warn!("EnumType '{}' doesn't exists!", enum_type),
          }
          conversation_iter.events.pop_front();
          continue;
        },
        Event::Jump(id) => match dialog_mgr.build(id) {
          Some(entry_point) => match entry_point {
            DialogEntryPoint::Choicer(choices) => {
              warn!("Jumping to Choicer '{}'", id);
              self.state = PlayerState::WaitingChoice(choices.clone());   
              break;
            }
            DialogEntryPoint::Conversation(iter) => {
              warn!("Jumping to Conversation '{}'", id);
              next_iter = Some(iter);
              break;
            }
          },
          None => warn!("Failed to jumping. '{}' doesnt exists", id),
        },
        Event::RemoveParamater(enum_type) => {
          match enum_map.get(enum_type.as_str()) {
            Some(myenum) => {
              let params = myenum
                .0
                .values()
                .next()
                .context("EnumType is empty")
                .unwrap();
              for p in params {
                // FIXME: Remove &'static str
                warn!("Removing '{}'", p.0);
                animator.remove_parameter(&p.0.to_string());
              }
            }
            None => warn!("EnumType '{}' doesn't exists!", enum_type),
          }
          animator.remove_parameter(enum_type);
          conversation_iter.events.pop_front();
          continue;
        }
        Event::SetAnim(name) => match motion_mgr.get(name) {
          Some(motion) => animator.play_motion(motion.clone(), true),
          None => warn!("Animation '{}' not found", name),
        },
        Event::Next => {
          iter.queue.pop_front();
          break;
        }
        ev => {
          debug!("{:#?}", ev);
        }
      }

      conversation_iter.events.pop_front();
      break;
    }

    if let Some(next) = next_iter {
      *iter = next;
    }
  }
}
