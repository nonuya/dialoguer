use std::{fs, path::Path};
use anyhow::{Context, ensure};
use glow::HasContext;
use log::debug;

pub fn load_texture_from_astc_path(
  gl: &glow::Context,
  filepath: &Path,
) -> anyhow::Result<glow::Texture> {
  let bytes = fs::read(&filepath).context("Failed to read ASTC Texture")?;
  let astc = parse_astc_from_bytes(&bytes).context("Failed to parse ASTC file")?;
  
  debug!(
    "Texture 2D loaded with w={} h={} with ASTC {}x{} path='{}'",
    astc.dim_x, astc.dim_y, astc.block_x, astc.block_y, filepath.display()
  );

  let texture = unsafe {
    gl.create_texture()
      .map_err(anyhow::Error::msg)
      .context("Failed to create an OpenGL texture")?
  };

  unsafe {
    gl.bind_texture(glow::TEXTURE_2D, Some(texture));

    gl.tex_parameter_i32(
      glow::TEXTURE_2D,
      glow::TEXTURE_WRAP_S,
      glow::CLAMP_TO_EDGE as i32,
    );

    gl.tex_parameter_i32(
      glow::TEXTURE_2D,
      glow::TEXTURE_WRAP_T,
      glow::CLAMP_TO_EDGE as i32,
    );

    gl.tex_parameter_i32(
      glow::TEXTURE_2D,
      glow::TEXTURE_MIN_FILTER,
      glow::LINEAR as i32,
    );

    gl.tex_parameter_i32(
      glow::TEXTURE_2D,
      glow::TEXTURE_MAG_FILTER,
      glow::LINEAR as i32,
    );

    gl.compressed_tex_image_2d(
      glow::TEXTURE_2D,
      0,
      get_gl_compressed_type((astc.block_x, astc.block_y)) as i32,
      astc.dim_x as i32,
      astc.dim_y as i32,
      0,
      astc.bytes.len() as i32,
      astc.bytes,
    );

    gl.bind_texture(glow::TEXTURE_2D, None);
  }

  Ok(texture)
}

struct AstcFile<'a> {
  block_x: u32,
  block_y: u32,
  dim_x: u32,
  dim_y: u32,
  bytes: &'a [u8],
}

fn parse_astc_from_bytes(bytes: &[u8]) -> anyhow::Result<AstcFile<'_>> {
  ensure!(bytes.len() >= 16, "This file isn't an ASTC");
  ensure!(
    bytes[0] == 0x13 && bytes[1] == 0xAB && bytes[2] == 0xA1 && bytes[3] == 0x5C,
    "This file isn't an ASTC. Magic Number is invalid"
  );

  let block_x = bytes[4] as u32;
  let block_y = bytes[5] as u32;
  // let _block_z = bytes[6];

  let dim_x = u32::from_le_bytes([bytes[7], bytes[8], bytes[9], 0]);
  let dim_y = u32::from_le_bytes([bytes[10], bytes[11], bytes[12], 0]);

  // let _dim_z = u32::from_le_bytes([bytes[13], bytes[14], bytes[15], 0]);

  Ok(AstcFile {
    block_x,
    block_y,
    dim_x,
    dim_y,
    bytes: &bytes[16..],
  })
}

const fn get_gl_compressed_type(block: (u32, u32)) -> u32 {
  match block {
    (4, 4) => glow::COMPRESSED_RGBA_ASTC_4x4_KHR,
    (5, 4) => glow::COMPRESSED_RGBA_ASTC_5x4_KHR,
    (5, 5) => glow::COMPRESSED_RGBA_ASTC_5x5_KHR,
    (6, 5) => glow::COMPRESSED_RGBA_ASTC_6x5_KHR,
    (6, 6) => glow::COMPRESSED_RGBA_ASTC_6x6_KHR,
    (8, 5) => glow::COMPRESSED_RGBA_ASTC_8x5_KHR,
    (8, 6) => glow::COMPRESSED_RGBA_ASTC_8x6_KHR,
    (8, 8) => glow::COMPRESSED_RGBA_ASTC_8x8_KHR,
    (10, 5) => glow::COMPRESSED_RGBA_ASTC_10x5_KHR,
    (10, 6) => glow::COMPRESSED_RGBA_ASTC_10x6_KHR,
    (10, 8) => glow::COMPRESSED_RGBA_ASTC_10x8_KHR,
    (10, 10) => glow::COMPRESSED_RGBA_ASTC_10x10_KHR,
    (12, 10) => glow::COMPRESSED_RGBA_ASTC_12x10_KHR,
    (12, 12) => glow::COMPRESSED_RGBA_ASTC_12x12_KHR,
    _ => 0,
  }
}
