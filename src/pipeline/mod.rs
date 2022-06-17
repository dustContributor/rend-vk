use ash::vk;

use crate::render::BatchType;

pub mod file;
mod load;
mod state;

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
}

#[derive(Clone)]
pub struct Stage {
    pub name: String,
    pub pre_rendering: vk::RenderingInfo,
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub outputs: Vec<Attachment>,
    pub inputs: Vec<Attachment>,
    pub updaters: Vec<String>,
    pub batch: BatchType,
    pub is_final: bool,
}

impl Stage {
    fn mask_layout_aspect_for(
        format: crate::format::Format,
    ) -> (vk::AccessFlags, vk::ImageLayout, vk::ImageAspectFlags) {
        if format.has_depth() {
            let (layout, aspect) = if format.has_stencil() {
                (
                    vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                    vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL,
                )
            } else {
                (
                    vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL,
                    vk::ImageAspectFlags::DEPTH,
                )
            };
            (
                vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
                layout,
                aspect,
            )
        } else {
            (
                vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                vk::ImageAspectFlags::COLOR,
            )
        }
    }

    pub fn render<F: FnOnce(&ash::Device, vk::CommandBuffer)>(
        &self,
        device: &ash::Device,
        command_buffer: vk::CommandBuffer,
        default_attachment: &Attachment,
        draw_commands: F,
    ) {
        let default_output = vec![default_attachment.clone()];
        let outputs = if self.is_final {
            &default_output
        } else {
            &self.outputs
        };
        let pre_transition_barriers: Vec<_> = outputs
            .iter()
            .map(|e| {
                let (msk, layout, aspect) = Self::mask_layout_aspect_for(e.format);
                vk::ImageMemoryBarrier::builder()
                    .image(e.image)
                    .dst_access_mask(msk)
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(layout)
                    .subresource_range(
                        vk::ImageSubresourceRange::builder()
                            .aspect_mask(aspect)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(1)
                            .build(),
                    )
                    .build()
            })
            .collect();
        unsafe {
            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &pre_transition_barriers,
            );
            let pre_rendering = if self.is_final {
                vk::RenderingInfo::builder()
                    .color_attachments(&[vk::RenderingAttachmentInfo {
                        image_view: default_attachment.view.clone(),
                        image_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                        load_op: vk::AttachmentLoadOp::CLEAR,
                        store_op: vk::AttachmentStoreOp::STORE,
                        ..Default::default()
                    }])
                    .render_area(vk::Rect2D {
                        extent: default_attachment.extent,
                        ..Default::default()
                    })
                    .layer_count(1)
                    .build()
            } else {
                self.pre_rendering
            };
            device.cmd_begin_rendering(command_buffer, &pre_rendering);
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );
            draw_commands(&device, command_buffer);
            device.cmd_end_rendering(command_buffer);
        };
        let is_final = self.is_final;
        let post_transition_barriers: Vec<_> = outputs
            .iter()
            .map(|e| {
                vk::ImageMemoryBarrier::builder()
                    .image(e.image)
                    .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                    .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                    .subresource_range(
                        vk::ImageSubresourceRange::builder()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(1)
                            .build(),
                    )
                    .build()
            })
            .collect();
        unsafe {
            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &post_transition_barriers,
            );
        }
    }
}

#[derive(Clone)]
pub struct Pipeline {
    pub stages: Vec<Stage>,
    pub attachments: Vec<Attachment>,
}

impl Pipeline {
    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            for stage in &self.stages {
                device.destroy_pipeline(stage.pipeline, None);
                device.destroy_pipeline_layout(stage.layout, None);
            }
            for attachment in &self.attachments {
                if attachment.is_default() {
                    // Default attachments are owned by the swapchain
                    continue;
                }
                device.free_memory(attachment.memory, None);
                device.destroy_image_view(attachment.view, None);
                device.destroy_image(attachment.image, None);
            }
        }
    }
}
