use std::collections::HashMap;

use ash::vk;

use self::descriptor::DescriptorGroup;
use self::sampler::SamplerKey;

use crate::buffer::DeviceAllocator;

use crate::pipeline::attachment::Attachment;
use crate::pipeline::sampler::Sampler;
use crate::render_task::RenderTask;
use crate::renderer::MeshBuffer;
use crate::shader_resource::{ResourceKind, SingleResource};

pub mod attachment;
mod barrier_gen;
pub mod blit_stage;
pub mod descriptor;
pub mod file;
mod load;
pub mod render_stage;
pub mod sampler;
pub mod stage;
mod state;

// Fixed descriptor set indices
pub const DESCRIPTOR_SET_SAMPLER: u32 = 0;
pub const DESCRIPTOR_SET_TEXTURE: u32 = 1;
pub const DESCRIPTOR_SET_TARGET_IMAGE: u32 = 2;

pub struct Pipeline {
    pub stages: Vec<Box<dyn stage::Stage>>,
    pub attachments: Vec<Attachment>,
    pub descriptor_pool: vk::DescriptorPool,
    pub image_descriptors: DescriptorGroup,
    pub sampler_descriptors: DescriptorGroup,
    pub samplers_by_key: HashMap<SamplerKey, Sampler>,
}

pub fn signal_value_for(current_frame: u64, total_stages: u32, stage_index: u32) -> u64 {
    current_frame * total_stages as u64 + stage_index as u64
}

#[derive(Clone)]
pub struct RenderContext<'a> {
    pub vulkan: &'a crate::context::VulkanContext,
    pub batches_by_task_type: &'a Vec<Vec<RenderTask>>,
    pub mesh_buffers_by_id: &'a HashMap<u32, MeshBuffer>,
    pub shader_resources_by_kind: &'a HashMap<ResourceKind, SingleResource>,
    pub sampler_descriptors: &'a DescriptorGroup,
    pub image_descriptors: &'a DescriptorGroup,
    pub buffer_allocator: &'a DeviceAllocator,
    pub command_buffer: vk::CommandBuffer,
    pub default_attachment: &'a Attachment,
}

impl Pipeline {
    pub fn process_stages(
        &mut self,
        pass_semaphore: vk::Semaphore,
        render_queue: vk::Queue,
        current_frame: u64,
        render_context: RenderContext,
    ) {
        let total_stages = self.stages.len() as u32;
        for stage in self.stages.iter_mut() {
            render_context.vulkan.try_begin_debug_label(render_context.command_buffer, stage.name());
            stage.wait_for_previous_frame(
                &render_context.vulkan.device,
                current_frame,
                total_stages,
                pass_semaphore,
            );
            stage.work(render_context.clone());
            stage.signal_next_frame(
                &render_context.vulkan.device,
                current_frame,
                total_stages,
                pass_semaphore,
                render_queue,
            );
            render_context.vulkan.try_end_debug_label(render_context.command_buffer);
        }
    }

    pub fn total_stages(&self) -> u32 {
        self.stages.len() as u32
    }

    pub fn signal_value_for(&self, current_frame: u64, stage_index: u32) -> u64 {
        signal_value_for(current_frame, self.total_stages(), stage_index)
    }

    pub fn gen_initial_barriers(&self) -> Vec<vk::ImageMemoryBarrier2> {
        let mut first_layout_by_image: HashMap<
            vk::Image,
            (vk::ImageLayout, vk::ImageSubresourceRange),
        > = HashMap::new();

        self.stages
            .iter()
            .flat_map(|e| e.image_barriers())
            .for_each(|barrier| {
                if first_layout_by_image.contains_key(&barrier.image) {
                    // Already registered the first occurence of this image
                    return;
                }
                first_layout_by_image.insert(
                    barrier.image,
                    (barrier.old_layout, barrier.subresource_range),
                );
            });

        let initial_barriers: Vec<_> = first_layout_by_image
            .into_iter()
            .map(|e| {
                let image = e.0;
                let layout = e.1 .0;
                let subr = e.1 .1;
                vk::ImageMemoryBarrier2::builder()
                    .image(image)
                    .src_access_mask(vk::AccessFlags2::NONE)
                    .dst_access_mask(vk::AccessFlags2::NONE)
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(layout)
                    .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                    .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                    .subresource_range(subr)
                    .build()
            })
            .collect();

        return initial_barriers;
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            for e in [&self.image_descriptors, &self.sampler_descriptors] {
                e.destroy(device);
            }
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            for e in self.samplers_by_key.values() {
                e.destroy(device);
            }
            for stage in &self.stages {
                stage.destroy(device);
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
