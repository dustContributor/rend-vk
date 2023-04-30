pub mod attachment;
pub mod file;
pub mod stage;
pub mod sampler;
mod load;
mod state;

#[derive(Clone)]
pub struct Pipeline {
    pub stages: Vec<crate::pipeline::stage::Stage>,
    pub attachments: Vec<crate::pipeline::attachment::Attachment>,
    pub nearest_sampler: crate::pipeline::sampler::Sampler,
    pub linear_sampler: crate::pipeline::sampler::Sampler,
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
