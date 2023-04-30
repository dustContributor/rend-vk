use ash::vk;

#[derive(Clone)]
pub struct Attachment {
    pub name: String,
    pub memory: vk::DeviceMemory,
    pub format: crate::format::Format,
    // Keep the equivalent vulkan value for convenience.
    pub vk_format: vk::Format,
    pub image: vk::Image,
    pub view: vk::ImageView,
    pub extent: vk::Extent2D,
}

impl Attachment {
    pub const DEFAULT_NAME: &'static str = "default";
    pub const DEPTH_NAME: &'static str = "depth";

    pub fn default_attachment_of(
        vk_format: vk::Format,
        image: vk::Image,
        image_view: vk::ImageView,
        extent: vk::Extent2D,
    ) -> Attachment {
        Attachment {
            format: crate::format::Format::UNDEFINED,
            vk_format,
            image,
            memory: vk::DeviceMemory::null(),
            name: Attachment::DEFAULT_NAME.to_string(),
            view: image_view,
            extent,
        }
    }

    pub fn is_default(&self) -> bool {
        self.name == Attachment::DEFAULT_NAME
    }

    pub fn default_attachment_write_barrier(image: vk::Image) -> vk::ImageMemoryBarrier2 {
        vk::ImageMemoryBarrier2::builder()
            .image(image)
            .src_access_mask(vk::AccessFlags2::MEMORY_READ)
            .dst_access_mask(vk::AccessFlags2::MEMORY_WRITE)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::ATTACHMENT_OPTIMAL)
            .src_stage_mask(vk::PipelineStageFlags2::NONE)
            .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
            .subresource_range(Self::color_subresource_range())
            .build()
    }

    pub fn default_attachment_present_barrier(image: vk::Image) -> vk::ImageMemoryBarrier2 {
        vk::ImageMemoryBarrier2::builder()
            .image(image)
            .src_access_mask(vk::AccessFlags2::MEMORY_WRITE)
            .dst_access_mask(vk::AccessFlags2::NONE)
            .old_layout(vk::ImageLayout::ATTACHMENT_OPTIMAL)
            .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .src_stage_mask(vk::PipelineStageFlags2::NONE)
            .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
            .subresource_range(Self::color_subresource_range())
            .build()
    }

    pub fn color_subresource_range() -> vk::ImageSubresourceRange {
        vk::ImageSubresourceRange::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1)
            .build()
    }
}
