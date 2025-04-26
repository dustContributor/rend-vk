use ash::vk;

use crate::{context::VulkanContext, format::Format};

#[derive(Clone)]
pub struct Attachment {
    pub name: String,
    pub memory: vk::DeviceMemory,
    pub format: crate::format::Format,
    // Keep the equivalent vulkan value for convenience.
    pub vk_format: vk::Format,
    pub image: vk::Image,
    pub view: vk::ImageView,
    pub per_level_views: Vec<vk::ImageView>,
    pub level_usage: u8,
    pub extent: vk::Extent2D,
    pub descriptor_index: u32,
}

impl Attachment {
    pub const DEFAULT_NAME: &'static str = "default";

    pub fn levels(&self) -> u8 {
        1.max(self.per_level_views.len()) as u8
    }

    pub fn per_level_view(&self, level: u8) -> vk::ImageView {
        match self.per_level_views.get(level as usize) {
            Some(vw) => *vw,
            None => panic!("attachment {} doesn't has mip level {}", self.name, level),
        }
    }

    pub fn usage_view(&self) -> vk::ImageView {
        match crate::texture::MipMap::is_all_levels_value(self.level_usage) {
            true => self.view,                              // all levels view
            false => self.per_level_view(self.level_usage), // specific level view
        }
    }

    pub fn destroy(&self, device: &ash::Device) {
        if self.is_default() {
            // Default attachments are owned by the swapchain
            return;
        }
        unsafe {
            device.free_memory(self.memory, None);
            device.destroy_image_view(self.view, None);
            for view in &self.per_level_views {
                let view = *view;
                /*
                 * paranoid check, if the main vieattachmentw matches only one level,
                 * it may be present here too
                 */
                if self.view != view {
                    device.destroy_image_view(view, None);
                }
            }
            device.destroy_image(self.image, None);
        }
    }

    pub fn per_level_views_of(
        ctx: &VulkanContext,
        image: vk::Image,
        format: Format,
        levels: u8,
    ) -> Vec<vk::ImageView> {
        let vk_format = format.to_vk();
        let infos = (0..levels).map(|l| {
            vk::ImageViewCreateInfo::builder()
                .subresource_range(
                    vk::ImageSubresourceRange::builder()
                        .aspect_mask(format.aspect())
                        .base_mip_level(l as u32)
                        .level_count(1)
                        .layer_count(1)
                        .build(),
                )
                .image(image)
                .format(vk_format)
                .view_type(vk::ImageViewType::TYPE_2D)
                .build()
        });
        infos
            .map(|info| unsafe {
                ctx.device
                    .create_image_view(&info, None)
                    .expect("failed creating image view")
            })
            .collect()
    }

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
            per_level_views: [image_view].into(),
            level_usage: 0,
            extent,
            descriptor_index: 0,
        }
    }

    pub fn is_default(&self) -> bool {
        self.name == Attachment::DEFAULT_NAME
    }

    pub fn render_area_no_offset(&self) -> vk::Rect2D {
        vk::Rect2D {
            extent: self.extent,
            ..Default::default()
        }
    }

    pub fn default_attachment_write_barrier(image: vk::Image) -> vk::ImageMemoryBarrier2 {
        vk::ImageMemoryBarrier2::builder()
            .image(image)
            .src_access_mask(vk::AccessFlags2::MEMORY_READ)
            .dst_access_mask(vk::AccessFlags2::MEMORY_WRITE)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::ATTACHMENT_OPTIMAL)
            .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
            .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
            .subresource_range(Self::color_subresource_range())
            .build()
    }

    pub fn default_attachment_present_barrier(image: vk::Image) -> vk::ImageMemoryBarrier2 {
        vk::ImageMemoryBarrier2::builder()
            .image(image)
            .src_access_mask(vk::AccessFlags2::MEMORY_WRITE)
            // None is the expected access mask for presenting
            .dst_access_mask(vk::AccessFlags2::NONE)
            .old_layout(vk::ImageLayout::ATTACHMENT_OPTIMAL)
            .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
            .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
            .subresource_range(Self::color_subresource_range())
            .build()
    }

    pub fn default_attachment_rendering_attachment_info(
        a: &Attachment,
    ) -> vk::RenderingAttachmentInfo {
        vk::RenderingAttachmentInfo {
            image_view: a.view,
            image_layout: vk::ImageLayout::ATTACHMENT_OPTIMAL,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            ..Default::default()
        }
    }

    pub fn color_subresource_range() -> vk::ImageSubresourceRange {
        Self::default_subresource_range(vk::ImageAspectFlags::COLOR)
    }

    pub fn default_subresource_range(aspect: vk::ImageAspectFlags) -> vk::ImageSubresourceRange {
        Self::subresource_range_wlevels(aspect, 0, vk::REMAINING_MIP_LEVELS)
    }

    pub fn subresource_range_wlevels(
        aspect: vk::ImageAspectFlags,
        base: u32,
        count: u32,
    ) -> vk::ImageSubresourceRange {
        vk::ImageSubresourceRange::builder()
            .aspect_mask(aspect)
            .base_mip_level(base)
            .level_count(count)
            .base_array_layer(0)
            .layer_count(1)
            .build()
    }
}
