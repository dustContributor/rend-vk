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
    pub pre_rendering: vk::RenderingInfo,
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub outputs: Vec<Attachment>,
    pub inputs: Vec<Attachment>,
    pub updaters: Vec<String>,
    pub batch: BatchType,
    pub is_final: bool,
    pub image_barriers: Vec<vk::ImageMemoryBarrier2>,
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
        let default_image_barriers = vec![Attachment::default_attachment_write_barrier(
            default_attachment.image,
        )];
        let default_rendering_attachment_infos =
            vec![Attachment::default_attachment_rendering_attachment_info(
                default_attachment,
            )];
        let barrier_dep_info = vk::DependencyInfo::builder()
            .image_memory_barriers(if self.is_final {
                &default_image_barriers
            } else {
                &self.image_barriers
            })
            .build();

        let pre_rendering = if self.is_final {
            vk::RenderingInfo::builder()
                .color_attachments(&default_rendering_attachment_infos)
                .render_area(default_attachment.render_area_no_offset())
                .layer_count(1)
                .build()
        } else {
            self.pre_rendering
        };
        unsafe {
            device.cmd_pipeline_barrier2(command_buffer, &barrier_dep_info);
            device.cmd_begin_rendering(command_buffer, &pre_rendering);
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
