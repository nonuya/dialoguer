use std::rc::Rc;

use crate::{
  dialog::{Dialog, DialogEntryPoint, DialogManager, Event, manager::DialogIter, serializable},
  live2d::animator::MotionManager,
};

enum State {
  Running,
  WaitingInput,
  WaitingChoice(Vec<(Rc<str>, DialogIter)>),
  Finished
}

pub struct DialogPlayer {
  entry_point: DialogEntryPoint,
  iter: Option<DialogIter>,
  state: State,
}

impl DialogPlayer {
  pub fn new(entry_point: DialogEntryPoint) -> Self {
    Self {
      entry_point,
      iter: None,
      state: State::Running,
    }
  }

  pub fn play(&mut self) {
    match &self.entry_point {
      DialogEntryPoint::Choicer(choices) => {
        self.state = State::WaitingChoice(choices.clone());
      },
      DialogEntryPoint::Conversation(iter) => {
        self.iter = Some(iter.clone());
        self.state = State::Running;
      }
    }
  }

  pub fn update(
    &mut self,
    model_definition: &serializable::ModelDefinition,
    dialog_mgr: &DialogManager,
    motion_mgr: &MotionManager,
  ) {
  }
}

enum ConsumeOutcome {
  Advanced, // non blocking events
  Jump(Rc<str>),
  WaitingInput,
  Finished,
}

enum EventEffect {
  SetMainChoicer(Rc<str>),
  RemoveEnum(Rc<str>),
  Text(Rc<str>, Rc<str>),
  SetView(serializable::ViewType),
  SetParameters(Rc<str>, Rc<str>, Vec<serializable::ParamValue>),
  PlayAnimation(Rc<cubism::motion::Motion>, bool),
  Wait(f32),
}

fn consume_iter(
  iter: DialogIter,
  model_definition: &serializable::ModelDefinition,
  dialog_mgr: &DialogManager,
  motion_mgr: &MotionManager,
  onwarning: impl Fn(String),
) -> (DialogIter, ConsumeOutcome, Vec<EventEffect>) {
  let Some(node) = dialog_mgr
    .pool
    .get(iter.dialog_idx)
    .map(Dialog::nodes)
    .and_then(|nodes| nodes.get(iter.node_cursor))
  else {
    return (iter, ConsumeOutcome::Finished, vec![]);
  };

  let event_end = node.events.len();

  let mut effects = Vec::new();
  for i in iter.event_cursor..event_end {
    match &node.events[i] {
      Event::SetMainChoicer(id) => {
        effects.push(EventEffect::SetMainChoicer(id.clone()));
      }
      Event::PlayAnimation(name, looped) => match motion_mgr.get(name) {
        Some(motion) => effects.push(EventEffect::PlayAnimation(motion.clone(), *looped)),
        None => onwarning(format!("[PlayAnimation] Animation '{name}' not found")),
      },
      Event::SetParameter(enumname, valuename) => match model_definition.enums.get(enumname) {
        Some(myenum) => match myenum.values.get(valuename) {
          Some(params) => effects.push(EventEffect::SetParameters(
            enumname.clone(),
            valuename.clone(),
            params.clone(),
          )),
          None => onwarning(format!(
            "[SetParameter] EnumValue '{enumname}' doesnt exists in '{valuename}'"
          )),
        },
        None => onwarning(format!("[SetParameter] Enum '{enumname}' doesnt exists")),
      },
      Event::RemoveEnum(enumname) => {
        if model_definition.enums.contains_key(enumname) {
          effects.push(EventEffect::RemoveEnum(enumname.clone()));
        } else {
          onwarning(format!("[RemoveEnum] Enum doesnt exists '{enumname}'"))
        }
      }
      Event::Text(text) => {
        effects.push(EventEffect::Text(node.label.clone(), text.clone()));
      }
      Event::Wait(seconds) => {
        effects.push(EventEffect::Wait(*seconds));
        return (
          DialogIter {
            event_cursor: i + 1,
            ..iter
          },
          ConsumeOutcome::Advanced,
          effects,
        );
      }
      Event::Jump(id) => {
        return (iter, ConsumeOutcome::Jump(id.clone()), effects);
      }
      Event::Next => {
        let (next_iter, outcome, mut next_effects) = consume_iter(
          DialogIter {
            node_cursor: iter.node_cursor + 1,
            event_cursor: 0,
            ..iter
          },
          model_definition,
          dialog_mgr,
          motion_mgr,
          onwarning,
        );

        effects.append(&mut next_effects);

        return (next_iter, outcome, next_effects);
      }
      Event::SetView(name) => match model_definition.views.get(name) {
        Some(view) => effects.push(EventEffect::SetView(view.clone())),
        None => onwarning(format!("[SetView] View '{name}' not found")),
      },
    }
  }

  (
    DialogIter {
      event_cursor: 0,
      node_cursor: iter.node_cursor + 1,
      ..iter
    },
    ConsumeOutcome::WaitingInput,
    effects,
  )
}
