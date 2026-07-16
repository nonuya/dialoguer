use crate::{
  dialog_parser,
  live2d::{
    self,
    animator::{Animator, Value},
  },
};
use anyhow::Context;
use chumsky::Parser;
use cubism::motion::Motion;
use glam::vec3;
use glutin::display::GlDisplay;
use log::{debug, info, warn};
use std::{
  collections::{HashMap, VecDeque},
  path::PathBuf,
  rc::Rc,
};
use winit::event::KeyEvent;

const SPEAKER_BLOCK: &str = r#"
  @set AnimType.WaistWait01
  @set ViewType.Yori01
  @set BlushType.On
  @set SweatType.On
  @set BreathDisplayType.NonControl
  @set SteamDisplayType.None
  @set ApproachType.Near
  @set EyeBlowType.Normal
  @set EyeChangeType.Normal
  @set EyeType.Smile
  @set EyeBallType.Normal
  @set EyeBallScaleType.Normal
  @set EyeHeartType.Per25
  @set EyeStatusType.Normal
  @set MouthType.Mouth04
  @set PussyType.Normal
  @set PussyMosaicType.NonControl
  @set UnderwearBottomType.On
  @set UnderwearBottomSweatType.NonControl
  @set UnderBodySweatType.NonControl
  @set FloodSemenType.None
  @set ManType.None
  @set ManCockType.Normal
  @set CockSemenType.None
  @set ManTanType.None
  @set ManRightHandType.None
  @set ManLeftHandType.None
  Yay♡ I'm looking forward to it♡
Player:
  @set AnimType.Wait02
  @set EyeBallType.Center04
  @set PussyType.Open
  @set PussyMosaicType.On
  @set ManRightHandType.Open
  @set ManLeftHandType.Open
  First, let's get your pussy loosened up.
Saya-Chan:
  @set EyeType.Close
  @wait 0.15
  @set EyeType.Normal
  @set EyeBallType.Normal
  Oh♡ If we don't loosen up properly, I'll be in danger, right♡
Player:
  Looks like you were ready to go.
Saya-Chan:
  @set EyeType.Half
  I wonder♡ Let's check it with your fingers♡
Player:
  @set UnderwearBottomType.Zurashi
  @set UnderwearBottomType.None
  @set UnderwearBottomSweatType.None
  Well then...
Saya-Chan:
  @set AnimType.Teman01
  @set EyeChangeType.Blush
  @set EyeType.Smile
  @set EyeStatusType.EyeBlush01
  @set MouthType.Mouth07
  @set PussyType.NonAction
  @set ManRightHandType.Teman
  @set ManLeftHandType.None
  @wait 6
  @set EyeType.Normal
  @set MouthType.Mouth04
  Ah♡ Oh♡ I think my pussy might be loosened up♡
Player:
  @set AnimType.Tan02
  @set EyeBallType.Center04
  @set EyeStatusType.EyeBlush01Under
  @set MouthType.Mouth07
  @set ManTanType.On
  @set ManRightHandType.Open
  @set ManLeftHandType.Open
  @wait 6
  And don't forget the tip.
Saya-Chan:
  Ahh♡ That♡ That's where I tend to get stiff♡ Oh♡
Player:
  Sure, it's a bit stiff.
Saya-Chan:
  @set EyeType.Close
  @wait 0.15
  @set EyeType.Normal
  @set EyeBallType.Normal
  @set EyeStatusType.EyeBlush01
  @set MouthType.Mouth02
  L-Loosening up♡ The tip of my pussy is rubbed♡ My pussy is loosening up so much♡
Player:
  @wait 2
  @set AnimType.Wait03
  @set EyeBallType.Center02
  @set MouthType.Mouth09
  @set UnderBodySweatType.On
  @set ManType.On
  @set ManTanType.None
  @set ManRightHandType.Teman
  @set ManLeftHandType.None
  @wait 4
  I think it's time to go.
Player:
  @wait 0.5
  Try inserting it yourself.
Saya-Chan:
  O-Ofey♡ Nnchuu♡ Chuu♡
Saya-Chan:
  @wait 2
  @set AnimType.Waist02
  @set ViewType.Default
  @set ApproachType.Normal
  @set EyeBallType.Center04
  @set EyeHeartType.Per50
  @set EyeStatusType.EyeBlush01Under
  @set MouthType.Mouth04
  @set ManRightHandType.None
  @wait 4
  @set EyeType.Close
  @wait 0.15
  @set EyeType.Normal
  @set EyeBallType.Normal
  @set EyeStatusType.EyeBlush01
  H-Hey, {0}♡ I'm always begging you♡
Saya-Chan:
  @wait 0.5
  I wish {0} would beg me today♡
Player:
  Adults don't beg elementary school kids.
Saya-Chan:
  @set EyeBlowType.Blush01
  @set EyeType.Half
  @set MouthType.Mouth07
  Oh, no♡ Just once♡ Just one time♡
Saya-Chan:
  @set AnimType.Waist03
  @set MouthType.Mouth02
  @wait 4
  Please, pleeeease♡ Huff♡ Huff♡
Player:
  @set EyeType.Close
  @wait 0.15
  @set EyeBlowType.Normal
  @set EyeType.Normal
  @set MouthType.Mouth04
  Damn, you're although in elementary school...!
Player:
  @wait 0.5
  O-Okay.
Player:
  @wait 0.5
  My cock should be in your pussy
Player:
  @wait 0.5
  as soon as possible!
Saya-Chan:
  @set AnimType.Waist05
  @set EyeBallType.Center04
  @set EyeStatusType.EyeBlush01Under
  @set MouthType.Mouth02
  @wait 2
  "Please let it in",  right♡ Strike♡ Strike♡
Player:
  @set EyeType.Close
  @wait 0.15
  @set EyeType.Normal
  @set EyeBallType.Normal
  @set EyeStatusType.EyeBlush01
  @set MouthType.Mouth04
  Ugh...!
Player:
  @wait 0.5
  Oh, please!
Player:
  @wait 0.5
  Please let me put my cock
Player:
  @wait 0.5
  in your elementary school girl's pussy!
Saya-Chan:
  @set EyeType.Half
  You are really begging for a elementary school girl♡ I can't help you♡
Saya-Chan:
  @set AnimType.Waist02
  @set ViewType.Yori01
  @set ApproachType.Near
  @set EyeBallType.Center04
  @set EyeStatusType.EyeBlush01Under
  @set MouthType.Mouth03
  @wait 4
  @set AnimType.Insert01
  @set MouthType.Mouth04
  @wait 1.25
  One two, one two♡ Well, let's have it♡
  @set EyeType.Close
  @wait 0.15
  @set EyeType.Normal
  @set EyeBallType.Center03
  @set EyeStatusType.EyeBlush01Upper
  @wait 1.6
  @set EyeType.Close
  @set MouthType.Mouth07
  @wait 1
  @set EyeChangeType.Normal
  @set EyeType.Normal
  @set EyeBallType.Center02
  @set EyeBallScaleType.Small05
  @set EyeStatusType.Normal
  @set MouthType.Mouth02
  @wait 1
  One two, one two♡ Well, let's have it♡ Woooo♡♡♡♡
  @set EyeType.Close
  @wait 0.15
  @set EyeChangeType.Blush
  @set EyeType.Half
  @set EyeBallType.Center03
  @set EyeBallScaleType.Normal
  @set EyeStatusType.EyeBlush01Upper
  @wait 1.85
  @set EyeType.Normal
  @set EyeBallType.Center04
  @set EyeStatusType.EyeBlush01Under
  @set MouthType.Mouth04
  @wait 0.9
  @set AnimType.InsertWait01
  @set ViewType.Default
  @set ApproachType.Normal
  @wait 4
  @next
Saya-Chan:
  Huff♡ Huff♡
Saya-Chan:
  @set EyeType.Close
  @wait 0.15
  @set EyeType.Normal
  @set EyeBallType.Normal
  @set EyeStatusType.EyeBlush01
  Hey♡ How's my pussy??
Player:
  I-It feels so good!
Saya-Chan:
  @set EyeType.Half
  Aha♡ {0}, you're so cute♡
==="#;

static SPEAKER_BLOCK_2: &str = r#"
[Phase06.2]
  @setmainchoicer [[Phase08]]
Saya-Chan:
  @set AnimType.Piston05
  @set ViewType.Default
  @set BlushType.On
  @set SweatType.On
  @set BreathDisplayType.NonControl
  @set SteamDisplayType.None
  @set ApproachType.Normal
  @set EyeBlowType.Normal
  @set EyeChangeType.Blush
  @set EyeType.Half
  @set EyeBallType.Normal
  @set EyeBallScaleType.Normal
  @set EyeHeartType.Per50
  @set EyeStatusType.EyeBlush01
  @set MouthType.Mouth02
  @set PussyType.OpenInsertCock
  @set PussyMosaicType.None
  @set UnderwearBottomType.None
  @set UnderwearBottomSweatType.None
  @set UnderBodySweatType.On
  @set FloodSemenType.None
  @set ManType.On
  @set ManCockType.Normal
  @set CockSemenType.None
  @set ManTanType.None
  @set ManRightHandType.None
  @set ManLeftHandType.None
  @wait 4
  Heey♡ Are you cuming already♡ We're just getting started♡
Player:
  I can't stand this piston!
Player:
  @wait 0.5
  C-Cuming!
Saya-Chan:
  @set AnimType.PistonFinishWait01
  @set MouthType.Mouth04
  @wait 2
  Non, non, non♡ Don't cum yet♡ Aha♡
Player:
  Oh, no!
Saya-Chan:
  @set EyeBlowType.Blush01
  @set EyeType.Quater
  You are an adult, but your ejaculation is controlled by an elementary school student♡ And you have a pathetic face♡
Saya-Chan:
  @set AnimType.Piston05
  @set EyeType.Half
  @set EyeBallType.Center04
  @set EyeStatusType.EyeBlush01Under
  @wait 4
  @set EyeType.Close
  @wait 0.15
  @set EyeType.Half
  @set EyeBallType.Normal
  @set EyeStatusType.EyeBlush01
  Hey, hey♡ This is good, isn't it♡ Huff♡ Huff♡
Player:
  Wooooo! Cuming! Cuming!
Saya-Chan:
  @set AnimType.PistonFinishWait01
  @set EyeBlowType.Normal
  @set MouthType.Mouth02
  @wait 2
  So, you know♡ You can't just cum without my permission♡
Player:
  Oh, my Gosh, Saya-Chan... That's terrible!
Saya-Chan:
  @set AnimType.Piston02
  @set MouthType.Mouth03
  @wait 2
  @set MouthType.Mouth04
  If you want to cum that badly♡ You know what I mean♡ Huff♡
Player:
  Please let me cum...
Saya-Chan:
  @set EyeType.Quater
  @set MouthType.Mouth02
  No, no, no♡ You're not sincere enough♡ Maybe I should pull it out♡ Huff♡
Player:
  I'm a perverted adult who gets a boner from elementary school girls!
Player:
  @wait 0.5
  Please!! Pleeeease!!
Player:
  @set MouthType.Mouth04
  @wait 0.5
  Please, squeeze the cock semen by your loli pussyyyyy!
Player:
  @wait 0.5
  Pleeeease!!!!
Saya-Chan:
  @set EyeBlowType.Blush01
  All right, all right♡ Just this once♡
Player:
  @set AnimType.Piston05
  @set EyeType.Half
  @wait 4
  Oooohhhhh
Saya-Chan:
  Hey, hey, hey♡ I give you permission♡ You can cum♡
Saya-Chan:
  @wait 0.5
  Your noob cock, get squeezed down by my immature pussy and spurt the semen deep inside♡
Saya-Chan:
  @wait 0.5
  @set EyeBlowType.Normal
  @set EyeType.Quater
  @set MouthType.Mouth02
  Spurt♡ Spurt, Spurt, Spurt♡ Spuuuuurt♡
Player:
  Cuming! Cuming! Cuming! Cuming!
Saya-Chan:
  @set AnimType.PistonFinish02
  @set EyeType.Smile
  @set EyeBallType.Center03
  @set EyeStatusType.EyeBlush01Upper
  @set MouthType.Mouth04
  @set CockSemenType.NonAction
  @wait 0.25
  Oh♡♡♡♡ Aha♡♡♡♡
  @set EyeChangeType.Normal
  @set EyeType.Normal
  @set EyeBallScaleType.Small05
  @set EyeStatusType.Upper
  @set MouthType.Mouth02
  @wait 1.25
  @set EyeBlowType.Blush02
  @set EyeChangeType.Blush
  @set EyeType.Smile
  @set EyeBallType.Normal
  @set EyeBallScaleType.Normal
  @set EyeStatusType.Normal
  @set MouthType.Mouth06
  @wait 1
  @set EyeBlowType.Blush01
  @set EyeType.Half
  @set EyeBallType.Center03
  @set EyeStatusType.EyeBlush01Upper
  @wait 0.75
  @set EyeType.Smile
  @wait 0.15
  Oh♡♡♡♡ Aha♡♡♡♡ It's cuming out sooo much♡♡♡♡ Huff♡♡♡♡
Saya-Chan:
  @set EyeBlowType.Blush02
  @set MouthType.Mouth04
  @wait 0.85
  @set EyeBlowType.Blush01
  @set EyeType.Half
  @set EyeBallScaleType.Small05
  @set MouthType.Mouth05
  @wait 2.25
  @set EyeBallScaleType.Normal
  @set MouthType.Mouth04
  @wait 1.4
  @set AnimType.PistonFinishWait01
  @set EyeBlowType.Normal
  @set EyeBallType.Center04
  @set EyeHeartType.Per75
  @set EyeStatusType.EyeBlush01Under
  @wait 4
  @set EyeType.Close
  @wait 0.15
  @set EyeType.Normal
  @set EyeBallType.Normal
  @set EyeStatusType.EyeBlush01
  Huff♡ Huff♡ Did it feel good, {0}?
Player:
  I-It was so, so good...
Saya-Chan:
  @set EyeBlowType.Blush01
  @set EyeType.Quater
  Aha♡ {0} melty face is so cute♡
===
"#;

static SPEAKER_BLOCK_3: &str = r#"
Player:
  Cuming! Cuming! Cuming! Cuming!
Saya-Chan:
  @set AnimType.PistonFinish02
  @set EyeType.Smile
  @set EyeBallType.Center03
  @set EyeStatusType.EyeBlush01Upper
  @set MouthType.Mouth04
  @set CockSemenType.NonAction
  @wait 0.25
  Oh♡♡♡♡ Aha♡♡♡♡
  @set EyeChangeType.Normal
  @set EyeType.Normal
  @set EyeBallScaleType.Small05
  @set EyeStatusType.Upper
  @set MouthType.Mouth02
  @wait 1.25
  @set EyeBlowType.Blush02
  @set EyeChangeType.Blush
  @set EyeType.Smile
  @set EyeBallType.Normal
  @set EyeBallScaleType.Normal
  @set EyeStatusType.Normal
  @set MouthType.Mouth06
  @wait 1
  @set EyeBlowType.Blush01
  @set EyeType.Half
  @set EyeBallType.Center03
  @set EyeStatusType.EyeBlush01Upper
  @wait 0.75
  @set EyeType.Smile
  @wait 0.15
  Oh♡♡♡♡ Aha♡♡♡♡ It's cuming out sooo much♡♡♡♡ Huff♡♡♡♡
===
"#;

pub struct App {
  gl: Rc<glow::Context>,
  renderer: live2d::Renderer,
  model: live2d::Model,
  mvp: glam::Mat4,
  my_enums: HashMap<&'static str, EnumType>,
  animator: Animator,
  command_queue: VecDeque<Command>,
  clicked: bool,
  once: bool,
}

enum Command {
  Text(String),
  SetAnim(Motion),
  SetParameter(String, Value),
  RemoveParamater(String),
  Wait { remaining: f32 },
}

struct ParamValue(&'static str, Value);
struct EnumType(/*Values*/ HashMap<&'static str, Vec<ParamValue>>);

impl App {
  pub fn new(display: &impl GlDisplay) -> anyhow::Result<Self> {
    let gl = Rc::new(unsafe {
      glow::Context::from_loader_function_cstr(|symbol| display.get_proc_address(symbol))
    });

    let renderer = live2d::Renderer::new(gl.clone())?;
    let model = live2d::Model::new(gl.clone(), PathBuf::from("assets/models/iav_013_2"))?;
    let mut animator = Animator::new();

    let tokens = dialog_parser::dialog_block_lexer()
      .parse(SPEAKER_BLOCK_3)
      .into_result()
      .map_err(|err| anyhow::anyhow!("{:#?}", err))?;

    let my_enums = HashMap::from([
      (
        "BlushType",
        EnumType(HashMap::from([
          (
            "None",
            vec![ParamValue("Param83", Value::smooth(0.0, 1.0))],
          ),
          (
            "Half",
            vec![ParamValue("Param83", Value::smooth(50.0, 1.0))],
          ),
          (
            "On",
            vec![ParamValue("Param83", Value::smooth(100.0, 1.0))],
          ),
        ])),
      ),
      (
        "SweatType",
        EnumType(HashMap::from([
          (
            "None",
            vec![ParamValue("Param53", Value::smooth(0.0, 1.0))],
          ),
          (
            "Half",
            vec![ParamValue("Param53", Value::smooth(50.0, 1.0))],
          ),
          (
            "On",
            vec![ParamValue("Param53", Value::smooth(100.0, 1.0))],
          ),
        ])),
      ),
      (
        "BreathDisplayType",
        EnumType(HashMap::from([(
          "None",
          vec![ParamValue("Param", Value::Fixed(0.0))],
        )])),
      ),
      (
        "SteamDisplayType",
        EnumType(HashMap::from([(
          "None",
          vec![ParamValue("Param2", Value::Fixed(0.0))],
        )])),
      ),
      (
        "ApproachType",
        EnumType(HashMap::from([
          (
            "Normal",
            vec![ParamValue("Param96", Value::smooth(0.0, 0.1))],
          ),
          (
            "Half",
            vec![ParamValue("Param96", Value::smooth(15.0, 0.1))],
          ),
          (
            "Near",
            vec![ParamValue("Param96", Value::smooth(30.0, 0.1))],
          ),
        ])),
      ),
      (
        "EyeBlowType",
        EnumType(HashMap::from([
          (
            "Normal",
            vec![
              ParamValue("Param82", Value::smooth(0.0, 0.5)),
              ParamValue("Param172", Value::smooth(0.0, 0.1)),
            ],
          ),
          (
            "Blush01",
            vec![
              ParamValue("Param82", Value::smooth(1.0, 0.5)),
              ParamValue("Param172", Value::smooth(0.0, 0.1)),
            ],
          ),
          (
            "Blush02",
            vec![
              ParamValue("Param82", Value::smooth(1.0, 0.5)),
              ParamValue("Param172", Value::smooth(30.0, 0.1)),
            ],
          ),
        ])),
      ),
      (
        "EyeChangeType",
        EnumType(HashMap::from([
          (
            "Normal",
            vec![
              ParamValue("Param75", Value::Fixed(0.0)),
              ParamValue("Param77", Value::Fixed(0.0)),
            ],
          ),
          (
            "Blush",
            vec![
              ParamValue("Param75", Value::Fixed(1.0)),
              ParamValue("Param77", Value::Fixed(1.0)),
            ],
          ),
        ])),
      ),
      (
        "EyeType",
        EnumType(HashMap::from([
          (
            "Normal",
            vec![
              ParamValue("Param76", Value::smooth(20.0, 8.0)),
              ParamValue("Param78", Value::smooth(20.0, 8.0)),
            ],
          ),
          (
            "Close",
            vec![
              ParamValue("Param76", Value::smooth(0.0, 8.0)),
              ParamValue("Param78", Value::smooth(0.0, 8.0)),
            ],
          ),
          (
            "Smile",
            vec![
              ParamValue("Param76", Value::smooth(-1.0, 8.0)),
              ParamValue("Param78", Value::smooth(-1.0, 8.0)),
            ],
          ),
          (
            "Half",
            vec![
              ParamValue("Param76", Value::smooth(15.0, 8.0)),
              ParamValue("Param78", Value::smooth(15.0, 8.0)),
            ],
          ),
          (
            "Quater",
            vec![
              ParamValue("Param76", Value::smooth(7.5, 8.0)),
              ParamValue("Param78", Value::smooth(7.5, 8.0)),
            ],
          ),
          (
            "Wink",
            vec![
              ParamValue("Param76", Value::smooth(20.0, 8.0)),
              ParamValue("Param78", Value::smooth(-1.0, 8.0)),
            ],
          ),
          (
            "WinkHalf",
            vec![
              ParamValue("Param76", Value::smooth(15.0, 8.0)),
              ParamValue("Param78", Value::smooth(-1.0, 8.0)),
            ],
          ),
          (
            "WinkQuater",
            vec![
              ParamValue("Param76", Value::smooth(7.5, 8.0)),
              ParamValue("Param78", Value::smooth(-1.0, 8.0)),
            ],
          ),
        ])),
      ),
      (
        "EyeBallType",
        EnumType(HashMap::from([
          (
            "Normal",
            vec![
              ParamValue("Param173", Value::smooth(0.0, 2.0)),
              ParamValue("Param174", Value::smooth(7.5, 2.0)),
              ParamValue("Param176", Value::smooth(0.0, 2.0)),
              ParamValue("Param177", Value::smooth(7.5, 2.0)),
            ],
          ),
          (
            "Center01",
            vec![
              ParamValue("Param173", Value::smooth(0.0, 2.0)),
              ParamValue("Param174", Value::smooth(6.0, 2.0)),
              ParamValue("Param176", Value::smooth(0.0, 2.0)),
              ParamValue("Param177", Value::smooth(6.0, 2.0)),
            ],
          ),
          (
            "Center02",
            vec![
              ParamValue("Param173", Value::smooth(5.0, 2.0)),
              ParamValue("Param174", Value::smooth(12.5, 2.0)),
              ParamValue("Param176", Value::smooth(-5.0, 2.0)),
              ParamValue("Param177", Value::smooth(12.5, 2.0)),
            ],
          ),
          (
            "Center03",
            vec![
              ParamValue("Param173", Value::smooth(10.0, 2.0)),
              ParamValue("Param174", Value::smooth(20.0, 2.0)),
              ParamValue("Param176", Value::smooth(-10.0, 2.0)),
              ParamValue("Param177", Value::smooth(20.0, 2.0)),
            ],
          ),
          (
            "Avert01",
            vec![
              ParamValue("Param173", Value::smooth(15.0, 2.0)),
              ParamValue("Param174", Value::smooth(-10.0, 2.0)),
              ParamValue("Param176", Value::smooth(17.0, 2.0)),
              ParamValue("Param177", Value::smooth(-10.0, 2.0)),
            ],
          ),
          (
            "Avert02",
            vec![
              ParamValue("Param173", Value::smooth(15.0, 2.0)),
              ParamValue("Param174", Value::smooth(10.0, 2.0)),
              ParamValue("Param176", Value::smooth(17.0, 2.0)),
              ParamValue("Param177", Value::smooth(15.0, 2.0)),
            ],
          ),
          (
            "Center04",
            vec![
              ParamValue("Param173", Value::smooth(0.0, 2.0)),
              ParamValue("Param174", Value::smooth(0.0, 2.0)),
              ParamValue("Param176", Value::smooth(0.0, 2.0)),
              ParamValue("Param177", Value::smooth(0.0, 2.0)),
            ],
          ),
        ])),
      ),
      (
        "EyeBallScaleType",
        EnumType(HashMap::from([
          (
            "Normal",
            vec![
              ParamValue("Param175", Value::smooth(0.0, 1.0)),
              ParamValue("Param178", Value::smooth(0.0, 1.0)),
            ],
          ),
          (
            "Small05",
            vec![
              ParamValue("Param175", Value::smooth(5.0, 1.0)),
              ParamValue("Param178", Value::smooth(5.0, 1.0)),
            ],
          ),
          (
            "Small15",
            vec![
              ParamValue("Param175", Value::smooth(15.0, 1.0)),
              ParamValue("Param178", Value::smooth(15.0, 1.0)),
            ],
          ),
        ])),
      ),
      (
        "EyeHeartType",
        EnumType(HashMap::from([
          (
            "Per0",
            vec![ParamValue("Param81", Value::smooth(0.0, 8.0))],
          ),
          (
            "Per25",
            vec![ParamValue("Param81", Value::smooth(25.0, 8.0))],
          ),
          (
            "Per50",
            vec![ParamValue("Param81", Value::smooth(50.0, 8.0))],
          ),
          (
            "Per75",
            vec![ParamValue("Param81", Value::smooth(75.0, 8.0))],
          ),
          (
            "Per100",
            vec![ParamValue("Param81", Value::smooth(100.0, 8.0))],
          ),
        ])),
      ),
      (
        "EyeStatusType",
        EnumType(HashMap::from([
          (
            "Normal",
            vec![
              ParamValue("Param179", Value::smooth(0.0, 8.0)),
              ParamValue("Param181", Value::smooth(0.0, 8.0)),
              ParamValue("Param180", Value::smooth(0.0, 8.0)),
              ParamValue("Param182", Value::smooth(0.0, 8.0)),
              ParamValue("Param185", Value::Fixed(0.0)),
              ParamValue("Param186", Value::Fixed(0.0)),
              ParamValue("Param189", Value::Fixed(0.0)),
              ParamValue("Param191", Value::Fixed(0.0)),
            ],
          ),
          (
            "Upper",
            vec![
              ParamValue("Param179", Value::smooth(-20.0, 8.0)),
              ParamValue("Param181", Value::smooth(-10.0, 8.0)),
              ParamValue("Param180", Value::smooth(20.0, 8.0)),
              ParamValue("Param182", Value::smooth(-10.0, 8.0)),
              ParamValue("Param185", Value::Fixed(-15.0)),
              ParamValue("Param186", Value::Fixed(-15.0)),
              ParamValue("Param189", Value::Fixed(-5.0)),
              ParamValue("Param191", Value::Fixed(-5.0)),
            ],
          ),
          (
            "Under",
            vec![
              ParamValue("Param179", Value::smooth(0.0, 8.0)),
              ParamValue("Param181", Value::smooth(10.0, 8.0)),
              ParamValue("Param180", Value::smooth(0.0, 8.0)),
              ParamValue("Param182", Value::smooth(10.0, 8.0)),
              ParamValue("Param185", Value::Fixed(15.0)),
              ParamValue("Param186", Value::Fixed(15.0)),
              ParamValue("Param189", Value::Fixed(5.0)),
              ParamValue("Param191", Value::Fixed(5.0)),
            ],
          ),
          (
            "EyeBlush01",
            vec![
              ParamValue("Param179", Value::smooth(0.0, 8.0)),
              ParamValue("Param181", Value::smooth(-10.0, 8.0)),
              ParamValue("Param180", Value::smooth(0.0, 8.0)),
              ParamValue("Param182", Value::smooth(-10.0, 8.0)),
              ParamValue("Param185", Value::Fixed(-5.0)),
              ParamValue("Param186", Value::Fixed(-5.0)),
              ParamValue("Param189", Value::Fixed(-5.0)),
              ParamValue("Param191", Value::Fixed(-5.0)),
            ],
          ),
          (
            "EyeBlush01Under",
            vec![
              ParamValue("Param179", Value::smooth(0.0, 8.0)),
              ParamValue("Param181", Value::smooth(-5.0, 8.0)),
              ParamValue("Param180", Value::smooth(0.0, 8.0)),
              ParamValue("Param182", Value::smooth(-5.0, 8.0)),
              ParamValue("Param185", Value::Fixed(10.0)),
              ParamValue("Param186", Value::Fixed(10.0)),
              ParamValue("Param189", Value::Fixed(0.0)),
              ParamValue("Param191", Value::Fixed(0.0)),
            ],
          ),
          (
            "EyeBlush01Upper",
            vec![
              ParamValue("Param179", Value::smooth(0.0, 8.0)),
              ParamValue("Param181", Value::smooth(-20.0, 8.0)),
              ParamValue("Param180", Value::smooth(0.0, 8.0)),
              ParamValue("Param182", Value::smooth(-20.0, 8.0)),
              ParamValue("Param185", Value::Fixed(-30.0)),
              ParamValue("Param186", Value::Fixed(-30.0)),
              ParamValue("Param189", Value::Fixed(-10.0)),
              ParamValue("Param191", Value::Fixed(-10.0)),
            ],
          ),
        ])),
      ),
      (
        "MouthType",
        EnumType(HashMap::from([
          ("Mouth01", vec![ParamValue("Param74", Value::Fixed(0.0))]),
          ("Mouth02", vec![ParamValue("Param74", Value::Fixed(1.0))]),
          ("Mouth03", vec![ParamValue("Param74", Value::Fixed(2.0))]),
          ("Mouth04", vec![ParamValue("Param74", Value::Fixed(3.0))]),
          ("Mouth05", vec![ParamValue("Param74", Value::Fixed(4.0))]),
          ("Mouth06", vec![ParamValue("Param74", Value::Fixed(5.0))]),
          ("Mouth07", vec![ParamValue("Param74", Value::Fixed(6.0))]),
          ("Mouth08", vec![ParamValue("Param74", Value::Fixed(7.0))]),
          ("Mouth09", vec![ParamValue("Param74", Value::Fixed(8.0))]),
        ])),
      ),
      (
        "PussyType",
        EnumType(HashMap::from([
          (
            "Normal",
            vec![
              ParamValue("Param59", Value::smooth(0.0, 0.1)),
              ParamValue("Param60", Value::Fixed(0.0)),
            ],
          ),
          (
            "Open",
            vec![
              ParamValue("Param59", Value::smooth(1.0, 0.1)),
              ParamValue("Param60", Value::Fixed(0.0)),
            ],
          ),
          (
            "OpenInsertCock",
            vec![
              ParamValue("Param59", Value::smooth(1.0, 0.1)),
              ParamValue("Param60", Value::Fixed(1.0)),
            ],
          ),
          (
            "OpenInsertFinger",
            vec![
              ParamValue("Param59", Value::smooth(1.0, 0.1)),
              ParamValue("Param60", Value::Fixed(2.0)),
            ],
          ),
        ])),
      ),
      (
        "PussyMosaicType",
        EnumType(HashMap::from([
          ("None", vec![ParamValue("Param316", Value::Fixed(0.0))]),
          ("On", vec![ParamValue("Param316", Value::Fixed(1.0))]),
        ])),
      ),
      (
        "UnderwearBottomType",
        EnumType(HashMap::from([
          (
            "None",
            vec![
              ParamValue("Param68", Value::Fixed(0.0)),
              ParamValue("Param69", Value::Fixed(-1.0)),
            ],
          ),
          (
            "On",
            vec![
              ParamValue("Param68", Value::Fixed(100.0)),
              ParamValue("Param69", Value::Fixed(0.0)),
            ],
          ),
          (
            "Kuikomi",
            vec![
              ParamValue("Param68", Value::Fixed(100.0)),
              ParamValue("Param69", Value::Fixed(1.0)),
            ],
          ),
          (
            "Zurashi",
            vec![
              ParamValue("Param68", Value::Fixed(100.0)),
              ParamValue("Param69", Value::Fixed(2.0)),
            ],
          ),
        ])),
      ),
      (
        "UnderwearBottomSweatType",
        EnumType(HashMap::from([
          (
            "None",
            vec![ParamValue("Param71", Value::smooth(0.0, 1.0))],
          ),
          (
            "Half",
            vec![ParamValue("Param71", Value::smooth(50.0, 1.0))],
          ),
          (
            "On",
            vec![ParamValue("Param71", Value::smooth(100.0, 1.0))],
          ),
        ])),
      ),
      (
        "UnderBodySweatType",
        EnumType(HashMap::from([
          (
            "None",
            vec![ParamValue("Param52", Value::smooth(0.0, 1.0))],
          ),
          (
            "Half",
            vec![ParamValue("Param52", Value::smooth(50.0, 1.0))],
          ),
          (
            "On",
            vec![ParamValue("Param52", Value::smooth(100.0, 1.0))],
          ),
        ])),
      ),
      (
        "FloodSemenType",
        EnumType(HashMap::from([
          (
            "None",
            vec![
              ParamValue("Param319", Value::smooth(0.0, 8.0)),
              ParamValue("Param320", Value::smooth(-30.0, 8.0)),
            ],
          ),
          (
            "On",
            vec![
              ParamValue("Param319", Value::smooth(100.0, 8.0)),
              ParamValue("Param320", Value::smooth(30.0, 8.0)),
            ],
          ),
        ])),
      ),
      (
        "ManType",
        EnumType(HashMap::from([
          (
            "None",
            vec![
              ParamValue("Param36", Value::Fixed(0.0)),
              ParamValue("Param322", Value::Fixed(0.0)),
              ParamValue("Param325", Value::Fixed(0.0)),
            ],
          ),
          (
            "Clearness",
            vec![
              ParamValue("Param36", Value::Fixed(10.0)),
              ParamValue("Param322", Value::Fixed(1.0)),
              ParamValue("Param325", Value::Fixed(0.0)),
            ],
          ),
          (
            "On",
            vec![
              ParamValue("Param36", Value::Fixed(100.0)),
              ParamValue("Param322", Value::Fixed(1.0)),
              ParamValue("Param325", Value::Fixed(0.0)),
            ],
          ),
        ])),
      ),
      (
        "ManCockType",
        EnumType(HashMap::from([
          ("Normal", vec![ParamValue("Param318", Value::Fixed(0.0))]),
          ("Magari", vec![ParamValue("Param318", Value::Fixed(30.0))]),
        ])),
      ),
      (
        "CockSemenType",
        EnumType(HashMap::from([
          (
            "None",
            vec![
              ParamValue("Param31", Value::Fixed(0.0)),
              ParamValue("Param321", Value::Fixed(-30.0)),
            ],
          ),
          (
            "On",
            vec![
              ParamValue("Param31", Value::Fixed(100.0)),
              ParamValue("Param321", Value::Fixed(30.0)),
            ],
          ),
        ])),
      ),
      (
        "ManTanType",
        EnumType(HashMap::from([
          (
            "None",
            vec![ParamValue("Param37", Value::smooth(0.0, 0.1))],
          ),
          ("On", vec![ParamValue("Param37", Value::smooth(1.0, 0.1))]),
        ])),
      ),
      (
        "ManRightHandType",
        EnumType(HashMap::from([
          (
            "None",
            vec![
              ParamValue("Param47", Value::smooth(0.0, 0.1)),
              ParamValue("Param42", Value::smooth(0.0, 0.1)),
              ParamValue("Param39", Value::smooth(0.0, 0.1)),
            ],
          ),
          (
            "Open",
            vec![
              ParamValue("Param47", Value::smooth(1.0, 0.1)),
              ParamValue("Param42", Value::smooth(0.0, 0.1)),
              ParamValue("Param39", Value::smooth(0.0, 0.1)),
            ],
          ),
          (
            "Teman",
            vec![
              ParamValue("Param47", Value::smooth(0.0, 0.1)),
              ParamValue("Param42", Value::smooth(1.0, 0.1)),
              ParamValue("Param39", Value::smooth(0.0, 0.1)),
            ],
          ),
          (
            "Misetsuke",
            vec![
              ParamValue("Param47", Value::smooth(0.0, 0.1)),
              ParamValue("Param42", Value::smooth(0.0, 0.1)),
              ParamValue("Param39", Value::smooth(1.0, 0.1)),
            ],
          ),
        ])),
      ),
      (
        "ManLeftHandType",
        EnumType(HashMap::from([
          (
            "None",
            vec![ParamValue("Param49", Value::smooth(0.0, 0.1))],
          ),
          (
            "Open",
            vec![ParamValue("Param49", Value::smooth(1.0, 0.1))],
          ),
        ])),
      ),
    ]);

    let mut command_queue = VecDeque::new();

    for token in tokens {
      match token {
        dialog_parser::Token::Command(cmd) => {
          match cmd {
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
                            command_queue.push_back(Command::SetParameter(value.0.to_string(), value.1));
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
            dialog_parser::Command::Wait(secs) => command_queue.push_back(Command::Wait{remaining: secs}),
            _ => {}
          }
        },
        dialog_parser::Token::Text(text) => command_queue.push_back(Command::Text(text.to_string())),
        _ => {}
      }
    }

    Ok(Self {
      gl,
      renderer,
      model,
      mvp: glam::Mat4::from_scale(vec3(2.0, 2.0, 1.0)),
      my_enums,
      animator,
      command_queue,
      clicked: false,
      once: false,
    })
  }

  pub fn update(&mut self, deltatime: f32) {
    loop {
      let Some(cmd) = self.command_queue.front_mut() else {
        break;
      };

      match cmd {
        Command::Text(text) => {
          if !self.once {
            println!("{}", text);
            self.once = true;
          }
          if self.clicked {
            self.command_queue.pop_front();
            self.clicked = false;
            self.once = false;
          }
          break;
        },
        Command::SetAnim(motion) => {
          self.animator.play_motion(motion.clone(), true);
          self.command_queue.pop_front();
        },
        Command::SetParameter(id, value) => {
          self.animator.set_parameter(&id, value.clone());
          self.command_queue.pop_front();
        },
        Command::RemoveParamater(id) => {
          self.animator.remove_parameter(id);
          self.command_queue.pop_front();
        },
        Command::Wait { remaining } => {
          *remaining -= deltatime;

          if *remaining <= 0.0 {
            self.command_queue.pop_front();
          }

          break;
        }
      }
    }

    self.animator.update(deltatime, &mut self.model);
  }

  pub fn draw(&self) {
    self.renderer.draw(&self.model, &self.mvp);
  }

  pub fn resize(&mut self, width: u32, height: u32) {
    self.renderer.resize(width, height);
  }

  pub fn keyboard(&mut self, event: KeyEvent) {
    if event.state.is_pressed() {
      self.clicked = true;
    }
  }
}
