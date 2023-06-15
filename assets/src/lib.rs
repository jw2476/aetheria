use std::{collections::HashMap, path::Path, sync::{Arc, Weak}};
use bytemuck::{Pod, Zeroable, cast_slice};
use vulkan::{graphics::Shader, buffer::Buffer, device::Device, context::Context};
use ash::vk;
use glam::{Vec2, Vec3};

pub struct ShaderRegistry {
    registry: HashMap<String, Weak<Shader>>
}

impl ShaderRegistry {
    pub fn new() -> Self {
        Self {
            registry: HashMap::new()
        }
    }

    pub fn load(&mut self, device: &Device, path: &str) -> Arc<Shader> {
        let registry_value = self.registry.get(&path.to_owned()).map(|weak| weak.upgrade()).flatten();

        match registry_value {
            Some(value) => value,
            None => {
                let spv = Path::new("assets/shaders/compiled").join(path).with_extension("spv");
                let stage = match spv.file_stem().unwrap().to_str().unwrap().split(".").last().unwrap() {
                    "vert" => vk::ShaderStageFlags::VERTEX,
                    "frag" => vk::ShaderStageFlags::FRAGMENT,
                    "comp" => vk::ShaderStageFlags::COMPUTE,
                    shader_type => panic!("Unexpected shader type: {}", shader_type)
                };
                let code = std::fs::read(spv).ok().expect(&format!("Cannot find file: {}", path));
                let shader = Arc::new(Shader::new(device, &code, stage).unwrap());
                self.registry.insert(path.to_owned(), Arc::downgrade(&shader));
                shader
            }
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, Default)]
pub struct Vertex {
    pub pos: Vec3,
    pub _padding: f32 
}

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>
}

pub struct MeshRegistry {
    registry: HashMap<String, Weak<Mesh>>
}

impl MeshRegistry {
    pub fn new() -> Self {
        Self {
            registry: HashMap::new()
        }
    }

    pub fn load(&mut self, ctx: &Context, path: &str) -> Arc<Mesh> {
        let registry_value = self.registry.get(&path.to_owned()).map(|weak| weak.upgrade()).flatten();

        match registry_value {
            Some(value) => value,
            None => {
                let obj = Path::new("assets/meshes").join(path);
                println!("Loading: {}", obj.display());

                let (models, _) = tobj::load_obj(obj, &tobj::GPU_LOAD_OPTIONS).unwrap();
                if models.len() != 1 {
                    panic!("Obj file: {} has too many meshes", path);
                }

                let mesh = models.first()
                    .map(|model| &model.mesh)
                    .map(|mesh| { 
                        let positions = mesh.positions
                            .chunks_exact(3)
                            .map(|slice| (Vec3::from_slice(slice) + Vec3::new(1.0, 1.0, 1.0)) * 100.0)
                            .collect::<Vec<Vec3>>();

                        let vertices = positions.iter()
                            .cloned()
                            .map(|pos| Vertex { pos, ..Default::default() })
                            .collect::<Vec<Vertex>>();

                        let indices: Vec<u32> = mesh.indices.chunks_exact(3).flat_map(|slice| vec![slice[0], slice[2], slice[1]]).collect();

                        Mesh { vertices, indices }
                    }).unwrap();

                let mesh = Arc::new(mesh);
                self.registry.insert(path.to_owned(), Arc::downgrade(&mesh));
                mesh
            }
        }
    }
}
