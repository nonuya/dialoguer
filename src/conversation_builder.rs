use crate::dialog_parser;
use crate::live2d::animator;
use anyhow::Context;
use chumsky::{
  Parser,
  error::Rich,
  extra,
  primitive::{choice, just},
  select,
};
use cubism::motion::Motion;
use log::debug;

pub struct ConversationBuilder;

struct ChoiceItem {
  label: String,
  goto: JumpEvent,
}

enum JumpEvent {
  Conversation(String),
  Choicer(String),
}

enum Event<'a> {
  Text(String),
  SetMainChoicer(String),
  SetAnim(&'a Motion),
  Jump(JumpEvent),
  SetParameters(Vec<animator::EnumValue>),
  RemoveParamaters(Vec<String>),
  Wait(f32),
  Next,
}

/*
struct ConversationItem {
  who: String,
  events: Vec<Event>,
}

enum Block {
  Conversation {
    id: String,
    items: Vec<ConversationItem>,
  },
  Choicer {
    id: String,
    items: Vec<ChoiceItem>,
  }
}*/

impl ConversationBuilder {
  pub fn new(tokens: Vec<dialog_parser::Token>) {
    let res = dialog_parser::dialog_parser().parse(&tokens).into_result();
    debug!("{:#?}", res);
    // println!("{:#?}", parser().parse(&tokens));
    /*
    for token in tokens {
      match token {
        dialog_parser::Token::Command(cmd) => match cmd {
          dialog_parser::Command::Set { r#enum, value } => {
            if r#enum == "AnimType" {
              match model.get_motions().get(&value.to_string()) {
                Some(motion) => command_queue.push_back(Command::SetAnim(motion.clone())),
                None => warn!("Animation '{}' doesn't exists", value),
              }
            } else if r#enum == "ViewType" {
              warn!("Setting View but isn't implemented yet");
            } else {
              match my_enums.get(r#enum) {
                Some(enum_type) => {
                  if value == "NonControl" || value == "NonAction" {
                    let first = enum_type.0.values().next().context("Enum is empty")?;
                    for p in first {
                      command_queue.push_back(Command::RemoveParamater(p.0.to_string()));
                    }
                  } else {
                    match enum_type.0.get(value) {
                      Some(params) => {
                        for value in params {
                          command_queue
                            .push_back(Command::SetParameter(value.0.to_string(), value.1));
                        }
                      }
                      None => warn!("EnumValue '{}' doesn't exists in Enum '{}'", r#enum, value),
                    }
                  }
                }
                None => warn!("EnumType '{}' doesn't exists in Enum Map", r#enum),
              }
            }
          }
          dialog_parser::Command::Wait(secs) => {
            command_queue.push_back(Command::Wait { remaining: secs })
          }
          _ => {}
        },
        dialog_parser::Token::Text(text) => {
          command_queue.push_back(Command::Text(text.to_string()))
        }
        _ => {}
      }
    }*/
  }
}
