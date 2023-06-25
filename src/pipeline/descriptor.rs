use crate::buffer::{BufferKind, DeviceAllocator, DeviceSlice};
use ash::vk;
use bitvec::vec::BitVec;

use crate::context::VulkanContext;

#[derive(Clone)]
pub struct DescriptorBuffer {
    pub name: String,
    pub device: DeviceSlice,
    pub layout: vk::DescriptorSetLayout,
    pub descriptor_type: vk::DescriptorType,
    pub descriptor_size: usize,
    pub count: u32,
    pub subsets: u32,
    subset_size: u32,
    occupancy: BitVec,
    host: Box<[u8]>,
}

fn next_mul_u32(v: u32, mul: u32) -> u32 {
    ((v + mul - 1) / mul) * mul
}

fn next_mul_u64(v: u64, mul: u64) -> u64 {
    ((v + mul - 1) / mul) * mul
}

impl DescriptorBuffer {
    pub fn of(
        ctx: &VulkanContext,
        mem: &mut DeviceAllocator,
        name: String,
        descriptor_type: vk::DescriptorType,
        count: u32,
        subsets: u32,
        is_array: bool,
    ) -> Self {
        assert!(count > 0, "Cant have zero sized descriptor buffers!");
        assert!(
            BufferKind::Descriptor == mem.buffer.kind,
            "Allocator with kind {} passed, kind {} needed!",
            mem.buffer.kind,
            BufferKind::Descriptor
        );
        let descriptor_size = Self::size_of(descriptor_type, &ctx.instance, &ctx.physical_device);
        let subsets = subsets.max(1);
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
        /*
         * Layout buffer size will depend on an individual layout, aligned so immediately after
         * you can bind another one, multiplied by how many "subsets" will be buffered inside this
         * descriptor buffer.
         */
        let subset_size = next_mul_u64(Self::layout_size_of(ctx, layout), mem.alignment()) as u32;
        let buffer_size = subset_size as u64 * subsets as u64;
        let host = vec![0u8; subset_size as usize].into_boxed_slice();
        let device = if let Some(buffer) = mem.alloc(buffer_size) {
            buffer
        } else {
            panic!(
                "Not enough memory for the descriptor! Requested: {}, available: {}",
                buffer_size,
                mem.available()
            )
        };
        let occupancy = BitVec::repeat(false, count as usize);
        ctx.try_set_debug_name(&name, layout);
        Self {
            name,
            layout,
            device,
            subset_size,
            host,
            descriptor_type,
            descriptor_size,
            occupancy,
            count,
            subsets,
        }
    }

    fn layout_size_of(vulkan_context: &VulkanContext, layout: vk::DescriptorSetLayout) -> u64 {
        unsafe {
            vulkan_context
                .extension
                .descriptor_buffer
                .get_descriptor_set_layout_size(layout)
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

    pub fn place_at(&mut self, index: u32, subset: u32, data: &[u8]) -> (usize, u32) {
        let device_offset = self.offset_at(index, subset);
        let host_offset = self.offset_at(index, 0);
        self.host[host_offset..(host_offset + self.descriptor_size)].copy_from_slice(data);
        self.occupancy.set(
            (subset as usize * self.count as usize) + index as usize,
            true,
        );
        (device_offset, index)
    }

    pub fn offsets(&self) -> Vec<u64> {
        self.occupancy
            .iter_ones()
            .map(|i| self.device.device_addr + (self.descriptor_size * i) as u64)
            .collect()
    }

    pub fn place(&mut self, subset: u32, data: &[u8]) -> (usize, u32) {
        self.place_at(self.next_free() as u32, subset, data)
    }

    pub fn next_free(&self) -> usize {
        self.occupancy.first_zero().unwrap()
    }

    pub fn offset_at(&self, index: u32, subset: u32) -> usize {
        (subset as usize * self.subset_size as usize) + (index as usize * self.descriptor_size)
    }

    pub fn remove_at(&mut self, index: u32) {
        self.occupancy.set(index as usize, false);
    }

    pub fn place_sampler(
        &mut self,
        subset: u32,
        desc: vk::Sampler,
        desc_buffer_instance: &ash::extensions::ext::DescriptorBuffer,
    ) -> (usize, u32) {
        self.place_sampler_at(self.next_free() as u32, subset, desc, desc_buffer_instance)
    }

    pub fn place_sampler_at(
        &mut self,
        index: u32,
        subset: u32,
        desc: vk::Sampler,
        desc_buffer_instance: &ash::extensions::ext::DescriptorBuffer,
    ) -> (usize, u32) {
        self.get_desc_and_place(
            index,
            subset,
            desc,
            vk::DescriptorType::SAMPLER,
            desc_buffer_instance,
            |e| vk::DescriptorDataEXT { p_sampler: e },
        )
    }

    pub fn place_image(
        &mut self,
        subset: u32,
        desc: vk::DescriptorImageInfo,
        desc_buffer_instance: &ash::extensions::ext::DescriptorBuffer,
    ) -> (usize, u32) {
        self.place_image_at(self.next_free() as u32, subset, desc, desc_buffer_instance)
    }

    pub fn place_image_at(
        &mut self,
        index: u32,
        subset: u32,
        desc: vk::DescriptorImageInfo,
        desc_buffer_instance: &ash::extensions::ext::DescriptorBuffer,
    ) -> (usize, u32) {
        self.get_desc_and_place(
            index,
            subset,
            desc,
            vk::DescriptorType::SAMPLED_IMAGE,
            desc_buffer_instance,
            |e| vk::DescriptorDataEXT { p_sampled_image: e },
        )
    }

    pub fn place_ubo(
        &mut self,
        subset: u32,
        desc: vk::DescriptorAddressInfoEXT,
        desc_buffer_instance: &ash::extensions::ext::DescriptorBuffer,
    ) -> (usize, u32) {
        self.place_ubo_at(self.next_free() as u32, subset, desc, desc_buffer_instance)
    }

    pub fn place_ubo_at(
        &mut self,
        index: u32,
        subset: u32,
        desc: vk::DescriptorAddressInfoEXT,
        desc_buffer_instance: &ash::extensions::ext::DescriptorBuffer,
    ) -> (usize, u32) {
        self.get_desc_and_place(
            index,
            subset,
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
        subset: u32,
        desc: T,
        desc_type: vk::DescriptorType,
        desc_buffer_instance: &ash::extensions::ext::DescriptorBuffer,
        info_of: F,
    ) -> (usize, u32)
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
        self.place_at(index, subset, &data)
    }

    pub fn into_device(&mut self) {
        self.into_device_at(0)
    }

    pub fn into_device_at(&mut self, subset: u32) {
        assert!(
            subset < self.subsets,
            "Subset {} out of bounds! Total subsets {}",
            subset,
            self.subset_size
        );
        let offset = self.offset_at(0, subset);
        unsafe {
            let src = self.host.as_ptr();
            let dst = self.device.addr.add(offset) as *mut u8;
            let len = self.host.len();
            std::ptr::copy_nonoverlapping(src, dst, len)
        };
    }

    pub fn binding_info(&self) -> vk::DescriptorBufferBindingInfoEXT {
        self.binding_info_at(0)
    }

    pub fn binding_info_at(&self, subset: u32) -> vk::DescriptorBufferBindingInfoEXT {
        let offset = self.offset_at(0, subset) as u64;
        vk::DescriptorBufferBindingInfoEXT::builder()
            .address(self.device.device_addr + offset)
            .usage(self.device.kind.to_vk_usage_flags())
            .build()
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_descriptor_set_layout(self.layout, None);
        }
    }
}
