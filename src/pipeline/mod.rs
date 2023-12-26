use std::collections::HashMap;

use self::descriptor::DescriptorBuffer;
use self::sampler::SamplerKey;

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

// Fixed descriptor set indices
pub const DESCRIPTOR_SET_SAMPLER: u32 = 0;
pub const DESCRIPTOR_SET_TEXTURE: u32 = 1;
pub const DESCRIPTOR_SET_TARGET_IMAGE: u32 = 2;

pub struct Pipeline {
    pub stages: Vec<Stage>,
    pub attachments: Vec<Attachment>,
    pub image_descriptors: DescriptorBuffer,
    pub sampler_descriptors: DescriptorBuffer,
    pub samplers_by_key: HashMap<SamplerKey, Sampler>,
}

pub fn signal_value_for(current_frame: u64, total_stages: u32, stage_index: u32) -> u64 {
    current_frame * total_stages as u64 + stage_index as u64
}

impl Pipeline {
    pub fn total_stages(&self) -> u32 {
        self.stages.len() as u32
    }

    pub fn signal_value_for(&self, current_frame: u64, stage_index: u32) -> u64 {
        signal_value_for(current_frame, self.total_stages(), stage_index)
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            for e in [&self.image_descriptors, &self.sampler_descriptors] {
                e.destroy(device);
            }
            for e in self.samplers_by_key.values() {
                e.destroy(device);
            }
            for stage in &self.stages {
                device.destroy_pipeline(stage.pipeline, None);
                device.destroy_pipeline_layout(stage.layout, None);
                if let Some(desc) = &stage.attachment_descriptors {
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
