use std::rc::Rc;

use chumsky::prelude::*;

#[derive(Debug, PartialEq, Clone)]
pub enum Token<'a> {
  Block(Block<'a>),     // Either [MyConversation] or [[MyChoicer]]
  Speaker(&'a str),     // Saya:
  Text(&'a str),        // This is a text
  Choice(&'a str),      // -> My choice!!
  Command(Command<'a>), // @cmd arg
  End,                  // ===
}

#[derive(Debug, PartialEq, Clone)]
pub enum Block<'a> {
  Conversation(&'a str), // [MyConversation]
  Choicer(&'a str),      // [[MyChoicer]]
}

#[derive(Debug, PartialEq, Clone)]
pub enum Command<'a> {
  Wait(f32),       // @wait 2.3
  Jump(&'a str), // @jump <ID>
  Set {
    // @set MyEnum.value
    r#enum: &'a str,
    value: &'a str,
  },
  SetMainChoicer(&'a str), // @setmainchoicer [[MyChoicer]]
  Next,                    // @next
}

type Extra<'a> = extra::Err<Rich<'a, char>>;

/* This is the parser for a single Dialog Block */
pub fn dialog_block_lexer<'a>() -> impl Parser<'a, &'a str, Vec<Token<'a>>, Extra<'a>> {
  let parser_block_choicer = just("[[")
    .ignore_then(none_of(']').repeated().at_least(1).padded().to_slice())
    .then_ignore(just("]]"));

  let block_choicer = parser_block_choicer.map(Block::Choicer);

  let block_conversation = just::<_, _, Extra>('[')
    .ignore_then(none_of(']').repeated().at_least(1).padded().to_slice())
    .then_ignore(just(']'))
    .map(Block::Conversation);

  let block = choice((block_choicer, block_conversation)).map(Token::Block);

  let speaker = any::<&str, Extra>()
    .filter(|c: &char| c.is_ascii_alphanumeric() || *c == '-' || *c == '？')
    .repeated()
    .at_least(1)
    .to_slice()
    .then_ignore(just(':'))
    .map(Token::Speaker);

  let option = just::<_, &str, Extra>("->")
    .padded()
    .ignore_then(none_of('\n').repeated().at_least(1).to_slice())
    .map(Token::Choice);

  let end = just::<_, &str, Extra>("===").ignored().map(|_| Token::End);

  let id = just::<_, _, Extra>('<')
    .ignore_then(none_of('>').repeated().at_least(1).padded().to_slice())
    .then_ignore(just('>'));

  // ==========================
  // COMMANDS
  // ==========================
  let whitespace = just(' ').repeated().at_least(1);

  let jump = just("jump")
    .then_ignore(whitespace)
    .ignore_then(id)
    .map(Command::Jump);

  let wait = just::<_, _, Extra>("wait")
    .then_ignore(whitespace)
    .ignore_then(parse_float())
    .map(Command::Wait);

  let set = just("set")
    .then_ignore(whitespace)
    .ignore_then(
      text::ascii::ident()
        .then_ignore(just('.'))
        .then(text::ascii::ident()),
    )
    .map(|(r#enum, value)| Command::Set { r#enum, value });

  let setmainchoicer = just("setmainchoicer")
    .then_ignore(whitespace)
    .ignore_then(parser_block_choicer)
    .map(Command::SetMainChoicer);

  let next = just("next").ignored().map(|_| Command::Next);

  let cmd = just('@')
    .ignore_then(choice((jump, wait, set, setmainchoicer, next)))
    .map(Token::Command);
  // ==========================

  let line = none_of::<_, _, Extra>("@[]=:\n")
    .repeated()
    .at_least(1)
    .to_slice()
    .map(Token::Text);

  choice((block, speaker, option, end, cmd))
    .padded()
    .or(line)
    .repeated()
    .collect()
}

fn parse_float<'a>() -> impl Parser<'a, &'a str, f32, Extra<'a>> {
  let digits = text::digits(10).to_slice();
  let frac = just('.').then(digits);

  text::int(10)
    .then(frac.or_not())
    .to_slice()
    .map(|s: &str| s.parse().unwrap())
}

#[derive(Debug)]
pub enum Event {
  Text(Rc<str>),
  SetMainChoicer(Rc<str>),
  SetAnim(Rc<str>),
  SetView(Rc<str>),
  Jump(Rc<str>),
  SetParameter(Rc<str>, Rc<str>),
  RemoveParamater(Rc<str>),
  Wait(f32),
  Next,
}

#[derive(Debug)]
pub struct DialogNode {
  pub label: Rc<str>,
  pub events: Vec<Event>,
}

#[derive(Debug)]
pub enum Dialog {
  Conversation(Vec<DialogNode>),
  Choicer(Vec<DialogNode>),
}

pub fn dialog_parser<'a>()
-> impl Parser<'a, &'a [Token<'a>], Vec<(Rc<str>, Dialog)>, extra::Err<Rich<'a, Token<'a>>>> {
  let event = select! {
    Token::Text(text) => Event::Text(text.into()),
    Token::Command(Command::Wait(seconds)) => Event::Wait(seconds),
    Token::Command(Command::Jump(id)) => Event::Jump(id.into()),
    Token::Command(Command::Set { r#enum, value }) => {
      match r#enum {
        "AnimType" => Event::SetAnim(value.into()),
        "ViewType" => Event::SetView(value.into()),
        _ => {
          if value == "NonAction" || value == "NonControl" {
            Event::RemoveParamater(r#enum.into())
          } else {
            Event::SetParameter(r#enum.into(), value.into())
          }
        }
      }
    },
    Token::Command(Command::SetMainChoicer(id)) => Event::SetMainChoicer(id.into()),
    Token::Command(Command::Next) => Event::Next
  };

  let conversation_item = select! {
    Token::Speaker(speaker) => speaker
  }
  .then(event.repeated().collect())
  .map(|(who, events)| DialogNode { label: who.into(), events });

  let conversation_block = select! {
    Token::Block(Block::Conversation(id)) => id
  }
  .then(conversation_item.repeated().collect())
  .then_ignore(select! {
    Token::End => ()
  })
  .map(|(id, items)| (id.into(), Dialog::Conversation(items)));

  let choice_item = select! {
    Token::Choice(label) => label
  }
  // Many things here?
  .then(select! {
    Token::Command(Command::Jump(goto)) => goto.into(),
  })
  .map(|(label, goto)| DialogNode { label: label.into(), events: vec![Event::Jump(goto)] });

  let choicer_block = select! {
    Token::Block(Block::Choicer(id)) => id
  }
  .then(choice_item.repeated().collect())
  .then_ignore(select! {
    Token::End => ()
  })
  .map(|(id, items)| (id.into(), Dialog::Choicer(items)));

  choice((conversation_block, choicer_block))
    .repeated()
    .collect()
}
