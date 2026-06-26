use chumsky::prelude::*;

#[derive(Debug, PartialEq)]
pub enum Token<'a> {
  Block(Block<'a>),      // Either [MyConversation] or [[MyChoicer]]
  Speaker(&'a str),      // Saya:
  Text(&'a str),         // This is a text
  Choice(&'a str),       // -> My choice!!
  Command(Command<'a>),  // @cmd arg
  End                    // ===
}

#[derive(Debug, PartialEq)]
pub enum Block<'a> {
  Conversation(&'a str), // [MyConversation]
  Choicer(&'a str),      // [[MyChoicer]]
}

#[derive(Debug, PartialEq)]
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
    any()
      .filter(|c: &char| c.is_ascii_alphanumeric() || *c == '-')
      .repeated()
      .at_least(1)
      .to_slice()
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
    none_of("@[]=:\n")
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parsing_number() {
    let res = parse_float().parse("4").into_output();

    assert_eq!(res, Some(4.0));
  }

  #[test]
  fn parsing_zero_with_frac() {
    let res = parse_float().parse("0.2").into_output();

    assert_eq!(res, Some(0.2));
  }

  #[test]
  fn parsing_zero_with_leading_zero_frac() {
    let res = parse_float().parse("0.02").into_output();

    assert_eq!(res, Some(0.02));
  }

  #[test]
  fn parsing_block_conversation() {
    let res = dialog_block_lexer().parse("[Conversation]").into_output();

    assert_eq!(res, Some(vec![Token::Block(Block::Conversation("Conversation"))]));
  }

  #[test]
  fn parsing_block_choicer() {
    let res = dialog_block_lexer().parse("[[Choicer]]").into_output();

    assert_eq!(res, Some(vec![Token::Block(Block::Choicer("Choicer"))]));
  }

  #[test]
  fn parsing_speaker() {
    let res = dialog_block_lexer().parse("Saya-Chan:").into_output();
    let res1 = dialog_block_lexer().parse("Player:").into_output();
    let res2 = dialog_block_lexer().parse("  Kaori   :").into_output();

    assert_eq!(res, Some(vec![Token::Speaker("Saya-Chan")]));
    assert_eq!(res1, Some(vec![Token::Speaker("Player")]));
    assert_eq!(res2, None);
  }

  #[test]
  fn parsing_choice() {
    let res = dialog_block_lexer().parse("-> This is my choice!!!").into_output();

    assert_eq!(res, Some(vec![Token::Choice("This is my choice!!!")]));
  }

  #[test]
  fn parsing_command() {
    let res = dialog_block_lexer().parse("@jump [Conversation]").into_output();
    assert_eq!(res, Some(vec![Token::Command(Command::Jump(Block::Conversation("Conversation")))]));
    let res = dialog_block_lexer().parse("@jump [[Choicer]]").into_output();
    assert_eq!(res, Some(vec![Token::Command(Command::Jump(Block::Choicer("Choicer")))]));
    let res = dialog_block_lexer().parse("@jump").into_output();
    assert_eq!(res, None);
    let res = dialog_block_lexer().parse("@wait 1.3").into_output();
    assert_eq!(res, Some(vec![Token::Command(Command::Wait(1.3))]));
    let res = dialog_block_lexer().parse("@wait").into_output();
    assert_eq!(res, None);
    let res = dialog_block_lexer().parse("@wait a").into_output();
    assert_eq!(res, None);
    let res = dialog_block_lexer().parse("@set Anim.Value").into_output();
    assert_eq!(res, Some(vec![Token::Command(Command::Set{r#enum: "Anim", value: "Value"})]));
    let res = dialog_block_lexer().parse("@set Anim").into_output();
    assert_eq!(res, None);
    let res = dialog_block_lexer().parse("@set").into_output();
    assert_eq!(res, None);
    let res = dialog_block_lexer().parse("@setmainchoicer [Hola]").into_output();
    assert_eq!(res, None);
    let res = dialog_block_lexer().parse("@setmainchoicer").into_output();
    assert_eq!(res, None);
    let res = dialog_block_lexer().parse("@setmainchoicer [[Hola]]").into_output();
    assert_eq!(res, Some(vec![Token::Command(Command::SetMainChoicer("Hola"))]));
    let res = dialog_block_lexer().parse("@next").into_output();
    assert_eq!(res, Some(vec![Token::Command(Command::Next)]));
  }

  #[test]
  fn full_script_parsing() {
      let input = r#"
  [Conversation01]
  saya:
    Hello there!
  player:
    -> First option
    -> Second option
  saya:
    @wait 1.5
    @jump [[Choicer01]]
  player:
    @set Game.state
    @next
  ===
  "#;

      let tokens = dialog_block_lexer()
          .parse(input)
          .into_result()
          .unwrap();

      assert_eq!(
          tokens,
          vec![
              Token::Block(Block::Conversation("Conversation01")),
              
              Token::Speaker("saya"),
              Token::Text("Hello there!"),
              
              Token::Speaker("player"),
              Token::Choice("First option"),
              Token::Choice("Second option"),

              Token::Speaker("saya"),
              Token::Command(Command::Wait(1.5)),
              Token::Command(Command::Jump(Block::Choicer("Choicer01"))),

              Token::Speaker("player"),
              Token::Command(Command::Set {
                  r#enum: "Game",
                  value: "state",
              }),

              Token::Command(Command::Next),
              Token::End,
          ]
      );
  }

  #[test]
  fn pargin_multiblock() {
      let input = r#"
  [Conversation01]
  saya:
    Hello there!
  player:
    Hi!!!
  ===

  [[Choice]]
  -> Option1
  -> Option2
  -> Option3
  ===

  [Conversation02]
  hi:
    Hola
  ===
  "#;

      let tokens = dialog_block_lexer()
          .parse(input)
          .into_result()
          .unwrap();

      assert_eq!(
          tokens,
          vec![
              Token::Block(Block::Conversation("Conversation01")),
              
              Token::Speaker("saya"),
              Token::Text("Hello there!"),
              
              Token::Speaker("player"),
              Token::Text("Hi!!!"),

              Token::End,

              Token::Block(Block::Choicer("Choice")),
              
              Token::Choice("Option1"),
              Token::Choice("Option2"),
              Token::Choice("Option3"),

              Token::End,

              Token::Block(Block::Conversation("Conversation02")),
              
              Token::Speaker("hi"),
              Token::Text("Hola"),

              Token::End,
          ]
      );
  }
}
