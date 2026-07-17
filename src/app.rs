use std::{collections::HashMap, fs::File, path::PathBuf, rc::Rc};

use chumsky::Parser;
use glam::vec3;
use glutin::display::GlDisplay;
use log::debug;
use winit::{event::KeyEvent, keyboard::{KeyCode, NamedKey, PhysicalKey}};

use crate::{
  dialog::{self},
  live2d::{
    self,
    animator::{self, EnumType, ParamValue, Value},
  },
};

// TODO: Add Speed for Animations

pub struct App {
  gl: Rc<glow::Context>,
  renderer: live2d::Renderer,
  model: live2d::Model,
  mvp: glam::Mat4,
  animator: animator::Animator,
  motion_mgr: animator::MotionManager,
  dialog_mgr: dialog::manager::DialogManager,
  dialog_player: dialog::manager::DialogPlayer,
  my_enums: animator::EnumMap,
}

static SPEAKER_BLOCK_2: &str = include_str!("../model013.dialog");

impl App {
  pub fn new(display: &impl GlDisplay) -> anyhow::Result<Self> {
    let gl = Rc::new(unsafe {
      glow::Context::from_loader_function_cstr(|symbol| display.get_proc_address(symbol))
    });

    let renderer = live2d::Renderer::new(gl.clone())?;

    let model_path = PathBuf::from("assets/models/iav_013_2");
    let model_name = model_path
      .file_name()
      .ok_or_else(|| anyhow::anyhow!("Fail to get model name from '{}'", model_path.display()))?;

    let model_file = File::open(model_path.join(format!("{}.model3.json", model_name.display())))?;
    let model3 = cubism::json::model::Model3::from_reader(model_file)?;

    let model = live2d::Model::new(gl.clone(), &model_path, &model3)?;
    let motion_mgr = live2d::animator::MotionManager::new(&model_path, &model3)?;

    let tokens = dialog::parser::dialog_block_lexer()
      .parse(SPEAKER_BLOCK_2)
      .into_result()
      .map_err(|err| anyhow::anyhow!("Dialog Block Lexer {:#?}", err))?;

    let dialog_mgr = dialog::manager::DialogManager::new(tokens)?;
    let dialog_player = dialog::manager::DialogPlayer::new(dialog_mgr.build("Phase01").unwrap());
    let my_enums = HashMap::from([
      (
        "BlushType",
        EnumType(HashMap::from([
          ("None", vec![ParamValue("Param83", Value::smooth(0.0, 1.0))]),
          (
            "Half",
            vec![ParamValue("Param83", Value::smooth(50.0, 1.0))],
          ),
          ("On", vec![ParamValue("Param83", Value::smooth(100.0, 1.0))]),
        ])),
      ),
      (
        "SweatType",
        EnumType(HashMap::from([
          ("None", vec![ParamValue("Param53", Value::smooth(0.0, 1.0))]),
          (
            "Half",
            vec![ParamValue("Param53", Value::smooth(50.0, 1.0))],
          ),
          ("On", vec![ParamValue("Param53", Value::smooth(100.0, 1.0))]),
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
          ("Per0", vec![ParamValue("Param81", Value::smooth(0.0, 8.0))]),
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
          ("None", vec![ParamValue("Param71", Value::smooth(0.0, 1.0))]),
          (
            "Half",
            vec![ParamValue("Param71", Value::smooth(50.0, 1.0))],
          ),
          ("On", vec![ParamValue("Param71", Value::smooth(100.0, 1.0))]),
        ])),
      ),
      (
        "UnderBodySweatType",
        EnumType(HashMap::from([
          ("None", vec![ParamValue("Param52", Value::smooth(0.0, 1.0))]),
          (
            "Half",
            vec![ParamValue("Param52", Value::smooth(50.0, 1.0))],
          ),
          ("On", vec![ParamValue("Param52", Value::smooth(100.0, 1.0))]),
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
          ("None", vec![ParamValue("Param37", Value::smooth(0.0, 0.1))]),
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
          ("None", vec![ParamValue("Param49", Value::smooth(0.0, 0.1))]),
          ("Open", vec![ParamValue("Param49", Value::smooth(1.0, 0.1))]),
        ])),
      ),
    ]);

    Ok(Self {
      gl,
      renderer,
      model,
      mvp: glam::Mat4::from_scale(vec3(2.0, 2.0, 1.0)),
      my_enums,
      animator: animator::Animator::new(),
      dialog_mgr,
      motion_mgr,
      dialog_player,
    })
  }

  pub fn update(&mut self, deltatime: f32) {
    self.dialog_player.update(
      &self.dialog_mgr,
      &mut self.animator,
      &self.my_enums,
      &self.motion_mgr
    );

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
      match event.physical_key {
        PhysicalKey::Code(KeyCode::KeyI) => {
          self.dialog_player.play();
        },
        PhysicalKey::Code(KeyCode::Space) => {
          self.dialog_player.next();
        },
        PhysicalKey::Code(KeyCode::Digit1) => self.dialog_player.handle_input(0),
        PhysicalKey::Code(KeyCode::Digit2) => self.dialog_player.handle_input(1),
        PhysicalKey::Code(KeyCode::Digit3) => self.dialog_player.handle_input(2),
        PhysicalKey::Code(KeyCode::Digit4) => self.dialog_player.handle_input(3),
        PhysicalKey::Code(KeyCode::Digit5) => self.dialog_player.handle_input(4),
        PhysicalKey::Code(KeyCode::Digit6) => self.dialog_player.handle_input(5),
        _ => {}
      }
    }
      // dialog::manager::next_conversation(&mut self.dialog_iter);
  }
}
