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

fn next_mul_u64(v: u64, mul: u64) -> u64 {
    v.div_ceil(mul) * mul
}

impl DescriptorBuffer {
    pub fn of(
        ctx: &VulkanContext,
        desc_buffer_instance: &ash::extensions::ext::DescriptorBuffer,
        mem: &mut DeviceAllocator,
        name: String,
        descriptor_type: vk::DescriptorType,
        count: u32,
        subsets: u32,
        is_array: bool,
    ) -> Self {
        assert!(count > 0, "cant have zero sized descriptor buffers!");
        assert!(
            BufferKind::Descriptor == mem.kind(),
            "allocator with kind {} passed, kind {} needed!",
            mem.kind(),
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
        let subset_size = next_mul_u64(
            Self::layout_size_of(desc_buffer_instance, layout),
            mem.alignment(),
        ) as u32;
        let buffer_size = subset_size as u64 * subsets as u64;
        let host = vec![0u8; subset_size as usize].into_boxed_slice();
        let device = if let Some(buffer) = mem.alloc(buffer_size) {
            buffer
        } else {
            panic!(
                "not enough memory for the descriptor! Requested: {}, available: {}",
                buffer_size,
                mem.available()
            )
        };
        // Clear descriptor memory initially *just in case*. Should be a pretty small write.
        unsafe { std::ptr::write_bytes(device.addr as *mut u8, 0, device.size as usize) };
        // Every descriptor is initially unoccupied
        let occupancy = BitVec::repeat(false, count as usize);
        ctx.try_set_debug_name(&format!("{}_descriptor_buffer_layout", name), layout);
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

    fn layout_size_of(
        desc_buffer_instance: &ash::extensions::ext::DescriptorBuffer,
        layout: vk::DescriptorSetLayout,
    ) -> u64 {
        unsafe { desc_buffer_instance.get_descriptor_set_layout_size(layout) }
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
            vk::DescriptorType::COMBINED_IMAGE_SAMPLER => {
                props.combined_image_sampler_descriptor_size
            }
            vk::DescriptorType::SAMPLED_IMAGE => props.sampled_image_descriptor_size,
            vk::DescriptorType::UNIFORM_BUFFER => props.uniform_buffer_descriptor_size,
            vk::DescriptorType::SAMPLER => props.sampler_descriptor_size,
            _ => panic!("unsupported descriptor type {:?}", descriptor_type),
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

    pub fn place_image_sampler(
        &mut self,
        subset: u32,
        desc: vk::DescriptorImageInfo,
        desc_buffer_instance: &ash::extensions::ext::DescriptorBuffer,
    ) -> (usize, u32) {
        self.place_image_sampler_at(self.next_free() as u32, subset, desc, desc_buffer_instance)
    }

    pub fn place_image_sampler_at(
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
            vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            desc_buffer_instance,
            |e| vk::DescriptorDataEXT {
                p_combined_image_sampler: e,
            },
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

    pub fn read_host(&self) -> Vec<u8> {
        self.host.to_vec()
    }

    pub fn read_device(&self) -> Vec<u8> {
        let slice = unsafe {
            std::slice::from_raw_parts(self.device.addr as *const u8, self.device.size as usize)
        };
        slice.to_vec()
    }

    pub fn into_device(&mut self) {
        self.into_device_at(0)
    }

    pub fn into_device_at(&mut self, subset: u32) {
        assert!(
            subset < self.subsets,
            "subset {} out of bounds! total {}",
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

    pub fn into_device_single(&mut self, index: u32) {
        self.into_device_single_at(0, index)
    }

    pub fn into_device_single_at(&mut self, subset: u32, index: u32) {
        assert!(
            subset < self.subsets,
            "subset {} out of bounds! total {}",
            subset,
            self.subset_size
        );
        assert!(
            index < self.count,
            "index {} out of bounds! total {}",
            index,
            self.count
        );
        let device_offset = self.offset_at(index, subset);
        let host_offset = self.offset_at(index, 0);
        unsafe {
            let src = self.host.as_ptr().add(host_offset);
            let dst = self.device.addr.add(device_offset) as *mut u8;
            let len = self.descriptor_size;
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

#[derive(Clone)]
pub struct DescriptorGroup {
    pub name: String,
    pub layout: vk::DescriptorSetLayout,
    pub set: vk::DescriptorSet,
    pub descriptor_type: vk::DescriptorType,
    pub capacity: u32,
    pub is_array: bool,
    occupancy: BitVec,
}

pub const MAX_DESCRIPTOR_SETS: u32 = 16;
pub const MAX_DESCRIPTOR_IMAGE: u32 = 2048;
pub const MAX_DESCRIPTOR_IMAGE_SAMPLER: u32 = 64;
pub const MAX_DESCRIPTOR_SAMPLER: u32 = 32;

pub fn make_pool(ctx: &VulkanContext, is_dynamic: bool) -> vk::DescriptorPool {
    let image_sampler_size = vk::DescriptorPoolSize {
        descriptor_count: MAX_DESCRIPTOR_IMAGE_SAMPLER,
        ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
    };
    let image_size = vk::DescriptorPoolSize {
        descriptor_count: MAX_DESCRIPTOR_IMAGE,
        ty: vk::DescriptorType::SAMPLED_IMAGE,
    };
    let sampler_size = vk::DescriptorPoolSize {
        descriptor_count: MAX_DESCRIPTOR_SAMPLER,
        ty: vk::DescriptorType::SAMPLER,
    };
    let pool_sizes = [image_sampler_size, image_size, sampler_size];
    let info = vk::DescriptorPoolCreateInfo::builder()
        .flags(if is_dynamic {
            vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND
        } else {
            vk::DescriptorPoolCreateFlags::empty()
        })
        .max_sets(MAX_DESCRIPTOR_SETS)
        .pool_sizes(&pool_sizes)
        .build();
    // create pool and return
    unsafe { ctx.device.create_descriptor_pool(&info, None).unwrap() }
}

impl DescriptorGroup {
    pub fn of(
        ctx: &VulkanContext,
        pool: vk::DescriptorPool,
        name: String,
        descriptor_type: vk::DescriptorType,
        capacity: u32,
        is_array: bool,
        is_dynamic: bool,
    ) -> Self {
        assert!(capacity > 0, "cant have zero sized descriptor groups!");
        let bindings: Vec<_> = if is_array {
            vec![vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_type(descriptor_type)
                .descriptor_count(capacity)
                .stage_flags(vk::ShaderStageFlags::ALL_GRAPHICS)
                .build()]
        } else {
            (0..capacity)
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
        let mut info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&bindings)
            .flags(if is_dynamic {
                vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL
            } else {
                vk::DescriptorSetLayoutCreateFlags::empty()
            });
        let binding_flags_count = if is_array { 1 } else { capacity };
        let binding_flags: Vec<_> = (0..binding_flags_count)
            .map(|_| {
                vk::DescriptorBindingFlags::UPDATE_AFTER_BIND
                    | vk::DescriptorBindingFlags::PARTIALLY_BOUND
                    | vk::DescriptorBindingFlags::UPDATE_UNUSED_WHILE_PENDING
            })
            .collect();
        let mut binding_flags_info = vk::DescriptorSetLayoutBindingFlagsCreateInfo::builder()
            .binding_flags(&binding_flags)
            .build();
        if is_dynamic {
            info = info.push_next(&mut binding_flags_info);
        }
        let layout = unsafe { ctx.device.create_descriptor_set_layout(&info, None) }.unwrap();
        let set_layouts = [layout];
        let alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(pool)
            .set_layouts(&set_layouts)
            .build();

        let set = unsafe {
            ctx.device
                .allocate_descriptor_sets(&alloc_info)
                .unwrap_or_else(|_| panic!("faled allocating {}!", name))
        }
        .pop()
        .unwrap();
        // Every descriptor is initially unoccupied
        let occupancy = BitVec::repeat(false, capacity as usize);
        ctx.try_set_debug_name(&format!("{}_descriptor_set", name), set);
        ctx.try_set_debug_name(&format!("{}_descriptor_set_layout", name), layout);
        Self {
            name,
            layout,
            descriptor_type,
            occupancy,
            capacity,
            is_array,
            set,
        }
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            // Freed along the pool
            // device.free_descriptor_sets(pool, descriptor_sets)
            device.destroy_descriptor_set_layout(self.layout, None);
        }
    }

    pub fn count_active(&self) -> usize {
        self.occupancy.count_ones()
    }

    pub fn next_free(&self) -> usize {
        self.occupancy.first_zero().unwrap()
    }

    pub fn remove_at(&mut self, index: u32) {
        self.occupancy.set(index as usize, false);
    }

    pub fn place_sampler(&mut self, ctx: &VulkanContext, sampler: vk::Sampler) -> u32 {
        self.place_sampler_at(ctx, self.next_free() as u32, sampler)
    }

    pub fn place_sampler_at(
        &mut self,
        ctx: &VulkanContext,
        index: u32,
        sampler: vk::Sampler,
    ) -> u32 {
        self.queue_write(
            ctx,
            index,
            sampler,
            vk::ImageView::null(),
            vk::ImageLayout::UNDEFINED,
        )
    }

    pub fn place_image(
        &mut self,
        ctx: &VulkanContext,
        view: vk::ImageView,
        layout: vk::ImageLayout,
    ) -> u32 {
        self.place_image_at(ctx, self.next_free() as u32, view, layout)
    }

    pub fn place_image_at(
        &mut self,
        ctx: &VulkanContext,
        index: u32,
        view: vk::ImageView,
        layout: vk::ImageLayout,
    ) -> u32 {
        self.queue_write(ctx, index, vk::Sampler::null(), view, layout)
    }

    pub fn place_image_sampler(
        &mut self,
        ctx: &VulkanContext,
        view: vk::ImageView,
        layout: vk::ImageLayout,
        sampler: vk::Sampler,
    ) -> u32 {
        self.place_image_sampler_at(ctx, self.next_free() as u32, view, layout, sampler)
    }

    pub fn place_image_sampler_at(
        &mut self,
        ctx: &VulkanContext,
        index: u32,
        view: vk::ImageView,
        layout: vk::ImageLayout,
        sampler: vk::Sampler,
    ) -> u32 {
        self.queue_write(ctx, index, sampler, view, layout)
    }

    fn queue_write(
        &mut self,
        ctx: &VulkanContext,
        index: u32,
        sampler: vk::Sampler,
        image_view: vk::ImageView,
        image_layout: vk::ImageLayout,
    ) -> u32 {
        let info = vk::DescriptorImageInfo {
            image_view,
            image_layout,
            sampler,
        };
        let infos = [info];
        let (dst_binding, dst_array_element) = if self.is_array {
            (0, index)
        } else {
            (index, 0)
        };
        let write = vk::WriteDescriptorSet {
            descriptor_type: self.descriptor_type,
            descriptor_count: 1,
            dst_binding,
            dst_array_element,
            dst_set: self.set,
            p_image_info: infos.as_ptr(),
            ..Default::default()
        };
        let writes = [write];
        let copies = [];
        unsafe { ctx.device.update_descriptor_sets(&writes, &copies) };
        // Mark index slot as occupied and return
        self.occupancy.set(index as usize, true);
        index
    }
}
