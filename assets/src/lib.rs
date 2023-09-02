use ash::vk;
use bytemuck::{cast_slice, Pod, Zeroable};
use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Weak},
};
use vulkan::{buffer::Buffer, context::Context, device::Device, graphics::Shader, Texture};
use uuid::Uuid;

pub struct ShaderRegistry {
    registry: HashMap<String, Weak<Shader>>,
}

impl ShaderRegistry {
    pub fn new() -> Self {
        Self {
            registry: HashMap::new(),
        }
    }

    pub fn load(&mut self, device: &Device, path: &str) -> Arc<Shader> {
        let registry_value = self
            .registry
            .get(&path.to_owned())
            .map(|weak| weak.upgrade())
            .flatten();

        match registry_value {
            Some(value) => value,
            None => {
                let spv = Path::new("assets/shaders/compiled")
                    .join(path)
                    .with_extension("spv");
                let stage = match spv
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .split(".")
                    .last()
                    .unwrap()
                {
                    "vert" => vk::ShaderStageFlags::VERTEX,
                    "frag" => vk::ShaderStageFlags::FRAGMENT,
                    "comp" => vk::ShaderStageFlags::COMPUTE,
                    shader_type => panic!("Unexpected shader type: {}", shader_type),
                };
                let code = std::fs::read(spv)
                    .ok()
                    .expect(&format!("Cannot find file: {}", path));
                let shader = Arc::new(Shader::new(device, &code, stage).unwrap());
                self.registry
                    .insert(path.to_owned(), Arc::downgrade(&shader));
                shader
            }
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, Default)]
pub struct Vertex {
    pub pos: Vec3,
    pub _padding: f32,
    pub normal: Vec3,
    pub _padding2: f32,
}

pub struct Model {
    pub meshes: Vec<Mesh>,
}

pub struct Mesh {
    pub id: Uuid,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub color: Vec4,
    pub transform: Transform,
}

#[derive(Clone, Debug)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    pub fn get_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation.normalize(), self.translation)
    }

    pub fn from_matrix(matrix: &Mat4) -> Self {
        let (scale, rotation, translation) = matrix.to_scale_rotation_translation();
        Self {
            translation,
            rotation,
            scale,
        }
    }

    pub fn combine(&self, rhs: &Self) -> Self {
        Self {
            translation: self.translation + rhs.translation,
            rotation: self.rotation * rhs.rotation,
            scale: self.scale * rhs.scale
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

pub struct ModelRegistry {
    registry: HashMap<String, Weak<Model>>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            registry: HashMap::new(),
        }
    }

    pub fn get_models(&self) -> Vec<Arc<Model>> {
        self.registry
            .values()
            .filter_map(|weak| weak.upgrade())
            .collect()
    }

    pub fn load(&mut self, path: &str) -> Arc<Model> {
        fn get_transform(node: &gltf::Node) -> Mat4 {
            if let Some(matrix) = node.matrix {
                Mat4::from_cols_array(&matrix)
            } else {
                let scale = node
                    .scale
                    .map(|arr| Vec3::from_array(arr))
                    .unwrap_or(Vec3::ONE);
                let rotation = node
                    .rotation
                    .map(|arr| Quat::from_array(arr))
                    .unwrap_or(Quat::IDENTITY);
                let translation = node
                    .translation
                    .map(|arr| Vec3::from_array(arr))
                    .unwrap_or(Vec3::ZERO);
                Mat4::from_scale_rotation_translation(scale, rotation, translation)
            }
        }

        fn get_meshes(glb: &gltf::Glb, node: &gltf::Node, parent_transform: Mat4) -> Vec<Mesh> {
            let transform = get_transform(node) * parent_transform;

            let mut meshes = match node.mesh {
                None => Vec::new(),
                Some(mesh) => {
                    let mesh = &glb.gltf.meshes[mesh];
                    mesh.primitives
                        .iter()
                        .map(|primitive| {
                            let color = primitive
                                .material
                                .map(|material| &glb.gltf.materials[material])
                                .and_then(|material| material.pbr.base_color_factor)
                                .map(|arr| Vec4::from_array(arr))
                                .unwrap_or(Vec4::ONE);

                            let indices = primitive.get_indices_data(glb).expect("No indicies");
                            let positions = primitive
                                .get_attribute_data(glb, "POSITION")
                                .expect("No positions");
                            let positions = bytemuck::cast_slice::<u8, Vec3>(&positions).iter().map(|position| *position * 100.0).collect::<Vec<Vec3>>();
                            let normals = primitive
                                .get_attribute_data(glb, "NORMAL")
                                .expect("No normals");
                            let normals = bytemuck::cast_slice::<u8, Vec3>(&normals);
                            let vertices: Vec<Vertex> = std::iter::zip(positions, normals)
                                .map(|(pos, normal)| Vertex {
                                    pos,
                                    normal: *normal,
                                    ..Default::default()
                                })
                                .collect();
                            Mesh {
                                id: Uuid::new_v4(),
                                indices,
                                vertices,
                                color,
                                transform: Transform::from_matrix(&transform),
                            }
                        })
                        .collect()
                }
            };

            meshes.append(
                &mut node
                    .children
                    .iter()
                    .map(|child| &glb.gltf.nodes[*child])
                    .flat_map(|child| get_meshes(glb, child, transform))
                    .collect(),
            );

            meshes
        }

        let registry_value = self
            .registry
            .get(&path.to_owned())
            .map(|weak| weak.upgrade())
            .flatten();

        match registry_value {
            Some(value) => value,
            None => {
                let glb_path = Path::new("assets/meshes").join(path);
                println!("Loading: {}", glb_path.display());

                let glb = gltf::Glb::load(&std::fs::read(glb_path).unwrap()).unwrap();
                let scene = &glb.gltf.scenes[glb.gltf.scene];
                let meshes = scene
                    .nodes
                    .iter()
                    .map(|node| &glb.gltf.nodes[*node])
                    .flat_map(|node| get_meshes(&glb, node, Mat4::IDENTITY))
                    .collect();
                let model = Model { meshes };

                let model = Arc::new(model);
                self.registry
                    .insert(path.to_owned(), Arc::downgrade(&model));
                model
            }
        }
    }
}

pub struct TextureRegistry {
    registry: HashMap<String, Weak<Texture>>,
}

impl TextureRegistry {
    pub fn new() -> Self {
        Self {
            registry: HashMap::new(),
        }
    }

    pub fn get_meshes(&self) -> Vec<Arc<Texture>> {
        self.registry
            .values()
            .filter_map(|weak| weak.upgrade())
            .collect()
    }

    pub fn load(&mut self, ctx: &mut Context, path: &str, normalized_uv: bool) -> Arc<Texture> {
        let registry_value = self
            .registry
            .get(&path.to_owned())
            .map(|weak| weak.upgrade())
            .flatten();

        match registry_value {
            Some(value) => value,
            None => {
                let texture = Path::new("assets/textures/compiled").join(path);
                println!("Loading: {}", texture.display());

                let texture = Arc::new(
                    Texture::new(
                        ctx,
                        &std::fs::read(texture).expect("Failed to read texture"),
                        normalized_uv,
                    )
                    .expect("Failed to read texture"),
                );

                self.registry
                    .insert(path.to_owned(), Arc::downgrade(&texture));
                texture
            }
        }
    }
}
