use ash::vk;

use super::{attachment::Attachment, stage::Stage};

pub struct BlitStage<'a> {
    pub name: String,
    pub input: Attachment,
    /// Wont be present if the blit stage writes the current swapchain image
    pub output: Option<Attachment>,
    pub filter: vk::Filter,
    pub region: vk::ImageBlit,
    pub index: u32,
    pub image_barriers: Vec<vk::ImageMemoryBarrier2<'a>>,
    pub is_validation_layer_enabled: bool,
    pub is_final: bool,
}

impl<'a> Stage for BlitStage<'a> {
    fn name(&self) -> &str {
        &self.name
    }

    fn index(&self) -> u32 {
        self.index
    }

    fn is_validation_layer_enabled(&self) -> bool {
        self.is_validation_layer_enabled
    }

    fn image_barriers(&'_ self) -> Vec<vk::ImageMemoryBarrier2<'_>> {
        self.image_barriers.clone()
    }

    fn destroy(&self, _device: &ash::Device) {
        // Nothing to do
    }

    fn work(&mut self, ctx: super::RenderContext) {
        let mut image_barriers = self.image_barriers.clone();
        if self.is_final {
            image_barriers.push(Attachment::default_attachment_blit_dest_barrier(
                ctx.default_attachment.image,
            ));
        }
        if !image_barriers.is_empty() {
            let barrier_dep_info =
                vk::DependencyInfo::default().image_memory_barriers(&image_barriers);
            unsafe {
                ctx.vulkan
                    .device
                    .cmd_pipeline_barrier2(ctx.command_buffer, &barrier_dep_info);
            }
        }
        unsafe {
            let regions = [self.region];
            ctx.vulkan.device.cmd_blit_image(
                ctx.command_buffer,
                self.input.image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                if self.is_final {
                    ctx.default_attachment.image
                } else {
                    self.output.as_ref().map(|e| e.image).unwrap()
                },
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &regions,
                self.filter,
            );
        }
        if self.is_final {
            // Need to transition for presenting
            let present_image_barriers = vec![Attachment::default_attachment_blit_present_barrier(
                ctx.default_attachment.image,
            )];
            let barrier_dep_info =
                vk::DependencyInfo::default().image_memory_barriers(&present_image_barriers);
            unsafe {
                ctx.vulkan
                    .device
                    .cmd_pipeline_barrier2(ctx.command_buffer, &barrier_dep_info);
            }
        }
    }
}
