use super::{Buffer, Device, Texture};
use ash::vk;
use std::{collections::HashMap, ops::Deref, result::Result};

#[derive(Clone, Copy)]
pub struct Binding {
    pub(crate) binding: vk::DescriptorSetLayoutBinding,
}

impl Binding {
    fn new(index: usize, descriptor_type: vk::DescriptorType) -> Self {
        let binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(index.try_into().unwrap())
            .descriptor_type(descriptor_type)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::ALL)
            .build();

        Self { binding }
    }
}

impl Deref for Binding {
    type Target = vk::DescriptorSetLayoutBinding;

    fn deref(&self) -> &Self::Target {
        &self.binding
    }
}

pub struct SetLayoutBuilder<'a> {
    device: &'a Device,
    bindings: Vec<Binding>,
}

impl<'a> SetLayoutBuilder<'a> {
    pub const fn new(device: &'a Device) -> Self {
        Self {
            device,
            bindings: Vec::new(),
        }
    }

    pub fn add(mut self, descriptor_type: vk::DescriptorType) -> Self {
        self.bindings
            .push(Binding::new(self.bindings.len(), descriptor_type));

        self
    }

    pub fn build(self) -> Result<SetLayout, vk::Result> {
        let bindings: Vec<vk::DescriptorSetLayoutBinding> =
            self.bindings.iter().map(|binding| **binding).collect();
        let create_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);

        let layout = unsafe {
            self.device
                .create_descriptor_set_layout(&create_info, None)?
        };

        Ok(SetLayout {
            layout,
            bindings: self.bindings,
        })
    }
}

#[derive(Clone)]
pub struct SetLayout {
    pub(crate) layout: vk::DescriptorSetLayout,
    pub bindings: Vec<Binding>,
}

impl Deref for SetLayout {
    type Target = vk::DescriptorSetLayout;

    fn deref(&self) -> &Self::Target {
        &self.layout
    }
}

#[derive(Clone)]
pub struct Set {
    pub(crate) set: vk::DescriptorSet,
    binding_types: Vec<vk::DescriptorType>
}

impl Set {
    pub fn update_buffer(&self, device: &Device, binding: u32, buffer: &Buffer) {
        let buffer_info = vk::DescriptorBufferInfo::builder()
            .buffer(**buffer)
            .offset(0)
            .range(buffer.size.try_into().unwrap());

        let buffer_infos = &[*buffer_info];
        let write_info = vk::WriteDescriptorSet::builder()
            .dst_set(**self)
            .dst_binding(binding)
            .dst_array_element(0)
            .descriptor_type(self.binding_types[binding as usize])
            .buffer_info(buffer_infos);

        let descriptor_writes = &[*write_info];

        unsafe { device.update_descriptor_sets(descriptor_writes, &[]) };
    }

    pub fn update_texture(
        &self,
        device: &Device,
        binding: u32,
        texture: &Texture,
        layout: vk::ImageLayout,
    ) {
        let image_info = vk::DescriptorImageInfo::builder()
            .sampler(texture.sampler)
            .image_view(texture.view)
            .image_layout(layout);

        let image_infos = &[*image_info];
        let write_info = vk::WriteDescriptorSet::builder()
            .dst_set(**self)
            .dst_binding(binding)
            .dst_array_element(0)
            .descriptor_type(self.binding_types[binding as usize])
            .image_info(image_infos);

        let descriptor_writes = &[*write_info];

        unsafe { device.update_descriptor_sets(descriptor_writes, &[]) };
    }
}

impl Deref for Set {
    type Target = vk::DescriptorSet;

    fn deref(&self) -> &Self::Target {
        &self.set
    }
}

pub struct Pool<'a> {
    pub(crate) pool: vk::DescriptorPool,
    device: &'a Device,
    layout: SetLayout,
    sets: Vec<Set>,
}

impl<'a> Pool<'a> {
    pub fn new(device: &'a Device, layout: SetLayout, capacity: usize) -> Result<Self, vk::Result> {
        let descriptor_types: Vec<vk::DescriptorType> = layout
            .bindings
            .iter()
            .map(|binding| binding.descriptor_type)
            .collect();

        let mut descriptor_type_amounts: HashMap<vk::DescriptorType, usize> = HashMap::new();
        for descriptor_type in &descriptor_types {
            match descriptor_type_amounts.get_mut(descriptor_type) {
                Some(amount) => {
                    *amount += 1;
                }
                None => {
                    descriptor_type_amounts.insert(*descriptor_type, 1);
                }
            }
        }

        let pool_sizes: Vec<vk::DescriptorPoolSize> = descriptor_type_amounts
            .into_iter()
            .map(|(descriptor_type, amount)| {
                vk::DescriptorPoolSize::builder()
                    .ty(descriptor_type)
                    .descriptor_count((amount * capacity).try_into().unwrap())
                    .build()
            })
            .collect();

        let create_info = vk::DescriptorPoolCreateInfo::builder()
            .max_sets(capacity.try_into().unwrap())
            .pool_sizes(&pool_sizes);

        let pool = unsafe { device.create_descriptor_pool(&create_info, None)? };

        Ok(Self {
            pool,
            device,
            layout,
            sets: Vec::new(),
        })
    }

    pub fn allocate(&mut self) -> Result<Set, vk::Result> {
        let set_layouts = &[*self.layout];
        let allocate_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(**self)
            .set_layouts(set_layouts);

        let set = unsafe { self.device.allocate_descriptor_sets(&allocate_info)?[0] };

        Ok(Set {
            set,
            binding_types: self.layout.bindings.iter().map(|binding| binding.descriptor_type).collect(),
        })
    }
}

impl Deref for Pool<'_> {
    type Target = vk::DescriptorPool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}
