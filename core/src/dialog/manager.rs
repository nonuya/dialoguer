use std::{collections::HashMap, rc::Rc};

use chumsky::Parser;

use crate::dialog::{Dialog, Token, dialog_parser};

pub struct DialogManager {
  pub(in crate::dialog) pool: Vec<Dialog>,
  pub(in crate::dialog) index_map: HashMap<Rc<str>, usize>,
}

impl DialogManager {
  pub fn from_entries(entries: Vec<(Rc<str>, Dialog)>) -> Self {
    let (pool, index_map) = entries.into_iter().fold(
      (Vec::new(), HashMap::new()),
      |(mut pool, mut index_map), (id, dialog)| {
        let index = pool.len();

        pool.push(dialog);
        assert!(index_map.insert(id, index).is_none(), "Existing Dialog!");

        (pool, index_map)
      },
    );

    Self { pool, index_map }
  }

  pub fn from_tokens(tokens: Vec<Token>) -> anyhow::Result<Self> {
    let entries = dialog_parser()
      .parse(&tokens)
      .into_result()
      .map_err(|err| anyhow::anyhow!("Dialog Block Parser {:#?}", err))?;

    Ok(Self::from_entries(entries))
  }

  pub fn get_dialogs(&self) -> Vec<(&Rc<str>, &Dialog)> {
    self
      .index_map
      .iter()
      .map(|(k, v)| (k, &self.pool[*v]))
      .collect()
  }
}

pub enum DialogEntryPoint {
  Choicer(Vec<(Rc<str>, DialogIter)>),
  Conversation(DialogIter),
}

impl DialogManager {
  pub fn build(&self, id: &str) -> Option<DialogEntryPoint> {
    let idx = self.index_map.get(id)?;
    let d = self.pool.get(*idx)?;

    match d {
      Dialog::Conversation(_) => Some(DialogEntryPoint::Conversation(DialogIter {
        dialog_idx: *idx,
        node_cursor: 0,
        event_cursor: 0,
      })),
      Dialog::Choicer(nodes) => Some(DialogEntryPoint::Choicer(
        nodes
          .iter()
          .enumerate()
          .map(|(i, n)| {
            (
              n.label.clone(),
              DialogIter {
                dialog_idx: *idx,
                node_cursor: i,
                event_cursor: 0,
              },
            )
          })
          .collect(),
      )),
    }
  }
}

#[derive(PartialEq, Clone, Copy)]
pub struct DialogIter {
  pub(in crate::dialog) dialog_idx: usize, // Where is the dialog in the Dialog Pool?
  pub(in crate::dialog) node_cursor: usize, // Who is speaking rn?
  pub(in crate::dialog) event_cursor: usize, // In which event am i?
}
