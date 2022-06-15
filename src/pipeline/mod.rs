use ash::vk;

use crate::render::BatchType;

pub mod file;
mod load;
mod state;

#[derive(Clone)]
pub struct Attachment {
    name: String,
    memory: vk::DeviceMemory,
    format: crate::format::Format,
    // Keep the equivalent vulkan value for convenience.
    vk_format: vk::Format,
    image: vk::Image,
    view: vk::ImageView,
}

impl Attachment {
    const DEFAULT_NAME: &'static str = "default";
    const DEPTH_NAME: &'static str = "depth";
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

#[derive(Clone)]
pub struct Pipeline {
    pub stages: Vec<Stage>,
    pub attachments: Vec<Attachment>,
}

impl Pipeline {
    pub fn destroy(&self, device: ash::Device) {
        unsafe {
            for stage in &self.stages {
                device.destroy_pipeline(stage.pipeline, None);
                device.destroy_pipeline_layout(stage.layout, None);
            }
            for attachment in &self.attachments {
                device.free_memory(attachment.memory, None);
                device.destroy_image_view(attachment.view, None);
                device.destroy_image(attachment.image, None);
            }
        }
    }
}
