use chumsky::prelude::*;

#[derive(Debug)]
pub enum Token<'a> {
  Block(Block<'a>),      // Either [MyConversation] or [[MyChoicer]]
  Speaker(&'a str),      // Saya:
  Text(&'a str),         // This is a text
  Choice(&'a str),       // -> My choice!!
  Command(Command<'a>),  // @cmd arg
  End                    // ===
}

#[derive(Debug)]
pub enum Block<'a> {
  Conversation(&'a str), // [MyConversation]
  Choicer(&'a str),      // [[MyChoicer]]
}

#[derive(Debug)]
pub enum Command<'a> {
  Wait(f32),               // @wait 2.3
  Jump(Block<'a>),         // @jump [Conversation], @jump [[Choicer]]
  Set{                     // @set MyEnum.value 
    r#enum: &'a str,
    value: &'a str,
  },
  SetMainChoicer(&'a str), // @setmainchoicer [[MyChoicer]]
  Next,                    // @next
}

/* This is the parser for a single Dialog Block */
pub fn dialog_block_lexer<'a>() -> impl Parser<'a, &'a str, Vec<Token<'a>>> {
  let parser_block_choicer = 
    just("[[")
      .ignore_then(
        none_of(']')
          .repeated()
          .at_least(1)
          .padded()
          .to_slice()
      )
      .then_ignore(just("]]"));

  let block_choicer = parser_block_choicer.map(Block::Choicer);

  let block_conversation = 
    just('[')
      .ignore_then(
        none_of(']')
          .repeated()
          .at_least(1)
          .padded()
          .to_slice()
      )
      .then_ignore(just(']'))
      .map(Block::Conversation);

  let block = 
    choice((
        block_choicer,
        block_conversation
    ))
    .map(Token::Block);

  let speaker =
    text::ident()
      .padded()
      .then_ignore(just(':'))
      .map(Token::Speaker);

  let option = 
    just("->")
      .padded()
      .ignore_then(
        none_of('\n')
          .repeated()
          .at_least(1)
          .to_slice()
      )
      .map(Token::Choice);

  let end =
    just("===")
      .ignored()
      .map(|_| Token::End);

  // ==========================  
  // COMMANDS
  // ==========================  
  let whitespace = just(' ').repeated().at_least(1);

  let jump = 
    just("jump")
      .then_ignore(whitespace)
      .ignore_then(
        choice((
            block_choicer,
            block_conversation)))
      .padded()
      .map(Command::Jump);

  let wait =
    just("wait")
      .then_ignore(whitespace)
      .ignore_then(parse_float())
      .map(Command::Wait);

  let set =
    just("set")
      .then_ignore(whitespace)
      .ignore_then(
        text::ascii::ident()
          .then_ignore(just('.'))
          .then(text::ascii::ident())
      )
      .map(|(r#enum, value)| Command::Set {r#enum, value});

  let setmainchoicer =
    just("setmainchoicer")
      .then_ignore(whitespace)
      .ignore_then(
        parser_block_choicer
      )
      .map(Command::SetMainChoicer);

  let next =
    just("next")
      .ignored()
      .map(|_| Command::Next);

  let cmd =
    just('@')
      .ignore_then(
        choice((
            jump,
            wait,
            set,
            setmainchoicer,
            next
        ))
      )
      .map(Token::Command);
  // ==========================  
  
  let line = 
    none_of('\n')
      .repeated()
      .at_least(1)
      .to_slice()
      .map(Token::Text);

  choice((
      block,
      speaker,
      option,
      end,
      cmd,
  ))
  .padded()
  .or(line)
  .repeated()
  .collect()
}

fn parse_float<'a>() -> impl Parser<'a, &'a str, f32> {
  let digits = text::digits(10).to_slice();
  let frac = just('.').then(digits);
  
  text::int(10)
    .then(frac.or_not())
    .to_slice()
    .map(|s: &str| s.parse().unwrap())
}
