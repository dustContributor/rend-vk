use crate::buffer::{BufferKind, DeviceAllocator, DeviceSlice};
use ash::vk;
use bitvec::vec::BitVec;

use super::VulkanContext;

pub struct DescriptorBuffer {
    pub name: String,
    pub device: DeviceSlice,
    pub layout: vk::DescriptorSetLayout,
    pub descriptor_type: vk::DescriptorType,
    pub descriptor_size: usize,
    occupancy: BitVec,
    host: Box<[u8]>,
}

impl DescriptorBuffer {
    pub fn of(
        ctx: &VulkanContext,
        mem: &mut DeviceAllocator,
        name: String,
        descriptor_type: vk::DescriptorType,
        count: u32,
        is_array: bool,
    ) -> Self {
        assert!(count > 0, "Cant have zero sized descriptor buffers!");
        assert!(
            BufferKind::DESCRIPTOR == mem.buffer.kind,
            "Allocator with kind {} passed, kind {} needed!",
            mem.buffer.kind,
            BufferKind::DESCRIPTOR
        );
        let descriptor_size = Self::size_of(descriptor_type, &ctx.instance, &ctx.physical_device);
        let bindings: Vec<_> = if is_array {
            vec![vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_type(descriptor_type)
                .descriptor_count(count)
                .stage_flags(vk::ShaderStageFlags::ALL_GRAPHICS)
                .build()]
        } else {
            (0..count)
                .into_iter()
                .map(|e| {
                    vk::DescriptorSetLayoutBinding::builder()
                        .binding(e)
                        .descriptor_type(descriptor_type)
                        .descriptor_count(1)
                        .stage_flags(vk::ShaderStageFlags::ALL_GRAPHICS)
                        .build()
                })
                .collect()
        };
        let info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&bindings)
            .flags(vk::DescriptorSetLayoutCreateFlags::DESCRIPTOR_BUFFER_EXT)
            .build();
        let layout = unsafe { ctx.device.create_descriptor_set_layout(&info, None) }.unwrap();
        let layout_size = unsafe {
            ctx.desc_buffer_instance
                .get_descriptor_set_layout_size(layout)
        };
        let boxed: Box<[u8]> = vec![0; layout_size as usize].into_boxed_slice();
        let buffer = if let Some(buffer) = mem.alloc(layout_size) {
            buffer
        } else {
            panic!(
                "Not enough memory for the descriptor! Requested: {}, available: {}",
                layout_size,
                mem.available()
            )
        };
        let occupancy = BitVec::repeat(false, count as usize);
        Self {
            name,
            layout,
            device: buffer,
            host: boxed,
            descriptor_type,
            descriptor_size,
            occupancy,
        }
    }

    fn size_of(
        descriptor_type: vk::DescriptorType,
        instance: &ash::Instance,
        physical_device: &vk::PhysicalDevice,
    ) -> usize {
        let mut props = vk::PhysicalDeviceDescriptorBufferPropertiesEXT {
            ..Default::default()
        };
        let mut device_props = vk::PhysicalDeviceProperties2::builder()
            .push_next(&mut props)
            .build();
        unsafe { instance.get_physical_device_properties2(*physical_device, &mut device_props) };
        match descriptor_type {
            vk::DescriptorType::SAMPLED_IMAGE => props.sampled_image_descriptor_size,
            vk::DescriptorType::UNIFORM_BUFFER => props.uniform_buffer_descriptor_size,
            vk::DescriptorType::SAMPLER => props.sampler_descriptor_size,
            _ => panic!("Unsupported descriptor type {:?}", descriptor_type),
        }
    }

    pub fn place_at(&mut self, index: u32, data: &[u8]) -> usize {
        let offset = self.offset_of(index);
        self.host[offset..(offset + self.descriptor_size)].copy_from_slice(data);
        self.occupancy.set(index as usize, true);
        offset
    }

    pub fn offsets(&self) -> Vec<u64> {
        self.occupancy
            .iter_ones()
            .map(|i| self.device.device_addr + (self.descriptor_size * i) as u64)
            .collect()
    }

    pub fn place(&mut self, data: &[u8]) -> usize {
        self.place_at(self.next_free() as u32, data)
    }

    pub fn next_free(&self) -> usize {
        self.occupancy.first_zero().unwrap()
    }

    pub fn offset_of(&self, index: u32) -> usize {
        index as usize * self.descriptor_size
    }

    pub fn place_sampler(
        &mut self,
        desc: vk::Sampler,
        desc_buffer_instance: &ash::extensions::ext::DescriptorBuffer,
    ) -> usize {
        self.place_sampler_at(self.next_free() as u32, desc, desc_buffer_instance)
    }

    pub fn place_sampler_at(
        &mut self,
        index: u32,
        desc: vk::Sampler,
        desc_buffer_instance: &ash::extensions::ext::DescriptorBuffer,
    ) -> usize {
        self.get_desc_and_place(
            index,
            desc,
            vk::DescriptorType::SAMPLER,
            desc_buffer_instance,
            |e| vk::DescriptorDataEXT { p_sampler: e },
        )
    }

    pub fn place_image(
        &mut self,
        desc: vk::DescriptorImageInfo,
        desc_buffer_instance: &ash::extensions::ext::DescriptorBuffer,
    ) -> usize {
        self.place_image_at(self.next_free() as u32, desc, desc_buffer_instance)
    }

    pub fn place_image_at(
        &mut self,
        index: u32,
        desc: vk::DescriptorImageInfo,
        desc_buffer_instance: &ash::extensions::ext::DescriptorBuffer,
    ) -> usize {
        self.get_desc_and_place(
            index,
            desc,
            vk::DescriptorType::SAMPLED_IMAGE,
            desc_buffer_instance,
            |e| vk::DescriptorDataEXT { p_sampled_image: e },
        )
    }

    pub fn place_ubo(
        &mut self,
        desc: vk::DescriptorAddressInfoEXT,
        desc_buffer_instance: &ash::extensions::ext::DescriptorBuffer,
    ) -> usize {
        self.place_ubo_at(self.next_free() as u32, desc, desc_buffer_instance)
    }

    pub fn place_ubo_at(
        &mut self,
        index: u32,
        desc: vk::DescriptorAddressInfoEXT,
        desc_buffer_instance: &ash::extensions::ext::DescriptorBuffer,
    ) -> usize {
        self.get_desc_and_place(
            index,
            desc,
            vk::DescriptorType::UNIFORM_BUFFER,
            desc_buffer_instance,
            |e| vk::DescriptorDataEXT {
                p_uniform_buffer: e,
            },
        )
    }

    fn get_desc_and_place<T, F>(
        &mut self,
        index: u32,
        desc: T,
        desc_type: vk::DescriptorType,
        desc_buffer_instance: &ash::extensions::ext::DescriptorBuffer,
        info_of: F,
    ) -> usize
    where
        F: FnOnce(*const T) -> vk::DescriptorDataEXT,
    {
        assert!(
            desc_type == self.descriptor_type,
            "Can't place a {:?} on a {:?} buffer!",
            desc_type,
            self.descriptor_type
        );
        let p_desc = &desc as *const T;
        let info_data = info_of(p_desc);
        let info = vk::DescriptorGetInfoEXT {
            ty: desc_type,
            data: info_data,
            ..Default::default()
        };
        let mut data = vec![0; self.descriptor_size];
        unsafe {
            desc_buffer_instance.get_descriptor(&info, &mut data);
        }
        self.place_at(index, &data)
    }

    pub fn into_device(&mut self) {
        let mut device_buffer_slice = unsafe {
            ash::util::Align::new(
                self.device.addr,
                self.descriptor_size as u64,
                self.host.len() as u64,
            )
        };
        device_buffer_slice.copy_from_slice(&self.host);
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_descriptor_set_layout(self.layout, None);
        }
    }
}
