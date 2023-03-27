use bevy_ecs::world::World;
use bytemuck::cast_slice;
use glam::{Mat4, Quat, Vec2, Vec3};
use gltf::Glb;

use crate::{
    mesh::{
        Mesh, MeshRef, MeshRegistry, TextureRef, Transform, TransformRef, TransformRegistry, Vertex,
    },
    renderer::Renderer,
};

pub struct Model {
    glb: Glb,
}

impl Model {
    fn add_node(
        glb: &Glb,
        node: &gltf::Node,
        world: &mut World,
        parent_transform: Mat4,
        texture: TextureRef,
    ) {
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
                    Some(texture),
                )
                .unwrap();
                let mesh: MeshRef = world.get_resource_mut::<MeshRegistry>().unwrap().add(mesh);

                world.spawn((mesh, transform));
            })
        }

        node.children.iter().for_each(|child| {
            let child = glb.gltf.nodes.get(*child).unwrap();
            Self::add_node(glb, child, world, transform_matrix, texture)
        });
    }

    pub fn load(bytes: &[u8], world: &mut World, texture: TextureRef) -> Self {
        let glb = Glb::load(bytes).expect("Failed to load GLB file");

        let scene = glb.gltf.scenes.get(glb.gltf.scene).unwrap();
        scene.nodes.iter().for_each(|node| {
            let node = glb.gltf.nodes.get(*node).unwrap();
            Self::add_node(&glb, node, world, Mat4::IDENTITY, texture)
        });

        Self { glb }
    }
}
