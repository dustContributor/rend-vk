use ash::vk;
use bitvec::vec::BitVec;

use crate::context::VulkanContext;

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
