pub mod shaders;

pub enum ShaderStage {
    Vertex,
    Fragment,
}

pub struct Shader {
    pub stage: ShaderStage,
    pub code: Vec<u32>,
}
