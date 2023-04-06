use gltf::Glb;
use quote::quote;
use std::{
    io::Cursor,
    path::{Path, PathBuf},
};

fn write_output(path: &Path, data: &[u8]) {
    let path = Path::new("./out").join(path);

    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, data).unwrap();
}

#[derive(Clone, Debug)]
pub struct Model {
    path: PathBuf,
    glb: Glb,
}

impl Model {
    pub fn new(path: &Path, data: &[u8]) -> Self {
        Model {
            path: path.to_owned(),
            glb: Glb::load(data).unwrap(),
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub enum ShaderStage {
    Vertex,
    Fragment,
}

impl From<ShaderStage> for shaderc::ShaderKind {
    fn from(value: ShaderStage) -> Self {
        match value {
            ShaderStage::Vertex => Self::Vertex,
            ShaderStage::Fragment => Self::Fragment,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Shader {
    path: PathBuf,
    stage: ShaderStage,
    code: String,
}

impl Shader {
    pub fn new(path: &Path, data: &[u8]) -> Self {
        let stage = path
            .display()
            .to_string()
            .chars()
            .rev()
            .skip(5)
            .take(4)
            .collect::<String>();
        let stage = stage.chars().rev().collect::<String>();

        let stage = match stage.as_str() {
            "vert" => ShaderStage::Vertex,
            "frag" => ShaderStage::Fragment,
            _ => panic!("Unknown shader stage: {}", stage),
        };

        Self {
            path: path.to_owned(),
            stage,
            code: String::from_utf8(data.to_owned()).unwrap(),
        }
    }

    pub fn compile(self) -> Self {
        let compiler = shaderc::Compiler::new().unwrap();
        let binary = compiler
            .compile_into_spirv(&self.code, self.stage.into(), "", "main", None)
            .unwrap();

        write_output(&self.path.with_extension("spv"), binary.as_binary_u8());

        self
    }

    pub fn codegen(self) {
        let name = self
            .path
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .split(".")
            .map(|part| {
                let part = match part {
                    "vert" => "vertex",
                    "frag" => "fragment",
                    _ => part,
                };

                let mut c = part.chars();
                c.next().unwrap().to_uppercase().collect::<String>() + c.as_str()
            })
            .collect::<Vec<String>>()
            .join("")
            + "Shader";

        let file_name = self
            .path
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .split(".")
            .collect::<Vec<&str>>()
            .join("_")
            + ".rs";

        let stage = format!("{:?}", self.stage);

        let path = Path::new("./out").join(&self.path).with_extension("spv");
        let path = path.to_str().unwrap();

        let header = "use assets_macros::impl_load;
use crate::{Shader, ShaderStage};
use bytemuck::cast_slice;";

        let shader = format!(
            "pub struct {0} {{
    shader: Shader,
}}

impl_load!({0});

impl {0} {{
    fn new() -> Self {{
        let shader = Shader {{
            stage: ShaderStage::{1},
            code: cast_slice::<u8, u32>(&std::fs::read(\"{2}\").unwrap()).to_owned()
        }};

        Self {{ shader }}
    }}
}}
        ",
            name, stage, path
        );

        let code = format!("{}\n{}", header, shader);
        std::fs::write(format!("./src/shaders/{}", file_name), code).unwrap();
    }
}

#[derive(Clone, Debug)]
pub struct Texture {
    path: PathBuf,
    image: image::DynamicImage,
}

impl Texture {
    pub fn new(path: &Path, data: &[u8]) -> Self {
        let img = image::io::Reader::new(Cursor::new(data))
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap();

        Self {
            path: path.to_owned(),
            image: img,
        }
    }

    pub fn compile(&self) {
        let image = self.image.to_rgba8();
        let encoded = qoi::encode_to_vec(image.as_raw(), image.width(), image.height()).unwrap();

        write_output(&self.path.with_extension("qoi"), &encoded);
    }
}
