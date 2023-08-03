use ash::vk::{self, Extent2D};

use crate::{buffer::DeviceSlice, context::VulkanContext};

#[derive(Clone)]
pub struct Texture {
    pub id: u32,
    pub format: crate::format::Format,
    pub mip_levels: u32,
    pub name: String,
    pub memory: vk::DeviceMemory,
    pub image: vk::Image,
    pub view: vk::ImageView,
    pub extent: vk::Extent2D,
    pub staging: Option<Box<DeviceSlice>>,
}

impl Texture {
    fn subresource_range(&self) -> vk::ImageSubresourceRange {
        vk::ImageSubresourceRange {
            aspect_mask: self.format.aspect(),
            level_count: self.mip_levels,
            layer_count: 1,
            ..Default::default()
        }
    }

    pub fn transition_to_optimal(&self, ctx: &VulkanContext, cmd_buffer: vk::CommandBuffer) {
        let barrier_initial = vk::ImageMemoryBarrier {
            image: self.image,
            dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            subresource_range: self.subresource_range(),
            ..Default::default()
        };
        unsafe {
            ctx.device.cmd_pipeline_barrier(
                cmd_buffer,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier_initial],
            )
        };
        // TODO: Research how to copy mipmaps. One by one? All in one go?
        let buffer_copy_regions = vk::BufferImageCopy::builder()
            .image_subresource(
                vk::ImageSubresourceLayers::builder()
                    .aspect_mask(self.format.aspect())
                    .layer_count(1)
                    .build(),
            )
            .image_extent(self.extent.into())
            .build();
        let image_buffer = self.staging.as_ref().unwrap().buffer;
        unsafe {
            ctx.device.cmd_copy_buffer_to_image(
                cmd_buffer,
                image_buffer,
                self.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[buffer_copy_regions],
            )
        };
        let barrier_end = vk::ImageMemoryBarrier {
            src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            dst_access_mask: vk::AccessFlags::SHADER_READ,
            old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            image: self.image,
            subresource_range: self.subresource_range(),
            ..Default::default()
        };
        unsafe {
            ctx.device.cmd_pipeline_barrier(
                cmd_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier_end],
            )
        };
    }
}

pub fn make(
    ctx: &VulkanContext,
    id: u32,
    name: String,
    extent: Extent2D,
    mip_levels: u32,
    format: crate::format::Format,
    is_attachment: bool,
    staging: Option<Box<DeviceSlice>>,
) -> Texture {
    let vk_format = format.to_vk();
    let create_info = vk::ImageCreateInfo {
        image_type: vk::ImageType::TYPE_2D,
        format: vk_format,
        extent: extent.into(),
        mip_levels,
        array_layers: 1,
        samples: vk::SampleCountFlags::TYPE_1,
        tiling: vk::ImageTiling::OPTIMAL,
        usage: if is_attachment {
            (if format.has_depth() {
                vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT
            } else {
                vk::ImageUsageFlags::COLOR_ATTACHMENT
            }) | vk::ImageUsageFlags::SAMPLED
        } else {
            vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED
        },
        sharing_mode: vk::SharingMode::EXCLUSIVE,
        ..Default::default()
    };
    let image = unsafe { ctx.device.create_image(&create_info, None) }.unwrap();
    let mut dedicated_req = vk::MemoryDedicatedRequirements {
        ..Default::default()
    };
    let mut memory_req = vk::MemoryRequirements2::builder()
        .push_next(&mut dedicated_req)
        .build();

    let requirements_info = vk::ImageMemoryRequirementsInfo2::builder()
        .image(image)
        .build();
    unsafe {
        ctx.device
            .get_image_memory_requirements2(&requirements_info, &mut memory_req)
    };

    let mut dedicated_info = vk::MemoryDedicatedAllocateInfo::builder()
        .image(image)
        .build();

    let memory_allocate_info = vk::MemoryAllocateInfo::builder()
        .push_next(&mut dedicated_info)
        .allocation_size(memory_req.memory_requirements.size)
        .memory_type_index(
            ctx.memory_type_index_for(
                memory_req.memory_requirements.memory_type_bits,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )
            .unwrap(),
        )
        .build();

    let memory = unsafe {
        ctx.device
            .allocate_memory(&memory_allocate_info, None)
            .expect("failed image memory alloc")
    };

    unsafe {
        ctx.device
            .bind_image_memory(image, memory, 0)
            .expect("failed image memory bind")
    };

    let image_view_info = vk::ImageViewCreateInfo::builder()
        .subresource_range(
            vk::ImageSubresourceRange::builder()
                .aspect_mask(format.aspect())
                .level_count(mip_levels)
                .layer_count(1)
                .build(),
        )
        .image(image)
        .format(vk_format)
        .view_type(vk::ImageViewType::TYPE_2D);

    ctx.try_set_debug_name(&name, image);

    let view = unsafe {
        ctx.device
            .create_image_view(&image_view_info, None)
            .expect("failed image view")
    };
    Texture {
        name,
        id,
        mip_levels,
        memory,
        format,
        image,
        view,
        extent,
        staging,
    }
}
