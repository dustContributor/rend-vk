use std::collections::HashMap;

use crate::{
    buffer::{DeviceAllocator, DeviceSlice},
    pipeline::{attachment::Attachment, descriptor::DescriptorBuffer},
    render_task::{RenderTask, ResourceKind, TaskKind},
    renderer::MeshBuffer,
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
    pub updaters: Vec<ResourceKind>,
    pub input_descriptors: Option<Box<DescriptorBuffer>>,
    pub ubo_descriptors: Option<Box<DescriptorBuffer>>,
    pub task_kind: TaskKind,
    pub index: u32,
    pub is_final: bool,
    pub image_barriers: Vec<vk::ImageMemoryBarrier2>,
    pub reserved_buffers: Vec<DeviceSlice>,
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
        let barrier_dep_info = vk::DependencyInfo::builder()
            .image_memory_barriers(&image_barriers)
            .build();
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
        let rendering_info_builder = vk::RenderingInfo::builder()
            .color_attachments(&rendering_attachments)
            .render_area(if let Some(att) = self.outputs.first() {
                att.render_area_no_offset()
            } else {
                default_attachment.render_area_no_offset()
            })
            .layer_count(1);
        let rendering_info = if let Some(att) = self.rendering.depth_stencil {
            rendering_info_builder.depth_attachment(&att).build()
        } else {
            rendering_info_builder.build()
        };
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
        let mut desc_buffer_offsets = vec![
            sampler_descriptors.device.offset,
            image_descriptors.device.offset,
        ];
        if let Some(desc) = &self.input_descriptors {
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
        }
        unsafe {
            ctx.device
                .cmd_pipeline_barrier2(command_buffer, &barrier_dep_info);
            ctx.device
                .cmd_begin_rendering(command_buffer, &rendering_info);
            ctx.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );
        }
        if self.task_kind == TaskKind::Fullscreen {
            unsafe { ctx.device.cmd_draw(command_buffer, 3, 1, 0, 0) }
        } else {
            let tasks = &batches_by_task_type[self.task_kind.to_usize()];
            for task in tasks {
                /* Upload all the per-instance data for this task */
                let mesh_buffer = mesh_buffers_by_id.get(&task.mesh_buffer_id).unwrap();
                let mut push_constants = vec![
                    mesh_buffer.vertices.device_addr,
                    mesh_buffer.normals.device_addr,
                    mesh_buffer.tex_coords.device_addr,
                ];
                // Append resource buffer addresses in the order they appear
                push_constants.append(&mut self.reserve_and_fill_buffers(&buffer_allocator, task));
                unsafe {
                    // Upload push constants
                    let push_constants = push_constants.align_to::<u8>().1;
                    ctx.device.cmd_push_constants(
                        command_buffer,
                        self.layout,
                        ShaderStageFlags::ALL_GRAPHICS,
                        0u32,
                        &push_constants,
                    );
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
        if crate::VALIDATION_LAYER_ENABLED && current_frame < 1 {
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

    fn reserve_and_fill_buffers(&mut self, mem: &DeviceAllocator, task: &RenderTask) -> Vec<u64> {
        // We'll need the addresses to pass them to the shaders later
        let mut device_addrs = Vec::new();
        device_addrs.reserve_exact(self.updaters.len());

        for kind in self.updaters.clone() {
            let buffer = updater::alloc_and_fill(mem, task, kind.clone());
            device_addrs.push(buffer.device_addr);
            self.reserved_buffers.push(buffer);
        }

        device_addrs
    }
}
