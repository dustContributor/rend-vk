use crate::pipeline::attachment::Attachment;
use ash::vk;

#[derive(serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(Copy, Clone)]
pub enum BatchType {
    Opaque,
    Fullscreen,
    PointLight,
}

#[derive(Clone)]
pub struct Stage {
    pub name: String,
    pub rendering: Rendering,
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub outputs: Vec<Attachment>,
    pub inputs: Vec<Attachment>,
    pub updaters: Vec<String>,
    pub batch: BatchType,
    pub is_final: bool,
    pub image_barriers: Vec<vk::ImageMemoryBarrier2>,
}

#[derive(Clone)]
pub struct Rendering {
    pub attachments: Vec<vk::RenderingAttachmentInfo>,
    pub depth_stencil: Option<vk::RenderingAttachmentInfo>,
    pub default_attachment_index: Option<usize>,
}

impl Stage {
    pub fn render<F: FnOnce(&ash::Device, vk::CommandBuffer)>(
        &self,
        device: &ash::Device,
        command_buffer: vk::CommandBuffer,
        default_attachment: &Attachment,
        draw_commands: F,
    ) {
        let default_image_barriers = vec![Attachment::default_attachment_write_barrier(
            default_attachment.image,
        )];
        let barrier_dep_info = vk::DependencyInfo::builder()
            .image_memory_barriers(if self.is_final {
                &default_image_barriers
            } else {
                &self.image_barriers
            })
            .build();
        let mut rendering_attachments = self.rendering.attachments.clone();
        if let Some(dai) = self.rendering.default_attachment_index {
            /*
             * If default attachment is present, override
             * the view with the current swapchain target
             */
            rendering_attachments[dai] = vk::RenderingAttachmentInfo {
                image_view: default_attachment.view,
                ..rendering_attachments[dai]
            };
        };
        /*
         * New rendering info because lifetimes for the
         * arrays inside are too complex to keep around
         */
        let rendering_info_builder = vk::RenderingInfo::builder()
            .color_attachments(&rendering_attachments)
            .render_area(if let Some(att) = self.outputs.first() {
                att.render_area_no_offset()
            } else {
                default_attachment.render_area_no_offset()
            })
            .layer_count(1);
        let rendering_info = if let Some(att) = self.rendering.depth_stencil {
            rendering_info_builder.depth_attachment(&att).build()
        } else {
            rendering_info_builder.build()
        };
        unsafe {
            device.cmd_pipeline_barrier2(command_buffer, &barrier_dep_info);
            device.cmd_begin_rendering(command_buffer, &rendering_info);
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );
            draw_commands(&device, command_buffer);
            device.cmd_end_rendering(command_buffer);
        }
        if !self.is_final {
            // Nothing else to do
            return;
        }
        // Need to transition for presenting
        let present_image_barriers = vec![Attachment::default_attachment_present_barrier(
            default_attachment.image,
        )];
        let barrier_dep_info = vk::DependencyInfo::builder()
            .image_memory_barriers(&present_image_barriers)
            .build();
        unsafe {
            device.cmd_pipeline_barrier2(command_buffer, &barrier_dep_info);
        }
    }
}
