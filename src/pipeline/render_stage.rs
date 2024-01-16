use std::collections::HashMap;

use crate::{
    buffer::{DeviceAllocator, DeviceSlice},
    pipeline::{attachment::Attachment, descriptor::DescriptorBuffer},
    render_task::{RenderTask, TaskKind},
    shader_resource::{ResourceKind, SingleResource},
    updater,
};
use ash::vk::{self, ShaderStageFlags};

use super::stage::Stage;

pub struct RenderStage {
    pub name: String,
    pub rendering: Rendering,
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub outputs: Vec<Attachment>,
    pub inputs: Vec<Attachment>,
    pub per_instance_updaters: Vec<ResourceKind>,
    pub per_pass_updaters: Vec<ResourceKind>,
    pub attachment_descriptors: Option<Box<DescriptorBuffer>>,
    pub task_kind: TaskKind,
    pub index: u32,
    pub is_final: bool,
    pub image_barriers: Vec<vk::ImageMemoryBarrier2>,
    pub reserved_buffers: Vec<DeviceSlice>,
    pub is_validation_layer_enabled: bool,
}

#[derive(Clone)]
pub struct Rendering {
    pub attachments: Vec<vk::RenderingAttachmentInfo>,
    pub depth_stencil: Option<vk::RenderingAttachmentInfo>,
    pub default_attachment_index: Option<usize>,
}

impl Stage for RenderStage {
    fn work(&mut self, ctx: super::RenderContext) {
        let mut image_barriers = self.image_barriers.clone();
        if self.is_final {
            image_barriers.push(Attachment::default_attachment_write_barrier(
                ctx.default_attachment.image,
            ));
        }
        if !image_barriers.is_empty() {
            let barrier_dep_info = vk::DependencyInfo::builder()
                .image_memory_barriers(&image_barriers)
                .build();
            unsafe {
                ctx.vulkan
                    .device
                    .cmd_pipeline_barrier2(ctx.command_buffer, &barrier_dep_info);
            }
        }
        let mut rendering_attachments = self.rendering.attachments.clone();
        if let Some(dai) = self.rendering.default_attachment_index {
            /*
             * If default attachment is present, override
             * the view with the current swapchain target
             */
            rendering_attachments[dai] = vk::RenderingAttachmentInfo {
                image_view: ctx.default_attachment.view,
                ..rendering_attachments[dai]
            };
        };
        /*
         * New rendering info because lifetimes for the
         * arrays inside are too complex to keep around
         */
        let mut rendering_info_builder = vk::RenderingInfo::builder()
            .color_attachments(&rendering_attachments)
            .render_area(if let Some(att) = self.outputs.first() {
                att.render_area_no_offset()
            } else {
                ctx.default_attachment.render_area_no_offset()
            })
            .layer_count(1);
        if let Some(att) = &self.rendering.depth_stencil {
            rendering_info_builder = rendering_info_builder.depth_attachment(att);
        }
        let rendering_info = rendering_info_builder.build();
        /*
         *  At this point we already waited for the previous stage invocation to finish,
         *  we can free the buffers used back then.
         */
        self.release_reserved_buffers(&ctx.buffer_allocator);
        let mut desc_buffer_info = vec![
            ctx.sampler_descriptors.binding_info(),
            ctx.image_descriptors.binding_info(),
        ];
        let mut desc_buffer_indices = vec![0, 1];
        let mut desc_buffer_offsets = vec![0, 0];
        if let Some(desc) = &self.attachment_descriptors {
            desc_buffer_info.push(desc.binding_info());
            desc_buffer_indices.push(2);
            desc_buffer_offsets.push(0);
        }
        unsafe {
            ctx.vulkan
                .extension
                .descriptor_buffer
                .cmd_bind_descriptor_buffers(ctx.command_buffer, &desc_buffer_info);
            ctx.vulkan
                .extension
                .descriptor_buffer
                .cmd_set_descriptor_buffer_offsets(
                    ctx.command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.layout,
                    0,
                    &desc_buffer_indices,
                    &desc_buffer_offsets,
                );
            ctx.vulkan
                .device
                .cmd_begin_rendering(ctx.command_buffer, &rendering_info);
            ctx.vulkan.device.cmd_bind_pipeline(
                ctx.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );
        }
        let tasks = &ctx.batches_by_task_type[self.task_kind.to_usize()];
        let per_pass_buffers = if tasks.is_empty() {
            // Nothing to draw, nothing to reserve
            Vec::new()
        } else {
            self.reserve_pass_buffers(&ctx.buffer_allocator, ctx.shader_resources_by_kind)
        };
        for task in tasks {
            let mesh_buffer = ctx.mesh_buffers_by_id.get(&task.mesh_buffer_id).unwrap();
            let is_indexed = !mesh_buffer.indices.is_empty();
            // Most of the time it's nowehere near going to be close to 32 addresses
            let mut push_constants: Vec<u64> = Vec::with_capacity(32);
            // First appearing, the per-pass data, uploaded once and repeated for all tasks
            push_constants.extend(&per_pass_buffers);
            // Second, the addresses pointing to the already uploaded vertex data
            if self.task_kind != TaskKind::Fullscreen {
                push_constants.extend(&[
                    mesh_buffer.vertices.device_addr,
                    mesh_buffer.normals.device_addr,
                    mesh_buffer.tex_coords.device_addr,
                ]);
            }
            // Third, the per-instance date for the task, uploaded per task
            push_constants.extend(&self.reserve_instance_buffers(&ctx.buffer_allocator, task));
            // Now we push the data into the command stream and issue the draws
            unsafe {
                if !push_constants.is_empty() {
                    let push_constants = push_constants.align_to::<u8>().1;
                    ctx.vulkan.device.cmd_push_constants(
                        ctx.command_buffer,
                        self.layout,
                        ShaderStageFlags::ALL_GRAPHICS,
                        0u32,
                        &push_constants,
                    );
                }
                if is_indexed {
                    ctx.vulkan.device.cmd_bind_index_buffer(
                        ctx.command_buffer,
                        mesh_buffer.indices.buffer,
                        mesh_buffer.indices.offset,
                        vk::IndexType::UINT32,
                    );
                    ctx.vulkan.device.cmd_draw_indexed(
                        ctx.command_buffer,
                        mesh_buffer.count,
                        task.instance_count,
                        0,
                        0,
                        0,
                    );
                } else {
                    ctx.vulkan.device.cmd_draw(
                        ctx.command_buffer,
                        mesh_buffer.count,
                        task.instance_count,
                        0,
                        0,
                    )
                }
            }
        }
        // End drawing this stage
        unsafe { ctx.vulkan.device.cmd_end_rendering(ctx.command_buffer) }
        if !self.is_final {
            // Nothing else to do
            return;
        }
        // Need to transition for presenting
        let present_image_barriers = vec![Attachment::default_attachment_present_barrier(
            ctx.default_attachment.image,
        )];
        let barrier_dep_info = vk::DependencyInfo::builder()
            .image_memory_barriers(&present_image_barriers)
            .build();
        unsafe {
            ctx.vulkan
                .device
                .cmd_pipeline_barrier2(ctx.command_buffer, &barrier_dep_info);
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn index(&self) -> u32 {
        self.index
    }

    fn is_validation_layer_enabled(&self) -> bool {
        self.is_validation_layer_enabled
    }

    fn image_barriers(&self) -> Vec<vk::ImageMemoryBarrier2> {
        self.image_barriers.clone()
    }

    fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.layout, None);
            if let Some(desc) = &self.attachment_descriptors {
                desc.destroy(device)
            }
        }
    }
}

impl RenderStage {
    fn release_reserved_buffers(&mut self, mem: &DeviceAllocator) {
        for buffer in self.reserved_buffers.drain(..) {
            mem.free(buffer);
        }
    }

    fn reserve_instance_buffers(&mut self, mem: &DeviceAllocator, task: &RenderTask) -> Vec<u64> {
        if self.per_instance_updaters.is_empty() {
            // Nothing to upload
            return Vec::new();
        }
        // We'll need the addresses to pass them to the shaders later
        let mut device_addrs = Vec::with_capacity(self.per_instance_updaters.len());
        for kind in self.per_instance_updaters.clone() {
            if let Some(res) = task.resources.get(&kind) {
                let buffer = updater::alloc_and_fill_multi(mem, res, task.instance_count);
                device_addrs.push(buffer.device_addr);
                self.reserved_buffers.push(buffer);
            } else {
                panic!("unavailable resource kind {}", kind)
            }
        }
        device_addrs
    }

    fn reserve_pass_buffers(
        &mut self,
        mem: &DeviceAllocator,
        shader_resources_by_kind: &HashMap<ResourceKind, SingleResource>,
    ) -> Vec<u64> {
        if self.per_pass_updaters.is_empty() {
            // Nothing to upload
            return Vec::new();
        }
        let total_size: usize = self
            .per_pass_updaters
            .iter()
            .map(|e| e.resource_size())
            .sum();
        let dst = mem.alloc(total_size as u64).unwrap();
        let mut offset = 0u64;
        for kind in self.per_pass_updaters.clone() {
            if let Some(res) = shader_resources_by_kind.get(&kind) {
                offset = updater::fill_single(res, &dst, offset);
            } else {
                panic!("unavailable resource kind {}", kind)
            }
        }
        // Will be freed later
        self.reserved_buffers.push(dst);
        // We'll need 1 address since all the data goes into the same buffer
        vec![dst.device_addr]
    }
}
