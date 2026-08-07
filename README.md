# dialoguer
Dialogue system for Rust using OpenGL & Live2D.

## Features
+ Simple human-readable dialog script (See [Dialog Script](#dialog-script)).
+ OpenGL Texture Target.
+ Parameter Changes based on events.
+ Smooth and Fixed Parameter Changes.
+ Visual Editor with Graph and Parameter Editor (See [Editor](#editor)).

## Texture Compression
Use below command for create an ASTC texture.
`.model3.json` must be point to the compressed texture.
```
./external/ASTCEncoder/astcenc-avx2 -cl texture.png texture.astc 4x4 -thorough
```

## Dialog Script
Using `chusmky` (https://docs.rs/chumsky/latest/chumsky/) for parse dialogs.

### Events:
+ Text
+ SetMainChoicer
+ SetAnim
+ Jump
+ SetParameter
+ RemoveParameter
+ Wait
+ Next
+ SetView

```
// This is a comment, doesnt work

// modelname.dialog
[[Choicer]] // Header, either [Conversation] or [[Choicer]]
-> Option 1
  @jump <Conversation> // ID of Header
===

[Conversation]
Player:
  @setmainchoicer [[Choicer]] // [[Choicer]]
  @set AnimType.MyAnim // AnimType.(name of animation file)
  @set MyEnum.Value1 // Set parameter
  @set MyEnum1.NonControl // Remove parameter
  // Setting from up to down
  Hello
NPC:
  Hi!
  @set MyEnum.Value2
  @wait 2
  How are you?
===

// modelname.map
(
  enums:{
    "MyEnum": (
      values: { // It doesnt need to put NonControl
        "Value1": [
          (
            name: "Param51",
            value: Fixed(0.0)
          )
        ],
      }
    ),
   "MyEnum1": (
      values: { // It doesnt need to put NonControl
        "Value1": [
          (
            name: "Param51",
            value: Smooth(
              target: 0.0,
              step: 1.0
            ),
            modification: Some(( // If previous parameter was put
              lhs: "MyEnum",
              rhs: "On",
              then: 50.0
            ))

          )
        ],
      }
    ),
  }
)
```

## Editor
#### UI
<img width="1920" height="1012" alt="image" src="https://github.com/user-attachments/assets/454b84fc-cda1-485d-91c0-97a99ca37028" />

#### Playing
<img width="1920" height="1012" alt="image" src="https://github.com/user-attachments/assets/98b61049-44de-4e53-bc60-e4cfe22de09d" />

#### Container
<img width="1920" height="1012" alt="image" src="https://github.com/user-attachments/assets/508b5613-a7ec-460c-be6f-496060b35787" />

#### Enum Editor
<img width="1920" height="1012" alt="image" src="https://github.com/user-attachments/assets/b4525a78-25b4-4c72-99cf-7c87b9cc04b7" />


