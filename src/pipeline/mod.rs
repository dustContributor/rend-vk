use ash::vk;

use crate::render::BatchType;

pub mod file;
mod load;
mod state;

#[derive(Clone)]
pub struct Attachment {
    name: String,
    memory: vk::DeviceMemory,
    format: vk::Format,
    image: vk::Image,
    view: vk::ImageView,
    clear: Option<vk::ClearValue>
}

#[derive(Clone)]
pub struct Stage {
    pub name: String,
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub outputs: Vec<Attachment>,
    pub inputs: Vec<Attachment>,
    pub updaters: Vec<String>,
    pub batch: BatchType,
}

#[derive(Clone)]
pub struct Pipeline {
    pub stages: Vec<Stage>,
    pub attachments: Vec<Attachment>,
    pub shader_modules: Vec<vk::ShaderModule>,
}

impl Pipeline {
    pub fn destroy(&self, device: ash::Device) {
        unsafe {
            for stage in &self.stages {
                device.destroy_pipeline(stage.pipeline, None);
                device.destroy_pipeline_layout(stage.layout, None);
            }
            for module in &self.shader_modules {
                device.destroy_shader_module(*module, None);
            }
            for attachment in &self.attachments {
                device.free_memory(attachment.memory, None);
                device.destroy_image_view(attachment.view, None);
                device.destroy_image(attachment.image, None);
            }
        }
    }
}
