use ash::vk::{self, Extent2D};

use crate::context::VulkanContext;

#[derive(Clone)]
pub struct Texture {
    pub name: String,
    pub id: u32,
    pub memory: vk::DeviceMemory,
    pub format: crate::format::Format,
    // Keep the equivalent vulkan value for convenience.
    pub vk_format: vk::Format,
    pub image: vk::Image,
    pub view: vk::ImageView,
    pub extent: vk::Extent2D,
}

pub fn make(
    ctx: &VulkanContext,
    id: u32,
    name: String,
    extent: Extent2D,
    mip_levels: u32,
    format: crate::format::Format,
    is_attachment: bool,
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
        memory,
        format,
        vk_format,
        image,
        view,
        extent,
    }
}
