use std::rc::Rc;

use crate::{
  dialog::{
    Dialog, DialogEntryPoint, DialogManager, Event,
    manager::{ChoiceIter, DialogIter},
    serializable,
  },
  live2d::animator::{Animator, MotionManager, ParameterChange},
};

pub enum PlayerEffect {
  SetMainChoicer(Rc<str>),
  RemoveEnum {
    name: Rc<str>,
    is_deferred: bool,
  },
  Text(Rc<str>, Rc<str>),
  SetView(serializable::ViewType),
  SetParameterChange {
    changes: Vec<ParameterChange>,
    is_deferred: bool,
  },
  PlayAnimation(cubism::motion::Motion, bool),
}

struct ConsumeOutput {
  next_iter: DialogIter,
  result: ConsumeResult,
  effects: Vec<PlayerEffect>,
}

enum ConsumeResult {
  Next,
  Jump(Rc<str>),
  WaitingTime(f32),
  WaitingInput,
  Finished,
}

fn consume_iter(
  iter: DialogIter,
  model_definition: &serializable::ModelDefinition,
  dialog_mgr: &DialogManager,
  motion_mgr: &MotionManager,
  animator: &Animator,
) -> ConsumeOutput {
  type PendingEnum<'a> = (&'a Rc<str>, &'a Rc<str>, &'a Vec<serializable::ParamValue>);

  fn calculate_parameter_changes(
    pending_enums: &Vec<PendingEnum>,
    animator: &Animator,
  ) -> Vec<ParameterChange> {
    pending_enums
      .iter()
      .map(|pending_enum| {
        ParameterChange::from_params(
          pending_enum.0.clone(),
          pending_enum.1.clone(),
          pending_enum.2,
          |lhs, rhs| {
            animator.get_parameter_state().is_enum_active(lhs, rhs)
              || pending_enums
                .iter()
                .find(|e| e.0 == lhs && e.1 == rhs)
                .is_some()
          },
        )
      })
      .collect()
  }

  let Some(node) = dialog_mgr
    .pool
    .get(iter.dialog_idx)
    .map(Dialog::nodes)
    .and_then(|nodes| nodes.get(iter.node_cursor))
  else {
    return ConsumeOutput {
      next_iter: iter,
      result: ConsumeResult::Finished,
      effects: vec![],
    };
  };

  let event_end = node.events.len();

  let mut effects = Vec::new();
  let mut is_deferred = false;
  let mut parameters = Vec::new();
  let create_change_effect = |parameters: &Vec<PendingEnum>, is_deferred: bool| {
    (!parameters.is_empty()).then_some(PlayerEffect::SetParameterChange {
      changes: calculate_parameter_changes(parameters, animator),
      is_deferred,
    })
  };

  for i in iter.event_cursor..event_end {
    match &node.events[i] {
      Event::SetMainChoicer(id) => {
        effects.push(PlayerEffect::SetMainChoicer(id.clone()));
      }
      Event::PlayAnimation(name, looped) => match motion_mgr.get(name) {
        Some(motion) => {
          is_deferred = true;
          effects.push(PlayerEffect::PlayAnimation(motion.clone(), *looped));
        }
        None => log::warn!("[PlayAnimation] Animation '{name}' not found"),
      },
      Event::SetParameter(enumname, valuename) => match model_definition.enums.get(enumname) {
        Some(myenum) => match myenum.values.get(valuename) {
          Some(params) => parameters.push((enumname, valuename, params)),
          None => {
            log::warn!("[SetParameter] EnumValue '{enumname}' doesnt exists in '{valuename}'")
          }
        },
        None => log::warn!("[SetParameter] Enum '{enumname}' doesnt exists"),
      },
      Event::RemoveEnum(enumname) => {
        if model_definition.enums.contains_key(enumname) {
          effects.push(PlayerEffect::RemoveEnum {
            name: enumname.clone(),
            is_deferred,
          });
        } else {
          log::warn!("[RemoveEnum] Enum doesnt exists '{enumname}'");
        }
      }
      Event::Text(text) => {
        effects.push(PlayerEffect::Text(node.label.clone(), text.clone()));
      }
      Event::Wait(seconds) => {
        create_change_effect(&parameters, is_deferred)
          .and_then(|changes| Some(effects.push(changes)));

        return ConsumeOutput {
          next_iter: DialogIter {
            event_cursor: i + 1,
            ..iter
          },
          result: ConsumeResult::WaitingTime(*seconds),
          effects,
        };
      }
      Event::Jump(id) => {
        create_change_effect(&parameters, is_deferred)
          .and_then(|changes| Some(effects.push(changes)));

        return ConsumeOutput {
          next_iter: iter,
          result: ConsumeResult::Jump(id.clone()),
          effects,
        };
      }
      Event::Next => {
        create_change_effect(&parameters, is_deferred)
          .and_then(|changes| Some(effects.push(changes)));

        return ConsumeOutput {
          next_iter: DialogIter {
            node_cursor: iter.node_cursor + 1,
            event_cursor: 0,
            ..iter
          },
          result: ConsumeResult::Next,
          effects,
        };
        /*
        let output = consume_iter(
          DialogIter {
            node_cursor: iter.node_cursor + 1,
            event_cursor: 0,
            ..iter
          },
          model_definition,
          dialog_mgr,
          motion_mgr,
          animator,
        );

        effects.extend(output.effects);

        create_change_effect(&parameters, is_deferred)
          .and_then(|changes| Some(effects.push(changes)));

        return ConsumeOutput {
          next_iter: output.next_iter,
          result: output.result,
          effects,
        };
        */
      }
      Event::SetView(name) => match model_definition.views.get(name) {
        Some(view) => effects.push(PlayerEffect::SetView(view.clone())),
        None => log::warn!("[SetView] View '{name}' not found"),
      },
    }
  }

  create_change_effect(&parameters, is_deferred).and_then(|changes| Some(effects.push(changes)));

  ConsumeOutput {
    next_iter: DialogIter {
      event_cursor: 0,
      node_cursor: iter.node_cursor + 1,
      ..iter
    },
    result: ConsumeResult::WaitingInput,
    effects,
  }
}

pub enum PlayerHandleStatus {
  Idle,
  Next,
  Running,
  ShowChoices(Vec<Rc<str>>),
}

pub struct PlayerHandleOutput {
  pub current_index: usize,
  pub effects: Vec<PlayerEffect>,
  pub status: PlayerHandleStatus,
}

impl PlayerHandleOutput {
  pub fn empty() -> Self {
    Self {
      current_index: 0,
      effects: vec![],
      status: PlayerHandleStatus::Idle,
    }
  }
}

pub enum PlayerState {
  Idle,
  Next(DialogIter),
  WaitingChoice(Vec<ChoiceIter>),
  WaitingInput(DialogIter),
  WaitingTime { iter: DialogIter, remaining: f32 },
  Finished,
}

pub enum PlayerCommand {
  Start,
  // Stop,
  Tick(f32),
  Continue,
  Chose(usize),
}

pub struct DialogPlayer {
  current_dialog_name: Rc<str>,
  entry_point: DialogEntryPoint,
  state: PlayerState,
}

impl DialogPlayer {
  pub fn new(current_dialog_name: Rc<str>, entry_point: DialogEntryPoint) -> Self {
    Self {
      entry_point,
      state: PlayerState::Idle,
      current_dialog_name,
    }
  }

  pub fn is_waiting(&self) -> bool {
    matches!(self.state, PlayerState::WaitingTime { .. })
  }

  pub fn is_next(&self) -> bool {
    matches!(self.state, PlayerState::Next(..))
  }

  pub fn get_current_dialog_name(&self) -> Rc<str> {
    self.current_dialog_name.clone()
  }

  pub fn handle(
    &mut self,
    cmd: PlayerCommand,
    model_definition: &serializable::ModelDefinition,
    dialog_mgr: &DialogManager,
    motion_mgr: &MotionManager,
    animator: &Animator,
  ) -> PlayerHandleOutput {
    match cmd {
      PlayerCommand::Start => match &self.entry_point {
        DialogEntryPoint::Choicer(choices) => {
          self.state = PlayerState::WaitingChoice(choices.clone());
          return PlayerHandleOutput {
            current_index: 0,
            effects: vec![],
            status: PlayerHandleStatus::ShowChoices(
              choices.iter().map(|c| c.label.clone()).collect(),
            ),
          };
        }
        DialogEntryPoint::Conversation(iter) => {
          self.consume(*iter, model_definition, dialog_mgr, motion_mgr, animator)
        }
      },
      PlayerCommand::Chose(index) => {
        let PlayerState::WaitingChoice(ref choices) = self.state else {
          return PlayerHandleOutput::empty();
        };

        let Some(choice) = choices.get(index) else {
          return PlayerHandleOutput::empty();
        };

        self.consume(choice.iter, model_definition, dialog_mgr, motion_mgr, animator)
      }
      PlayerCommand::Continue => {
        let (PlayerState::WaitingInput(iter) | PlayerState::Next(iter)) = self.state else {
          return PlayerHandleOutput::empty();
        };

        self.consume(iter, model_definition, dialog_mgr, motion_mgr, animator)
      }
      PlayerCommand::Tick(deltatime) => {
        if !matches!(self.state, PlayerState::WaitingTime { .. }) {
          return PlayerHandleOutput::empty();
        }

        let PlayerState::WaitingTime {
          iter,
          mut remaining,
        } = std::mem::replace(&mut self.state, PlayerState::Idle)
        else {
          return PlayerHandleOutput::empty();
        };

        remaining -= deltatime;

        if remaining > 0.0 {
          self.state = PlayerState::WaitingTime { iter, remaining };
          return PlayerHandleOutput::empty();
        }

        self.state = PlayerState::Idle;
        self.consume(iter, model_definition, dialog_mgr, motion_mgr, animator)
      } // PlayerCommand::Stop => {}
        // PlayerCommand::Chose(index) => {}
    }
  }

  fn consume(
    &mut self,
    iter: DialogIter,
    model_definition: &serializable::ModelDefinition,
    dialog_mgr: &DialogManager,
    motion_mgr: &MotionManager,
    animator: &Animator,
  ) -> PlayerHandleOutput {
    let output = consume_iter(iter, model_definition, dialog_mgr, motion_mgr, animator);
    match output.result {
      ConsumeResult::Next => {
        self.state = PlayerState::Next(output.next_iter);
        return PlayerHandleOutput {
          current_index: iter.node_cursor,
          effects: output.effects,
          status: PlayerHandleStatus::Next,
        };
      }
      ConsumeResult::Jump(id) => match dialog_mgr.build(&id) {
        Some(entry_point) => {
          let status = match entry_point {
            DialogEntryPoint::Choicer(choices) => {
              let choices_view = choices.iter().map(|c| c.label.clone()).collect();
              self.state = PlayerState::WaitingChoice(choices);
              PlayerHandleStatus::ShowChoices(choices_view)
            }
            DialogEntryPoint::Conversation(iter) => {
              self.state = PlayerState::Next(iter);
              PlayerHandleStatus::Next
            }
          };

          self.current_dialog_name = id.clone();

          return PlayerHandleOutput {
            current_index: iter.node_cursor,
            effects: output.effects,
            status,
          };
        }
        None => {
          log::warn!("[Player] Failed to jumping into {id}");
          return PlayerHandleOutput::empty();
        }
      },
      ConsumeResult::WaitingTime(seconds) => {
        self.state = PlayerState::WaitingTime {
          iter: output.next_iter,
          remaining: seconds,
        };
        return PlayerHandleOutput {
          current_index: iter.node_cursor,
          effects: output.effects,
          status: PlayerHandleStatus::Running,
        };
      }
      ConsumeResult::WaitingInput => {
        self.state = PlayerState::WaitingInput(output.next_iter);
        return PlayerHandleOutput {
          current_index: iter.node_cursor,
          effects: output.effects,
          status: PlayerHandleStatus::Running,
        };
      }
      ConsumeResult::Finished => {
        self.state = PlayerState::Finished;

        return PlayerHandleOutput {
          current_index: iter.node_cursor,
          effects: output.effects,
          status: PlayerHandleStatus::Running,
        };
      }
    }
  }

  pub fn play(&mut self) {
    /*match &self.entry_point {
      DialogEntryPoint::Choicer(choices) => {
        self.state = State::WaitingChoice(choices.clone());
      }
      DialogEntryPoint::Conversation(iter) => {
        self.iter = Some(iter.clone());
        self.state = State::Running;
      }
    }*/
  }

  pub fn update(
    &mut self,
    model_definition: &serializable::ModelDefinition,
    dialog_mgr: &DialogManager,
    motion_mgr: &MotionManager,
  ) {
  }
}
