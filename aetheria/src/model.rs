use std::io::Cursor;

use bevy_ecs::world::World;
use bytemuck::cast_slice;
use glam::{Mat4, Quat, Vec2, Vec3};
use gltf::Glb;
use image::io::Reader as ImageReader;

use crate::{
    mesh::{
        Mesh, MeshRef, MeshRegistry, Texture, TextureRef, TextureRegistry, Transform, TransformRef,
        TransformRegistry, Vertex,
    },
    renderer::Renderer,
};

pub struct Model {
    glb: Glb,
    texture_refs: Vec<TextureRef>,
}

impl Model {
    fn add_node(&self, glb: &Glb, node: &gltf::Node, world: &mut World, parent_transform: Mat4) {
        let child_transform = if let Some(matrix) = node.matrix {
            Mat4::from_cols_slice(&matrix)
        } else {
            let translation = Vec3::from_slice(&node.translation.unwrap_or([0.0; 3]));
            let rotation = Quat::from_slice(&node.rotation.unwrap_or([0.0, 0.0, 0.0, 1.0]));
            let scale = Vec3::from_slice(&node.scale.unwrap_or([1.0; 3]));
            Mat4::from_scale_rotation_translation(scale, rotation, translation)
        };
        let transform_matrix = parent_transform * child_transform; // NOTE: This might be the wrong way around
        if let Some(mesh) = node.mesh {
            let (scale, rotation, translation) = transform_matrix.to_scale_rotation_translation();
            let mut transform = Transform::new(&mut world.get_resource_mut().unwrap()).unwrap();
            transform.translation = translation;
            transform.rotation = rotation;
            transform.scale = scale;
            transform.update(&world.get_resource().unwrap()).unwrap();

            let transform: TransformRef = world
                .get_resource_mut::<TransformRegistry>()
                .unwrap()
                .add(transform);

            let mesh = glb.gltf.meshes.get(mesh).unwrap();

            mesh.primitives.iter().for_each(|primitive| {
                let color_texture = primitive
                    .material
                    .map(|material| glb.gltf.materials.get(material))
                    .flatten()
                    .map(|material| material.pbr.base_color_texture.as_ref())
                    .flatten()
                    .map(|texture| self.texture_refs.get(texture.index))
                    .flatten()
                    .unwrap_or(&TextureRef::WHITE);

                let positions = primitive.get_attribute_data(glb, "POSITION").unwrap();
                let uvs = primitive.get_attribute_data(glb, "TEXCOORD_0").unwrap();

                let positions = cast_slice::<u8, f32>(&positions)
                    .chunks_exact(3)
                    .map(|slice| Vec3::from_slice(slice))
                    .collect::<Vec<Vec3>>();

                let uvs = cast_slice::<u8, f32>(&uvs)
                    .chunks_exact(2)
                    .map(|slice| Vec2::from_slice(slice))
                    .collect::<Vec<Vec2>>();

                let vertices = std::iter::zip(positions, uvs)
                    .map(|(pos, uv)| Vertex { pos, uv })
                    .collect::<Vec<Vertex>>();
                let indices = primitive.get_indices_data(glb).unwrap();

                let mesh = Mesh::new(
                    &world.get_resource::<Renderer>().unwrap().ctx,
                    &vertices,
                    &indices,
                    Some(*color_texture),
                )
                .unwrap();
                let mesh: MeshRef = world.get_resource_mut::<MeshRegistry>().unwrap().add(mesh);

                world.spawn((mesh, transform));
            })
        }

        node.children.iter().for_each(|child| {
            let child = glb.gltf.nodes.get(*child).unwrap();
            self.add_node(glb, child, world, transform_matrix)
        });
    }

    pub fn load(bytes: &[u8], world: &mut World) -> Self {
        let glb = Glb::load(bytes).expect("Failed to load GLB file");

        let texture_refs: Vec<TextureRef> = glb
            .gltf
            .textures
            .iter()
            .map(|texture| {
                let image = glb.gltf.images.get(texture.source).unwrap();
                let buffer_view = glb
                    .gltf
                    .buffer_views
                    .get(
                        image
                            .buffer_view
                            .expect("Aetheria does not support textures outside of the glb buffer"),
                    )
                    .unwrap();
                let buffer = glb.gltf.buffers.get(buffer_view.buffer).unwrap();

                let bytes = &glb.buffer
                    [buffer_view.byte_offset..(buffer_view.byte_offset + buffer_view.byte_length)];
                let decoded = ImageReader::new(Cursor::new(bytes))
                    .with_guessed_format()
                    .unwrap()
                    .decode()
                    .unwrap();
                let texture_bytes = decoded.to_rgba8().to_vec();
                let texture_bytes =
                    qoi::encode_to_vec(&texture_bytes, decoded.width(), decoded.height()).unwrap(); // This is encoding it just to decode it again, will fix it when I compile glTF files to custom mesh and texture files
                let texture =
                    Texture::new(&mut world.get_resource_mut().unwrap(), &texture_bytes).unwrap();

                world
                    .get_resource_mut::<TextureRegistry>()
                    .unwrap()
                    .add(texture)
            })
            .collect();

        let model = Self { glb, texture_refs };

        let scene = model.glb.gltf.scenes.get(model.glb.gltf.scene).unwrap();
        scene.nodes.iter().for_each(|node| {
            let node = model.glb.gltf.nodes.get(*node).unwrap();
            Self::add_node(&model, &model.glb, node, world, Mat4::IDENTITY)
        });

        model
    }
}
