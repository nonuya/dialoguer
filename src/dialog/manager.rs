use std::collections::{HashMap, VecDeque};

use crate::{
  dialog::parser::{Conversation, Dialog, Event, Token, dialog_parser},
  live2d::animator::{Animator, EnumMap, MotionManager},
};
use anyhow::Context;
use chumsky::Parser;
use log::{debug, warn};

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

  pub fn build_dialog(&self, id: &str) -> Option<DialogIter> {
    match self.map.get(id) {
      Some(dialog_idx) => match &self.dialogs[*dialog_idx] {
        Dialog::Conversation(items) => {
          let queue = items
            .iter()
            .enumerate()
            .map(|(idx, item)| ConversationIter {
              idx,
              events: (0..item.events.len()).collect(),
            })
            .collect();

          Some(DialogIter {
            index: *dialog_idx,
            queue,
          })
        }
        Dialog::Choicer(_) => {
          warn!(
            "You want to build conversation '{}' but it is a choicer",
            id
          );
          None
        }
      },
      None => None,
    }
  }
}

#[derive(Debug)]
pub struct DialogIter {
  index: usize,                      // Dialogo
  queue: VecDeque<ConversationIter>, // Player, Saya-Chan, Player, ...
}

#[derive(Debug)]
pub struct ConversationIter {
  idx: usize,
  events: VecDeque<usize>,
}

pub fn run_dialog(
  mgr: &DialogManager,
  iter: &mut DialogIter,
  animator: &mut Animator,
  enum_map: &EnumMap,
  motion_mgr: &MotionManager,
) /*Finished*/ {
  if animator.is_timer_playing() {
    return;
  }

  let conversation_idx = iter.queue.front_mut();
  if conversation_idx.is_none() {
    return;
  }

  let dialog = &mgr.dialogs[iter.index];
  let conversation_iter = conversation_idx.unwrap();

  match dialog {
    Dialog::Conversation(conversations) => {
      let conversation = &conversations[conversation_iter.idx];
      // debug!("{}:", conversation.who);
      loop {
        let Some(idx) = conversation_iter.events.front_mut() else {
          break;
        };

        match &conversation.events[*idx] {
          Event::Text(text) => {
            println!("{}: {}", conversation.who, text);
            conversation_iter.events.pop_front();
            break;
          },
          Event::Wait(seconds) => {
            debug!("Waiting for {} seconds", seconds);
            animator.set_timer(*seconds);
            conversation_iter.events.pop_front();
            break;
          },
          Event::SetParameter(enum_type, enum_value) => {
            match enum_map.get(enum_type.as_str()) {
              Some(values) => {
                // FIXME: Delete .0.
                match values.0.get(enum_value.as_str()) {
                  Some(params) => {
                    for p in params {
                      animator.set_parameter(p.0, p.1);
                    }
                  },
                  None => warn!("EnumValue '{}' doesn't exists in '{}'", enum_type, enum_value)
                }
              }
              None => warn!("EnumType '{}' doesn't exists!", enum_type),
            }
            conversation_iter.events.pop_front();
          },
          Event::RemoveParamater(enum_type) => {
            match enum_map.get(enum_type.as_str()) {
              Some(myenum) => {
                let params = myenum.0.values().next().context("EnumType is empty").unwrap();
                for p in params {
                  // FIXME: Remove &'static str
                  warn!("Removing '{}'", p.0);
                  animator.remove_parameter(&p.0.to_string());
                }
              },
              None => warn!("EnumType '{}' doesn't exists!", enum_type),
            }
            animator.remove_parameter(enum_type);
            conversation_iter.events.pop_front();
          },
          Event::SetAnim(name) => {
            match motion_mgr.get(name) {
              Some(motion) => animator.play_motion(motion.clone(), true),
              None => warn!("Animation '{}' not found", name),
            }
            conversation_iter.events.pop_front();
            break;
          },
          ev => {
            debug!("{:#?}", ev);
            conversation_iter.events.pop_front();
            break;
          }
        }
      }
    }
    _ => {}
  }
}

pub fn next_conversation(
  iter: &mut DialogIter,
) {
  let conversation_idx = iter.queue.front();
  if conversation_idx.is_none() {
    return;
  }

  let conversation_iter = conversation_idx.unwrap();

  if conversation_iter.events.is_empty() {
    iter.queue.pop_front();
  }
}
