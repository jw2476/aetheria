#![feature(exact_size_is_empty)]

use std::{
    collections::HashMap,
    fmt::Debug,
    io::{Cursor, Read},
};

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Deserialize_repr, Serialize_repr, Debug)]
#[repr(u16)]
pub enum ComponentType {
    I8 = 5120,
    U8 = 5121,
    I16 = 5122,
    U16 = 5123,
    U32 = 5125,
    F32 = 5126,
}

impl ComponentType {
    pub fn size_of(&self) -> usize {
        match self {
            Self::I8 | Self::U8 => 1,
            Self::I16 | Self::U16 => 2,
            Self::U32 | Self::F32 => 4,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Accessor {
    #[serde(rename = "bufferView")]
    pub buffer_view: usize,
    #[serde(default)]
    #[serde(rename = "byteOffset")]
    pub byte_offset: usize,
    #[serde(rename = "componentType")]
    pub component_type: ComponentType,
    #[serde(default)]
    pub normalized: bool,
    pub count: usize,
    #[serde(rename = "type")]
    pub element_type: String,
    #[serde(default)]
    pub max: Option<Vec<f64>>,
    #[serde(default)]
    pub min: Option<Vec<f64>>,
}

impl Accessor {
    pub fn get_data(&self, glb: &Glb) -> Vec<u8> {
        let buffer_view = glb.gltf.buffer_views.get(self.buffer_view).unwrap();
        let buffer = glb.gltf.buffers.get(buffer_view.buffer).unwrap();

        let offset = self.byte_offset + buffer_view.byte_offset;

        let element_size = match self.element_type.as_str() {
            "SCALAR" => 1,
            "VEC2" => 2,
            "VEC3" => 3,
            "VEC4" | "MAT2" => 4,
            "MAT3" => 9,
            "MAT4" => 16,
            _ => panic!("Invalid element type"),
        };
        let size = self.component_type.size_of() * element_size * self.count;

        glb.buffer[offset..(offset + size)].to_vec()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Animation {}

#[derive(Serialize, Deserialize, Debug)]
pub struct Asset {
    #[serde(default)]
    pub copyright: Option<String>,
    #[serde(default)]
    pub generator: Option<String>,
    pub version: String,
    #[serde(default)]
    #[serde(rename = "minVersion")]
    pub min_version: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Buffer {
    #[serde(default)]
    pub uri: String,
    #[serde(rename = "byteLength")]
    pub byte_length: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BufferView {
    pub buffer: usize,
    #[serde(default)]
    #[serde(rename = "byteOffset")]
    pub byte_offset: usize,
    #[serde(rename = "byteLength")]
    pub byte_length: usize,
    #[serde(default)]
    #[serde(rename = "byteStride")]
    pub byte_stride: usize,
    #[serde(default)]
    pub target: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Camera {}

#[derive(Serialize, Deserialize, Debug)]
pub struct Image {
    #[serde(default)]
    pub uri: Option<String>,
    #[serde(default)]
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
    #[serde(default)]
    #[serde(rename = "bufferView")]
    pub buffer_view: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TextureInfo {
    pub index: usize,
    #[serde(default)]
    #[serde(rename = "texCoord")]
    pub tex_coord: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MaterialPBR {
    #[serde(default)]
    #[serde(rename = "baseColorFactor")]
    pub base_color_factor: Option<[f32; 4]>,
    #[serde(default)]
    #[serde(rename = "baseColorTexture")]
    pub base_color_texture: Option<TextureInfo>,
    #[serde(default)]
    #[serde(rename = "metallicFactor")]
    pub metallic_factor: Option<f32>,
    #[serde(default)]
    #[serde(rename = "roughnessFactor")]
    pub roughness_factor: Option<f32>,
    #[serde(default)]
    #[serde(rename = "metallicRoughnessTexture")]
    pub metallic_roughness_texture: Option<TextureInfo>,
}

impl Default for MaterialPBR {
    fn default() -> Self {
        Self {
            base_color_factor: Some([1.0, 1.0, 1.0, 1.0]),
            base_color_texture: None,
            metallic_factor: Some(1.0),
            roughness_factor: Some(1.0),
            metallic_roughness_texture: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MaterialNormalTexture {
    pub index: usize,
    #[serde(default)]
    #[serde(rename = "texCoord")]
    pub tex_coord: usize,
    #[serde(default)]
    pub scale: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MaterialOcclusionTexture {
    pub index: usize,
    #[serde(default)]
    #[serde(rename = "texCoord")]
    pub tex_coord: usize,
    #[serde(default)]
    pub strength: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Material {
    #[serde(default)]
    #[serde(rename = "pbrMetallicRoughness")]
    pub pbr: MaterialPBR,
    #[serde(default)]
    #[serde(rename = "normalTexture")]
    pub normal_texture: Option<MaterialNormalTexture>,
    #[serde(default)]
    #[serde(rename = "occlusionTexture")]
    pub occlusion_texture: Option<MaterialOcclusionTexture>,
    #[serde(default)]
    #[serde(rename = "emissiveTexture")]
    pub emissive_texture: Option<TextureInfo>,
    #[serde(default)]
    #[serde(rename = "emissiveFavtor")]
    pub emissive_factor: Option<[f32; 3]>,
    #[serde(default)]
    #[serde(rename = "alphaMode")]
    pub alpha_mode: Option<String>,
    #[serde(default)]
    #[serde(rename = "alphaCutoff")]
    pub alpha_cutoff: Option<f32>,
    #[serde(default)]
    #[serde(rename = "doubleSided")]
    pub double_sided: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MeshPrimitive {
    pub attributes: HashMap<String, usize>,
    #[serde(default)]
    pub indices: Option<usize>,
    #[serde(default)]
    pub material: Option<usize>,
}

impl MeshPrimitive {
    pub fn get_attribute_data(&self, glb: &Glb, attribute: &str) -> Option<Vec<u8>> {
        glb.gltf
            .accessors
            .get(*self.attributes.get(attribute)?)
            .map(|accessor| accessor.get_data(glb))
    }

    pub fn get_indices_data(&self, glb: &Glb) -> Option<Vec<u32>> {
        glb.gltf.accessors.get(self.indices?).map(|accessor| {
            let data = accessor.get_data(glb);
            match accessor.component_type {
                ComponentType::U16 => bytemuck::cast_slice::<u8, u16>(&data)
                    .iter()
                    .copied()
                    .map(|short| short as u32)
                    .collect(),
                ComponentType::U32 => bytemuck::cast_slice::<u8, u32>(&data).to_vec(),
                _ => panic!("Invalid index component type"),
            }
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Mesh {
    pub primitives: Vec<MeshPrimitive>,
    #[serde(default)]
    pub weights: Option<Vec<f64>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Node {
    #[serde(default)]
    pub camera: Option<usize>,
    #[serde(default)]
    pub children: Vec<usize>,
    #[serde(default)]
    pub skin: Option<usize>,
    #[serde(default)]
    pub matrix: Option<[f32; 16]>,
    #[serde(default)]
    pub mesh: Option<usize>,
    #[serde(default)]
    pub rotation: Option<[f32; 4]>,
    #[serde(default)]
    pub scale: Option<[f32; 3]>,
    #[serde(default)]
    pub translation: Option<[f32; 3]>,
    #[serde(default)]
    pub weights: Option<Vec<f64>>,
}

#[derive(Deserialize_repr, Serialize_repr, Debug)]
#[repr(u16)]
pub enum Filter {
    Nearest = 9728,
    Linear = 9729,
    NearestMipmapNearest = 9984,
    LinearMipmapNearest = 9985,
    NearestMipmapLinear = 9986,
    LinearMipmapLinear = 9987,
}

impl Default for Filter {
    fn default() -> Self {
        Self::Linear
    }
}

#[derive(Deserialize_repr, Serialize_repr, Debug)]
#[repr(u16)]
pub enum AddressMode {
    ClampToEdge = 33071,
    MirroredRepeat = 33648,
    Repeat = 10497,
}

impl Default for AddressMode {
    fn default() -> Self {
        Self::Repeat
    }
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Sampler {
    #[serde(default)]
    #[serde(rename = "magFilter")]
    pub mag_filter: Filter,
    #[serde(default)]
    #[serde(rename = "minFilter")]
    pub min_filter: Filter,
    #[serde(default)]
    #[serde(rename = "wrapS")]
    pub wrap_u: AddressMode,
    #[serde(default)]
    #[serde(rename = "wrapT")]
    pub wrap_v: AddressMode,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Scene {
    #[serde(default)]
    pub nodes: Vec<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Skin {}

#[derive(Serialize, Deserialize, Debug)]
pub struct Texture {
    #[serde(default)]
    pub sampler: Option<usize>,
    pub source: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Gltf {
    #[serde(default)]
    #[serde(rename = "extensionsUsed")]
    pub extensions_used: Vec<String>,
    #[serde(default)]
    #[serde(rename = "extensionsRequired")]
    pub extensions_required: Vec<String>,

    #[serde(default)]
    pub accessors: Vec<Accessor>,
    #[serde(default)]
    pub animations: Vec<Animation>,
    pub asset: Asset,
    #[serde(default)]
    pub buffers: Vec<Buffer>,
    #[serde(default)]
    #[serde(rename = "bufferViews")]
    pub buffer_views: Vec<BufferView>,
    #[serde(default)]
    pub cameras: Vec<Camera>,
    #[serde(default)]
    pub images: Vec<Image>,
    #[serde(default)]
    pub materials: Vec<Material>,
    #[serde(default)]
    pub meshes: Vec<Mesh>,
    #[serde(default)]
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub samplers: Vec<Sampler>,
    #[serde(default)]
    pub scene: usize,
    #[serde(default)]
    pub scenes: Vec<Scene>,
    #[serde(default)]
    pub skins: Vec<Skin>,
    #[serde(default)]
    pub textures: Vec<Texture>,
}

impl Gltf {
    fn load(bytes: &[u8]) -> serde_json::Result<Self> {
        serde_json::from_slice(bytes)
    }
}

pub struct Glb {
    pub gltf: Gltf,
    pub buffer: Vec<u8>,
}

impl Glb {
    fn get_u32(bytes: &mut impl Iterator<Item = u8>) -> u32 {
        *bytemuck::from_bytes::<u32>(&bytes.take(4).collect::<Vec<u8>>())
    }

    fn get(bytes: &mut impl Iterator<Item = u8>, length: usize) -> Vec<u8> {
        bytes.take(length).collect()
    }

    pub fn load(bytes: &[u8]) -> serde_json::Result<Self> {
        let mut bytes = bytes.iter().copied();

        let magic = Self::get_u32(&mut bytes);
        if magic != 0x46546C67 {
            panic!("Malformed GLB");
        }

        let version = Self::get_u32(&mut bytes);
        if version != 2 {
            panic!("Aetheria only supports glTF 2.0");
        }

        let _length = Self::get_u32(&mut bytes);

        let gltf_length = Self::get_u32(&mut bytes);
        let gltf_type = Self::get_u32(&mut bytes);
        if gltf_type != 0x4E4F534A {
            panic!("Malformed GLB");
        }

        let gltf_bytes: Vec<u8> = Self::get(&mut bytes, gltf_length as usize);
        let gltf = Gltf::load(&gltf_bytes)?;

        let mut buffer = Vec::new();
        if !bytes.is_empty() {
            let buffer_length = Self::get_u32(&mut bytes);
            let buffer_type = Self::get_u32(&mut bytes);
            if buffer_type != 0x004E4942 {
                panic!("Malformed GLB");
            }

            buffer = bytes.take(buffer_length as usize).collect();
        }

        Ok(Self { gltf, buffer })
    }
}
