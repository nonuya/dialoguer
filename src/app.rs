use std::{fs::File, path::PathBuf, rc::Rc};

use chumsky::Parser;
use glam::vec3;
use glutin::display::GlDisplay;
use winit::event::KeyEvent;

use crate::{conversation_builder::ConversationBuilder, dialog_parser, live2d};

/*
let builder: ConversationBuilder;

let conversation: VecDequeue<Dialog> = builder.build();

Dialog {
  DialogBlock {
    who,
    content: VecDequeue
  }
}

*/

static SPEAKER_BLOCK_3: &str = r#"
[Phase06.2]
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
  // model: live2d::Model,
  mvp: glam::Mat4,
  // animator: Animator,
}

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
    // let motion_mgr = live2d::motion_manager::MotionManager::new(&model_path, &model3)?;

    let tokens = dialog_parser::dialog_block_lexer()
      .parse(SPEAKER_BLOCK_3)
      .into_result()
      .map_err(|err| anyhow::anyhow!("{:#?}", err))?;

    ConversationBuilder::new(tokens);

    /*
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
    }*/

    Ok(Self {
      gl,
      renderer,
      // model,
      mvp: glam::Mat4::from_scale(vec3(2.0, 2.0, 1.0)),
      // my_enums,
      // animator: Animator::new(),
    })
  }

  pub fn update(&mut self, deltatime: f32) {
    /*
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
    */

    // self.animator.update(deltatime, &mut self.model);
  }

  pub fn draw(&self) {
    // self.renderer.draw(&self.model, &self.mvp);
  }

  pub fn resize(&mut self, width: u32, height: u32) {
    self.renderer.resize(width, height);
  }

  pub fn keyboard(&mut self, event: KeyEvent) {
  }
}
