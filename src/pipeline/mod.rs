use self::descriptor::DescriptorBuffer;

use crate::pipeline::attachment::Attachment;
use crate::pipeline::sampler::Sampler;
use crate::pipeline::stage::Stage;

pub mod attachment;
pub mod descriptor;
pub mod file;
mod load;
pub mod sampler;
pub mod stage;
mod state;

pub struct Pipeline {
    pub stages: Vec<Stage>,
    pub attachments: Vec<Attachment>,
    pub nearest_sampler: Sampler,
    pub linear_sampler: Sampler,
    pub image_descriptors: DescriptorBuffer,
    pub sampler_descriptors: DescriptorBuffer,
}

impl Pipeline {
    pub fn total_stages(&self) -> u32 {
        self.stages.len() as u32
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            for e in [&self.image_descriptors, &self.sampler_descriptors] {
                e.destroy(device);
            }
            for e in [&self.linear_sampler, &self.nearest_sampler] {
                e.destroy(device);
            }
            for stage in &self.stages {
                device.destroy_pipeline(stage.pipeline, None);
                device.destroy_pipeline_layout(stage.layout, None);
                if let Some(desc) = &stage.input_descriptors {
                    desc.destroy(device)
                }
                if let Some(desc) = &stage.ubo_descriptors {
                    desc.destroy(device)
                }
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
