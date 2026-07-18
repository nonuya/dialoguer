use std::collections::{HashMap, VecDeque};

use crate::{
  dialog::parser::{Dialog, Event, Token, dialog_parser},
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

    anyhow::ensure!(map.contains_key("Initial.1"), "Dialog File must be have [Initial.1]");
    anyhow::ensure!(map.contains_key("Idle"), "Dialog File must be have [Idle]");
    anyhow::ensure!(map.contains_key("Phase01"), "Dialog File must be have [[Phase01]]");

    Ok(Self { dialogs, map })
  }

  pub fn build_phase01(&self) -> DialogEntryPoint {
    self.build("Phase01").unwrap()
  }

  pub fn build_idle(&self) -> DialogEntryPoint {
    self.build("Idle").unwrap()
  }

  pub fn build_initial(&self) -> DialogEntryPoint {
    self.build("Initial.1").unwrap()
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
            println!("{}) {}", c.0+1, c.1.0);
           }
         }
       },
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
        Dialog::Choicer(nodes) => nodes
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
          match enum_map.enums.get(enum_type) {
            Some(myenum) => {
              match myenum.values.get(enum_value) {
                Some(params) => {
                  for p in params {
                    animator.set_parameter(&p.name, p.value);
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
                warn!("Removing '{}'", &p.name);
                animator.remove_parameter(&p.name);
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
