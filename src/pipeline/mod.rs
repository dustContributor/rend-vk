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

#[derive(Clone)]
pub struct RenderContext<'a> {
    pub vulkan: &'a crate::context::VulkanContext,
    pub batches_by_task_type: &'a HashMap<u64, Vec<RenderTask>>,
    pub mesh_buffers_by_id: &'a HashMap<u32, MeshBuffer>,
    pub shader_resources_by_kind: &'a HashMap<ResourceKind, SingleResource>,
    pub sampler_descriptors: &'a DescriptorGroup,
    pub image_descriptors: &'a DescriptorGroup,
    pub buffer_allocator: &'a DeviceAllocator,
    pub command_buffer: vk::CommandBuffer,
    pub default_attachment: &'a Attachment,
}

impl Pipeline {
    pub fn process_stages(&mut self, render_context: RenderContext) {
        for stage in self.stages.iter_mut() {
            render_context
                .vulkan
                .try_begin_debug_label(render_context.command_buffer, stage.name());
            stage.work(render_context.clone());
            render_context
                .vulkan
                .try_end_debug_label(render_context.command_buffer);
        }
    }

    pub fn total_stages(&self) -> u32 {
        self.stages.len() as u32
    }

    pub fn gen_initial_barriers(&'_ self) -> Vec<vk::ImageMemoryBarrier2<'_>> {
        let mut first_layouts: Vec<_> = Vec::new();
        // collect all barriers in the pipeline since we're going to check them all
        let barriers: Vec<_> = self
            .stages
            .iter()
            .flat_map(|e| e.image_barriers())
            .collect();
        // for every mip level of every attachment, find the first layout the render pipeline needs it to be in
        for att in &self.attachments {
            for lvl in 0..att.levels() as u32 {
                for barrier in &barriers {
                    if barrier.image != att.image {
                        // barrier doesn't corresponds to this image
                        continue;
                    }
                    let sub_range = barrier.subresource_range;
                    if lvl < sub_range.base_mip_level
                        || lvl >= (sub_range.base_mip_level + sub_range.level_count)
                    {
                        // barrier doesn't corresponds to level
                        continue;
                    }
                    // found the first layout for this specific level
                    first_layouts.push((att.image, barrier.old_layout, lvl, sub_range.aspect_mask));
                    break;
                }
            }
        }
        // generate all of the initial barriers transitioning into the first expected layout
        let initial_barriers: Vec<_> = first_layouts
            .into_iter()
            .map(|(image, initial_layout, base_mip_level, aspect_mask)| {
                vk::ImageMemoryBarrier2::default()
                    .image(image)
                    .src_access_mask(vk::AccessFlags2::NONE)
                    .dst_access_mask(vk::AccessFlags2::NONE)
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(initial_layout)
                    .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                    .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                    // every mip level is transitioned individually, occurs only once so no problem
                    .subresource_range(vk::ImageSubresourceRange {
                        base_mip_level,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                        aspect_mask,
                    })
            })
            .collect();
        // return initial barriers
        initial_barriers
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
                attachment.destroy(device);
            }
        }
    }
}
