use std::collections::HashMap;

use crate::{
    buffer::{DeviceAllocator, DeviceSlice},
    pipeline::{attachment::Attachment, descriptor::DescriptorBuffer},
    render_task::{RenderTask, TaskKind},
    renderer::MeshBuffer,
    shader_resource::{ResourceKind, SingleResource},
    updater,
};
use ash::vk::{self, ShaderStageFlags};

pub struct Stage {
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

impl Stage {
    pub fn render(
        &mut self,
        ctx: &crate::context::VulkanContext,
        batches_by_task_type: &Vec<Vec<RenderTask>>,
        mesh_buffers_by_id: &HashMap<u32, MeshBuffer>,
        shader_resources_by_kind: &HashMap<ResourceKind, SingleResource>,
        sampler_descriptors: &DescriptorBuffer,
        image_descriptors: &DescriptorBuffer,
        buffer_allocator: &DeviceAllocator,
        command_buffer: vk::CommandBuffer,
        default_attachment: &Attachment,
    ) {
        let mut image_barriers = self.image_barriers.clone();
        if self.is_final {
            image_barriers.push(Attachment::default_attachment_write_barrier(
                default_attachment.image,
            ));
        }
        if !image_barriers.is_empty() {
            let barrier_dep_info = vk::DependencyInfo::builder()
                .image_memory_barriers(&image_barriers)
                .build();
            unsafe {
                ctx.device
                    .cmd_pipeline_barrier2(command_buffer, &barrier_dep_info);
            }
        }
        let mut rendering_attachments = self.rendering.attachments.clone();
        if let Some(dai) = self.rendering.default_attachment_index {
            /*
             * If default attachment is present, override
             * the view with the current swapchain target
             */
            rendering_attachments[dai] = vk::RenderingAttachmentInfo {
                image_view: default_attachment.view,
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
                default_attachment.render_area_no_offset()
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
        self.release_reserved_buffers(&buffer_allocator);
        let mut desc_buffer_info = vec![
            sampler_descriptors.binding_info(),
            image_descriptors.binding_info(),
        ];
        let mut desc_buffer_indices = vec![0, 1];
        let mut desc_buffer_offsets = vec![0, 0];
        if let Some(desc) = &self.attachment_descriptors {
            desc_buffer_info.push(desc.binding_info());
            desc_buffer_indices.push(2);
            desc_buffer_offsets.push(0);
        }
        unsafe {
            ctx.extension
                .descriptor_buffer
                .cmd_bind_descriptor_buffers(command_buffer, &desc_buffer_info);
            ctx.extension
                .descriptor_buffer
                .cmd_set_descriptor_buffer_offsets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.layout,
                    0,
                    &desc_buffer_indices,
                    &desc_buffer_offsets,
                );
            ctx.device
                .cmd_begin_rendering(command_buffer, &rendering_info);
            ctx.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );
        }
        let per_pass_buffers =
            self.reserve_pass_buffers(&buffer_allocator, shader_resources_by_kind);
        let tasks = &batches_by_task_type[self.task_kind.to_usize()];
        for task in tasks {
            let mesh_buffer = mesh_buffers_by_id.get(&task.mesh_buffer_id).unwrap();
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
            push_constants.extend(&self.reserve_instance_buffers(&buffer_allocator, task));
            // Now we push the data into the command stream and issue the draws
            unsafe {
                if !push_constants.is_empty() {
                    let push_constants = push_constants.align_to::<u8>().1;
                    ctx.device.cmd_push_constants(
                        command_buffer,
                        self.layout,
                        ShaderStageFlags::ALL_GRAPHICS,
                        0u32,
                        &push_constants,
                    );
                }
                if is_indexed {
                    ctx.device.cmd_bind_index_buffer(
                        command_buffer,
                        mesh_buffer.indices.buffer,
                        mesh_buffer.indices.offset,
                        vk::IndexType::UINT32,
                    );
                    ctx.device.cmd_draw_indexed(
                        command_buffer,
                        mesh_buffer.count,
                        task.instance_count,
                        0,
                        0,
                        0,
                    );
                } else {
                    ctx.device.cmd_draw(
                        command_buffer,
                        mesh_buffer.count,
                        task.instance_count,
                        0,
                        0,
                    )
                }
            }
        }
        // End drawing this stage
        unsafe { ctx.device.cmd_end_rendering(command_buffer) }
        if !self.is_final {
            // Nothing else to do
            return;
        }
        // Need to transition for presenting
        let present_image_barriers = vec![Attachment::default_attachment_present_barrier(
            default_attachment.image,
        )];
        let barrier_dep_info = vk::DependencyInfo::builder()
            .image_memory_barriers(&present_image_barriers)
            .build();
        unsafe {
            ctx.device
                .cmd_pipeline_barrier2(command_buffer, &barrier_dep_info);
        }
    }

    pub fn wait_for_previous_frame(
        &self,
        device: &ash::Device,
        current_frame: u64,
        total_stages: u32,
        semaphore: vk::Semaphore,
    ) {
        if self.is_validation_layer_enabled && current_frame < 1 {
            /*
             * If validation layers are enabled, don't wait the first frame to avoid
             * a validation false positive that locks the main thread for a few seconds
             */
            return;
        }
        let wait_value = [self.signal_value_for(current_frame, total_stages)];
        let pass_timeline_semaphores = [semaphore];
        let wait_info = vk::SemaphoreWaitInfo::builder()
            .values(&wait_value)
            .semaphores(&pass_timeline_semaphores)
            .build();
        unsafe {
            device
                .wait_semaphores(
                    &wait_info,
                    std::time::Duration::from_secs(1).as_nanos() as u64,
                )
                .unwrap()
        };
    }

    pub fn signal_next_frame(
        &self,
        device: &ash::Device,
        current_frame: u64,
        total_stages: u32,
        semaphore: vk::Semaphore,
        queue: vk::Queue,
    ) {
        let signal_value = self.signal_value_for(current_frame + 1, total_stages);
        let pass_semaphore_signal_info = [vk::SemaphoreSubmitInfo::builder()
            .semaphore(semaphore)
            .stage_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE)
            .value(signal_value)
            .build()];
        let signal_submit_infos = [vk::SubmitInfo2::builder()
            .signal_semaphore_infos(&pass_semaphore_signal_info)
            .build()];
        unsafe {
            device
                .queue_submit2(queue, &signal_submit_infos, vk::Fence::null())
                .unwrap()
        };
    }

    fn signal_value_for(&self, current_frame: u64, total_stages: u32) -> u64 {
        crate::pipeline::signal_value_for(current_frame, total_stages, self.index)
    }

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
