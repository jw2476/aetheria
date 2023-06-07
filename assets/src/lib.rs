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
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub pos: Vec3,
    pub uv: Vec2,
    pub normal: Vec3,
}

pub struct Mesh {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
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
                            .map(|slice| Vec3::from_slice(slice))
                            .collect::<Vec<Vec3>>();

                        let uvs = mesh.texcoords
                            .chunks_exact(2)
                            .map(|slice| Vec2::from_slice(slice))
                            .collect::<Vec<Vec2>>();
                        let normals = mesh.normals
                            .chunks_exact(3)
                            .map(|slice| Vec3::from_slice(slice))
                            .collect::<Vec<Vec3>>();

                        let vertices = std::iter::zip(positions, std::iter::zip(uvs, normals))
                            .map(|(pos, (uv, normal))| Vertex { pos, uv, normal })
                            .collect::<Vec<Vertex>>();


                        let vertex_buffer = Buffer::new(
                            ctx,
                            cast_slice(&vertices),
                            vk::BufferUsageFlags::VERTEX_BUFFER,
                        ).unwrap();
                        let index_buffer =
                            Buffer::new(ctx, cast_slice(&mesh.indices), vk::BufferUsageFlags::INDEX_BUFFER).unwrap();

                        Mesh { vertex_buffer, index_buffer }
                    }).unwrap();

                let mesh = Arc::new(mesh);
                self.registry.insert(path.to_owned(), Arc::downgrade(&mesh));
                mesh
            }
        }
    }
}
