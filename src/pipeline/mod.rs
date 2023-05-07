use self::descriptor::DescriptorBuffer;
use crate::buffer::DeviceAllocator;
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

pub struct VulkanContext {
    pub instance: ash::Instance,
    pub device: ash::Device,
    pub physical_device: ash::vk::PhysicalDevice,
    pub desc_buffer_instance: ash::extensions::ext::DescriptorBuffer,
}

// #[derive(Clone)]
pub struct Pipeline {
    pub stages: Vec<Stage>,
    pub attachments: Vec<Attachment>,
    pub nearest_sampler: Sampler,
    pub linear_sampler: Sampler,
    pub ubo_descriptors: DescriptorBuffer,
    pub image_descriptors: DescriptorBuffer,
    pub buffer_allocator: DeviceAllocator,
    pub descriptor_allocator: DeviceAllocator,
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
