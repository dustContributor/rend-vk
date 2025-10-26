use ash::vk;

use super::{attachment::Attachment, stage::Stage};

pub struct BlitStage<'a> {
    pub name: String,
    pub output: Attachment,
    pub input: Attachment,
    pub filter: vk::Filter,
    pub region: vk::ImageBlit,
    pub index: u32,
    pub image_barriers: Vec<vk::ImageMemoryBarrier2<'a>>,
    pub is_validation_layer_enabled: bool,
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
        if !self.image_barriers.is_empty() {
            let barrier_dep_info =
                vk::DependencyInfo::default().image_memory_barriers(&self.image_barriers);
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
                self.output.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &regions,
                self.filter,
            );
        }
    }
}
