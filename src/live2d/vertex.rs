use glium::implement_vertex;

#[allow(non_snake_case)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
  pub a_position: [f32; 2],
  pub a_texCoord: [f32; 2], // Shaders requires exactly this name
}
implement_vertex!(Vertex, a_position, a_texCoord);
