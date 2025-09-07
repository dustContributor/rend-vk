use core::panic;
use std::{
    alloc::Layout,
    collections::HashMap,
    ffi::CStr,
    mem::align_of,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

use ash::{
    extensions::{ext::DebugUtils, khr},
    util::Align,
    vk, Entry,
};
use bitvec::vec::BitVec;

use crate::{
    buffer::{DeviceAllocator, DeviceSlice},
    context::{self, ExtensionContext, VulkanContext},
    debug::{self, DebugContext},
    format::Format,
    pipeline::{
        self,
        attachment::Attachment,
        sampler::{Sampler, SamplerKey},
        Pipeline,
    },
    render_task::{RenderTask, TaskKind},
    shader_resource::{ResourceKind, SingleResource},
    swapchain,
    texture::{MipMap, Texture},
    UsedAsIndex,
};

#[derive(Clone)]
pub struct MeshBuffer {
    pub vertices: DeviceSlice,
    pub normals: DeviceSlice,
    pub tex_coords: DeviceSlice,
    pub indices: DeviceSlice,
    pub count: u32,
}

#[derive(Clone)]
pub struct AllocatorStats {
    pub size: u64,
    pub available: u64,
    pub used: u64,
    pub alignment: u64,
    pub chunks: u64,
}

pub struct Renderer {
    pub vulkan_context: Rc<context::VulkanContext>,
    swapchain_context: Box<swapchain::SwapchainContext>,
    debug_context: Option<Box<debug::DebugContext>>,
    pipeline: Box<Pipeline>,
    general_allocator: Box<DeviceAllocator>,
    mesh_buffers_by_id: HashMap<u32, MeshBuffer>,
    textures_by_id: HashMap<u32, Texture>,
    shader_resources_by_kind: HashMap<ResourceKind, SingleResource>,
    batches_by_task_type: HashMap<u64, Vec<RenderTask>>,
    mesh_buffer_ids: BitVec,

    optimal_transition_queue: Vec<u32>,
    ongoing_optimal_transitions: Vec<(u32, u64)>,

    main_queue: vk::Queue,

    pool: vk::CommandPool,
    draw_command_buffer: vk::CommandBuffer,

    present_complete_semaphore: vk::Semaphore,
    rendering_complete_semaphore: vk::Semaphore,
    pass_timeline_semaphore: vk::Semaphore,

    draw_commands_reuse_fence: vk::Fence,
    setup_commands_reuse_fence: vk::Fence,

    current_frame: AtomicU64,
}

impl Renderer {
    pub const ID_TEST_TRIANGLE: u32 = 0;
    pub const MAX_SAMPLERS: u32 = 32;

    pub fn destroy(&mut self) {
        log::trace!("destroying renderer...");
        unsafe { self.vulkan_context.device.device_wait_idle().unwrap() };
        self.pipeline.destroy(&self.vulkan_context.device);
        self.general_allocator.destroy(&self.vulkan_context.device);
        unsafe {
            let destroy_semaphore = |s| self.vulkan_context.device.destroy_semaphore(s, None);
            let destroy_fence = |s| self.vulkan_context.device.destroy_fence(s, None);
            destroy_semaphore(self.present_complete_semaphore);
            destroy_semaphore(self.rendering_complete_semaphore);
            destroy_semaphore(self.pass_timeline_semaphore);
            destroy_fence(self.draw_commands_reuse_fence);
            destroy_fence(self.setup_commands_reuse_fence);
            self.vulkan_context
                .device
                .destroy_command_pool(self.pool, None);
            self.swapchain_context.destroy(&self.vulkan_context);
            self.vulkan_context.device.destroy_device(None);
        }
        // TODO: Read about Drop
        if self.debug_context.is_some() {
            let d = self.debug_context.as_mut().unwrap();
            d.destroy();
        }
        unsafe { self.vulkan_context.instance.destroy_instance(None) };
        log::trace!("renderer destroyed!");
    }

    pub fn add_task_to_queue(&mut self, task: RenderTask, parent_id: u32) {
        let key = task.kind.to_key(parent_id);
        let tasks = self.batches_by_task_type.entry(key).or_default();
        tasks.push(task);
    }

    pub fn try_get_sampler(&self, key: SamplerKey) -> Option<u8> {
        self.pipeline.samplers_by_key.get(&key).map(|s| s.position)
    }

    pub fn get_sampler(&mut self, key: SamplerKey) -> u8 {
        let id = self.try_get_sampler(key);
        if let Some(id) = id {
            return id;
        }
        //  Sampler for this key not found, generate one
        let id = self.pipeline.samplers_by_key.len() as u32;
        let sampler = Sampler::of_key(&self.vulkan_context, key, id as u8);
        let samplers_by_key = &mut self.pipeline.samplers_by_key;
        //  store it for later querying
        samplers_by_key.insert(key, sampler.clone());
        let sampler_descriptors = &mut self.pipeline.sampler_descriptors;
        // Write its descriptor into the GPU for later shader usage
        sampler_descriptors.place_sampler_at(&self.vulkan_context, id, sampler.sampler);
        // Return the ID for referencing on the client side
        id as u8
    }

    pub fn fetch_mesh(&self, id: u32) -> Option<&MeshBuffer> {
        self.mesh_buffers_by_id.get(&id)
    }

    pub fn fetch_mesh_or_fail(&self, id: u32) -> &MeshBuffer {
        self.fetch_mesh(id)
            .unwrap_or_else(|| panic!("couldn't find mesh with id {}", id))
    }

    pub fn free_mesh(&mut self, id: u32) {
        let mesh = self
            .mesh_buffers_by_id
            .remove(&id)
            .unwrap_or_else(|| panic!("couldn't find mesh with id {}", id));
        let free_if_not_empty = |v: &DeviceSlice| {
            if v.size > 0 {
                self.general_allocator.free(*v);
            }
        };
        free_if_not_empty(&mesh.vertices);
        free_if_not_empty(&mesh.normals);
        free_if_not_empty(&mesh.tex_coords);
        free_if_not_empty(&mesh.indices);
        self.mesh_buffer_ids.set(id as usize, false);
    }

    pub fn gen_mesh(
        &mut self,
        vertices_size: u32,
        normals_size: u32,
        tex_coords_size: u32,
        indices_size: u32,
        count: u32,
    ) -> u32 {
        let alloc_or_empty = |size: u32, purpose: &str| {
            if size > 0 {
                self.general_allocator
                    .alloc(size as u64)
                    .unwrap_or_else(|| {
                        panic!("couldnt allocate '{}' buffer of size {}", purpose, size)
                    })
            } else {
                DeviceSlice::empty()
            }
        };

        let vertices = alloc_or_empty(vertices_size, "vertex");
        let normals = alloc_or_empty(normals_size, "normal");
        let tex_coords = alloc_or_empty(tex_coords_size, "tex_coord");
        let indices = alloc_or_empty(indices_size, "index");
        // Reserve mesh id
        let mesh_id = self
            .mesh_buffer_ids
            .first_zero()
            .expect("ran out of mesh ids!") as u32;

        self.mesh_buffer_ids.set(mesh_id as usize, true);

        self.mesh_buffers_by_id.insert(
            mesh_id,
            MeshBuffer {
                vertices,
                normals,
                tex_coords,
                indices,
                count,
            },
        );

        mesh_id
    }

    pub fn fetch_texture(&self, id: u32) -> Option<&Texture> {
        self.textures_by_id.get(&id)
    }

    pub fn gen_texture(
        &mut self,
        name: String,
        format: crate::format::Format,
        mip_maps: &[MipMap],
        staging_size: u32,
    ) -> u32 {
        // Reserve texture id
        let texture_id = self.pipeline.image_descriptors.next_free() as u32;
        let staging = if staging_size > 0 {
            Some(Box::new(
                self.general_allocator
                    .alloc(staging_size as u64)
                    .unwrap_or_else(|| {
                        panic!(
                            "can't allocate staging buffer of size {} for {}",
                            name, staging_size
                        )
                    }),
            ))
        } else {
            None
        };
        let texture = Texture {
            id: texture_id,
            mip_maps: mip_maps.into(),
            staging,
            ..crate::texture::make(
                &self.vulkan_context,
                name,
                mip_maps[0].width,
                mip_maps[0].height,
                mip_maps.len() as u8,
                format,
                false,
            )
        };
        // Generate descriptor and place it in the image descriptor array buffer
        self.pipeline.image_descriptors.place_image_at(
            &self.vulkan_context,
            texture_id,
            texture.view,
            vk::ImageLayout::READ_ONLY_OPTIMAL,
        );
        self.textures_by_id.insert(texture_id, texture);
        texture_id
    }

    pub fn queue_texture_for_uploading(&mut self, id: u32) {
        if !self.textures_by_id.contains_key(&id) {
            panic!("missing texture with id {}", id);
        }
        self.optimal_transition_queue.push(id);
    }

    pub fn is_texture_uploaded(&self, id: u32) -> bool {
        let texture = self
            .textures_by_id
            .get(&id)
            .unwrap_or_else(|| panic!("missing texture with id {}", id));
        // If it no longer has staging memory, then it's uploaded
        texture.staging.is_none()
    }

    pub fn place_shader_resource(&mut self, kind: ResourceKind, item: SingleResource) {
        self.shader_resources_by_kind.insert(kind, item);
    }

    pub fn get_current_frame(&self) -> u64 {
        self.current_frame.load(Ordering::Relaxed)
    }

    pub fn get_allocator_stats(&self) -> AllocatorStats {
        AllocatorStats {
            size: self.general_allocator.size(),
            alignment: self.general_allocator.alignment(),
            available: self.general_allocator.available(),
            used: self.general_allocator.used(),
            chunks: self.general_allocator.chunks(),
        }
    }

    pub fn render(&mut self) {
        // TODO: Trace feature to write RenderTask to files
        // if log::log_enabled!(log::Level::Trace) {
        //     let base_path = std::path::Path::new("log");
        //     std::fs::DirBuilder::new()
        //         .recursive(true)
        //         .create(&base_path)
        //         .expect(&format!(
        //             "failed creating the log folder at {}!",
        //             base_path.to_str().unwrap()
        //         ));
        //     let dst_path = base_path.join(format!(
        //         "rendvk_renderer_rendertasks_{:?}.json",
        //         self.current_frame
        //     ));
        //     let dst_file = File::create(dst_path).unwrap();
        //     let mut writer = BufWriter::new(dst_file);
        //     serde_json::to_writer_pretty(&mut writer, &self.batches_by_task_type).unwrap();
        //     writer.flush().unwrap();
        // }
        let (attachment_index, default_attachment) = self.acquire_next_swapchain_attachment();

        self.setup_frame();

        self.record_and_submit_draw_commands(|r, c| {
            r.process_pipeline(c, &default_attachment);
        });

        self.present(attachment_index);

        // Clear batch queues for next frame
        for batch in &mut self.batches_by_task_type.values_mut() {
            batch.clear();
        }
        // Signal current frame and increment ID for next frame
        self.signal_frame();
    }

    fn acquire_next_swapchain_attachment(&self) -> (u32, Attachment) {
        let (present_index, _) = unsafe {
            self.vulkan_context
                .extension
                .swapchain
                .acquire_next_image(
                    self.swapchain_context.swapchain,
                    u64::MAX,
                    self.present_complete_semaphore,
                    vk::Fence::null(),
                )
                .unwrap()
        };
        let attachment = self.swapchain_context.attachments[present_index as usize].clone();
        (present_index, attachment)
    }

    fn present(&self, swapchain_attachment_index: u32) {
        let wait_semaphores = [self.rendering_complete_semaphore];
        let swapchains = [self.swapchain_context.swapchain];
        let image_indices = [swapchain_attachment_index];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&wait_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);
        unsafe {
            self.vulkan_context
                .extension
                .swapchain
                .queue_present(self.main_queue, &present_info)
                .expect("queue present failed!");
        };
    }

    fn setup_frame(&mut self) {
        if self.ongoing_optimal_transitions.is_empty() {
            return;
        }
        // Process any queued texture transitions
        let current_timeline_counter = unsafe {
            self.vulkan_context
                .device
                .get_semaphore_counter_value(self.pass_timeline_semaphore)
                .unwrap()
        };
        self.ongoing_optimal_transitions.retain(|e| {
            if e.1 >= current_timeline_counter {
                return true;
            }
            let texture = &mut self.textures_by_id.get_mut(&e.0).unwrap();
            // Free the staging buffer after it has been used
            match &texture.staging {
                Some(staging) => {
                    let device = *staging.as_ref();
                    self.general_allocator.free(device);
                }
                _ => panic!(
                    "staging buffer for texture {} {} is missing!",
                    texture.id, texture.name
                ),
            }
            // Set staging to None to mark the texture as "uploaded"
            texture.staging = None;
            // No longer retain the transition, already uploaded
            false
        });
    }

    fn process_pipeline(
        &mut self,
        command_buffer: vk::CommandBuffer,
        default_attachment: &Attachment,
    ) {
        let current_frame = self.get_current_frame();
        let sampler_descriptors = self.pipeline.sampler_descriptors.clone();
        let image_descriptors = self.pipeline.image_descriptors.clone();

        self.vulkan_context
            .try_begin_debug_label(command_buffer, "issue_queued_transitions");
        for texture_id in self.optimal_transition_queue.drain(..) {
            let texture = &self.textures_by_id[&texture_id];
            texture.transition_to_optimal(&self.vulkan_context, self.draw_command_buffer);
            self.ongoing_optimal_transitions
                .push((texture_id, current_frame))
        }
        self.vulkan_context.try_end_debug_label(command_buffer);

        self.pipeline.process_stages(pipeline::RenderContext {
            vulkan: &self.vulkan_context,
            batches_by_task_type: &self.batches_by_task_type,
            mesh_buffers_by_id: &self.mesh_buffers_by_id,
            shader_resources_by_kind: &self.shader_resources_by_kind,
            sampler_descriptors: &sampler_descriptors,
            image_descriptors: &image_descriptors,
            buffer_allocator: &self.general_allocator,
            command_buffer,
            default_attachment,
        });
    }

    fn signal_frame(&self) {
        let frame_index = self.current_frame.fetch_add(1, Ordering::Relaxed);
        let pass_semaphore_signal_info = [vk::SemaphoreSubmitInfo::builder()
            .semaphore(self.pass_timeline_semaphore)
            .stage_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE)
            .value(frame_index)
            .build()];
        let signal_submit_infos = [vk::SubmitInfo2::builder()
            .signal_semaphore_infos(&pass_semaphore_signal_info)
            .build()];
        unsafe {
            self.vulkan_context
                .device
                .queue_submit2(self.main_queue, &signal_submit_infos, vk::Fence::null())
                .unwrap()
        };
    }

    fn wait_and_reset_fence(&self, fence: vk::Fence) {
        let fences = [fence];
        unsafe {
            self.vulkan_context
                .device
                .wait_for_fences(&fences, true, u64::MAX)
                .expect("fence wait failed!");
            self.vulkan_context
                .device
                .reset_fences(&fences)
                .expect("fence reset failed!");
        }
    }

    /// Main draw command recording and submission logic
    fn record_and_submit_draw_commands<F>(&mut self, recording: F)
    where
        F: Fn(&mut Renderer, vk::CommandBuffer),
    {
        self.wait_and_reset_fence(self.draw_commands_reuse_fence);

        unsafe {
            self.vulkan_context
                .device
                .reset_command_buffer(
                    self.draw_command_buffer,
                    vk::CommandBufferResetFlags::RELEASE_RESOURCES,
                )
                .expect("reset command buffer failed!");

            let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
                .build();

            self.vulkan_context
                .device
                .begin_command_buffer(self.draw_command_buffer, &command_buffer_begin_info)
                .expect("begin commandbuffer failed!");

            recording(self, self.draw_command_buffer);

            self.vulkan_context
                .device
                .end_command_buffer(self.draw_command_buffer)
                .expect("end command buffer failed!");

            let command_buffers = [self.draw_command_buffer];

            let wait_mask = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let wait_semaphores = [self.present_complete_semaphore];
            let signal_semaphores = [self.rendering_complete_semaphore];

            let submit_info = vk::SubmitInfo::builder()
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&wait_mask)
                .command_buffers(&command_buffers)
                .signal_semaphores(&signal_semaphores);

            let submit_infos = [submit_info.build()];

            self.vulkan_context
                .device
                .queue_submit(
                    self.main_queue,
                    &submit_infos,
                    self.draw_commands_reuse_fence,
                )
                .expect("queue submit failed!");
        }
    }

    /// Used for renderer initialization, where several commands
    /// have to be submitted to  transition render targets
    /// for example. Since it's part of renderer initialization, it just
    /// waits for idle as synchronization mechanism
    fn submit_and_wait<F>(&mut self, recording: F)
    where
        F: Fn(&mut Renderer, vk::CommandBuffer),
    {
        let cmd_buffer = self.draw_command_buffer;
        unsafe {
            self.vulkan_context
                .device
                .reset_command_buffer(cmd_buffer, vk::CommandBufferResetFlags::RELEASE_RESOURCES)
                .expect("reset command buffer failed!");

            let begin_info = vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

            self.vulkan_context
                .device
                .begin_command_buffer(cmd_buffer, &begin_info)
                .expect("begin commandbuffer failed!");

            recording(self, cmd_buffer);

            self.vulkan_context
                .device
                .end_command_buffer(cmd_buffer)
                .expect("end command buffer failed!");

            let wait_mask = [vk::PipelineStageFlags::ALL_COMMANDS];
            let command_buffers = [cmd_buffer];

            let submit_info = vk::SubmitInfo::builder()
                .wait_dst_stage_mask(&wait_mask)
                .wait_semaphores(&[])
                .command_buffers(&command_buffers);

            let submit_infos = [submit_info.build()];

            self.vulkan_context
                .device
                .reset_fences(&[self.draw_commands_reuse_fence])
                .expect("fence reset failed!");

            self.vulkan_context
                .device
                .queue_submit(
                    self.main_queue,
                    &submit_infos,
                    self.draw_commands_reuse_fence,
                )
                .expect("queue submit failed!");

            self.vulkan_context
                .device
                .device_wait_idle()
                .expect("wait idle failed!");
        };
    }
}

pub fn make_renderer<F>(
    is_vsync_enabled: bool,
    is_debug_enabled: bool,
    is_validation_layer_enabled: bool,
    instance_extensions: &[*const i8],
    create_surface: F,
) -> Renderer
where
    F: FnOnce(&ash::Entry, &ash::Instance, *mut vk::SurfaceKHR) -> vk::Result,
{
    log::trace!("entering make_renderer");

    log::trace!("creating entry...");
    let entry = Entry::linked();
    log::trace!("entry created!");
    log::trace!("creating instance...");
    let instance = make_instance(
        &entry,
        instance_extensions,
        is_debug_enabled,
        is_validation_layer_enabled,
    );
    log::trace!("instance created!");

    let debug_context = if is_debug_enabled {
        Some(Box::new(DebugContext::new(&entry, &instance)))
    } else {
        None
    };

    let debug_utils_ext = if is_debug_enabled {
        Some(DebugUtils::new(&entry, &instance))
    } else {
        None
    };
    log::trace!("creating surface...");
    let surface_layout = Layout::new::<vk::SurfaceKHR>();
    let surface = unsafe { std::alloc::alloc(surface_layout) as *mut vk::SurfaceKHR };
    let create_surface_result = create_surface(&entry, &instance, surface);
    if create_surface_result != vk::Result::SUCCESS {
        panic!("error creating surface: {}", create_surface_result);
    }
    let surface = unsafe { *surface };
    log::trace!("surface created!");
    let surface_extension = khr::Surface::new(&entry, &instance);
    // let make_surface = func: unsafe extern "C" fn(u64, *mut c_void),
    log::trace!("selecting physical device...");
    let (physical_device, name, queue_family_index) =
        select_physical_device(&instance, &surface_extension, surface);
    log::trace!("physical device {name} with queue index {queue_family_index} selected!");
    log::trace!("creating device...");
    let device = make_device(
        &instance,
        physical_device,
        queue_family_index,
        is_debug_enabled,
    );
    log::trace!("device created!");

    let swapchain_extension = ash::extensions::khr::Swapchain::new(&instance, &device);

    let mem_props = unsafe { instance.get_physical_device_memory_properties(physical_device) };

    let ctx = Rc::new(VulkanContext {
        entry,
        device,
        instance,
        physical_device,
        memory_properties: mem_props,
        extension: ExtensionContext {
            debug_utils: debug_utils_ext,
            swapchain: swapchain_extension,
            surface: surface_extension,
        },
    });

    ctx.try_set_debug_name("main_physical_device", physical_device);
    ctx.try_set_debug_name("main_device", ctx.device.handle());
    ctx.try_set_debug_name("main_instance", ctx.instance.handle());

    log::trace!("creating command buffers...");
    let main_queue = unsafe { ctx.device.get_device_queue(queue_family_index, 0) };
    ctx.try_set_debug_name("main_queue", main_queue);

    let pool_create_info = vk::CommandPoolCreateInfo::builder()
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
        .queue_family_index(queue_family_index);

    let command_pool = unsafe {
        ctx.device
            .create_command_pool(&pool_create_info, None)
            .unwrap()
    };
    ctx.try_set_debug_name("main_command_pool", command_pool);

    let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
        .command_buffer_count(2)
        .command_pool(command_pool)
        .level(vk::CommandBufferLevel::PRIMARY);

    let command_buffers = unsafe {
        ctx.device
            .allocate_command_buffers(&command_buffer_allocate_info)
            .unwrap()
    };
    let setup_command_buffer = command_buffers[0];
    let draw_command_buffer = command_buffers[1];
    ctx.try_set_debug_name("setup_command_buffer", setup_command_buffer);
    ctx.try_set_debug_name("draw_command_buffer", draw_command_buffer);
    log::trace!("command buffers created!");

    log::trace!("creating fences...");
    let fence_create_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
    let draw_commands_reuse_fence = unsafe {
        ctx.device
            .create_fence(&fence_create_info, None)
            .expect("Create fence failed.")
    };
    let setup_commands_reuse_fence = unsafe {
        ctx.device
            .create_fence(&fence_create_info, None)
            .expect("Create fence failed.")
    };
    ctx.try_set_debug_name("draw_commands_reuse_fence", draw_commands_reuse_fence);
    ctx.try_set_debug_name("setup_commands_reuse_fence", setup_commands_reuse_fence);
    log::trace!("fences created!");

    log::trace!("creating semaphores...");
    let semaphore_create_info = vk::SemaphoreCreateInfo::default();
    let present_complete_semaphore = unsafe {
        ctx.device
            .create_semaphore(&semaphore_create_info, None)
            .unwrap()
    };
    let rendering_complete_semaphore = unsafe {
        ctx.device
            .create_semaphore(&semaphore_create_info, None)
            .unwrap()
    };
    ctx.try_set_debug_name("present_complete_semaphore", present_complete_semaphore);
    ctx.try_set_debug_name("rendering_complete_semaphore", rendering_complete_semaphore);
    let mut timeline_semaphore_type_create_info = vk::SemaphoreTypeCreateInfo::builder()
        .initial_value(0)
        .semaphore_type(vk::SemaphoreType::TIMELINE)
        .build();
    let timeline_semaphore_create_info = vk::SemaphoreCreateInfo::builder()
        .push_next(&mut timeline_semaphore_type_create_info)
        .build();
    let pass_timeline_semaphore = unsafe {
        ctx.device
            .create_semaphore(&timeline_semaphore_create_info, None)
            .unwrap()
    };
    ctx.try_set_debug_name("pass_timeline_semaphore", pass_timeline_semaphore);
    log::trace!("semaphores created!");

    log::trace!("creating allocators...");
    let mut general_allocator = DeviceAllocator::new_general(ctx.clone(), 64 * 1024 * 1024);
    log::trace!("allocators created!");

    log::trace!("creating swapchain...");
    let swapchain_context = swapchain::SwapchainContext::make(&ctx, surface, is_vsync_enabled);
    log::trace!("swapchain created!");

    log::trace!("creating pipeline...");
    let pip = pipeline::file::Pipeline::load(
        &ctx,
        swapchain_context.attachments[0].clone(),
        is_validation_layer_enabled,
        Some("pipeline.json"),
    );
    log::trace!("pipeline created!");

    log::trace!("creating test triangle...");
    let test_triangle = make_test_triangle(&mut general_allocator);

    let mut mesh_buffer_ids = BitVec::repeat(false, 1024);
    let mut mesh_buffers_by_id = HashMap::new();
    mesh_buffer_ids.set(Renderer::ID_TEST_TRIANGLE as usize, true);
    mesh_buffers_by_id.insert(Renderer::ID_TEST_TRIANGLE, test_triangle);

    let textures_by_id = HashMap::new();

    log::trace!("test triangle created!");
    let batches_by_task_type = HashMap::with_capacity(TaskKind::MAX_SIZE * 2);

    log::trace!("finishing renderer...");
    let mut renderer = Renderer {
        pipeline: Box::new(pip),
        batches_by_task_type,
        debug_context,
        swapchain_context: Box::new(swapchain_context),
        vulkan_context: ctx,
        general_allocator: Box::new(general_allocator),
        mesh_buffers_by_id,
        mesh_buffer_ids,
        textures_by_id,
        draw_command_buffer,
        main_queue,
        rendering_complete_semaphore,
        pass_timeline_semaphore,
        present_complete_semaphore,
        setup_commands_reuse_fence,
        draw_commands_reuse_fence,
        pool: command_pool,
        optimal_transition_queue: Vec::new(),
        ongoing_optimal_transitions: Vec::new(),
        shader_resources_by_kind: HashMap::new(),
        current_frame: AtomicU64::new(1),
    };
    // Reserve the texture ID 0 with an empty texture
    renderer.gen_texture(
        "default_texture".to_string(),
        Format::R8G8B8A8_UNORM,
        &[MipMap {
            index: 0,
            size: 4,
            offset: 0,
            width: 1,
            height: 1,
        }],
        0,
    );
    log::trace!("issuing initial layout transitions...");
    renderer.submit_and_wait(|r, c| {
        let barriers = r.pipeline.gen_initial_barriers();
        let barrier_dep_info = vk::DependencyInfo::builder()
            .image_memory_barriers(&barriers)
            .build();
        unsafe {
            r.vulkan_context
                .device
                .cmd_pipeline_barrier2(c, &barrier_dep_info);
        }
    });
    log::trace!("initial layout transitions issued!");
    log::trace!("renderer finished!");
    // Return initialized renderer
    renderer
}

pub fn make_device(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    queue_family_index: u32,
    is_debug_enabled: bool,
) -> ash::Device {
    let mut device_extension_names_raw = vec![khr::Swapchain::name().as_ptr()];
    let non_semantic_info_name = c"VK_KHR_shader_non_semantic_info";
    if is_debug_enabled {
        device_extension_names_raw.push(non_semantic_info_name.as_ptr());
    }
    let features = vk::PhysicalDeviceFeatures {
        shader_clip_distance: 1,
        depth_clamp: 1,
        sampler_anisotropy: 1,
        ..Default::default()
    };
    let mut features12 = vk::PhysicalDeviceVulkan12Features {
        descriptor_indexing: 1,
        descriptor_binding_update_unused_while_pending: 1,
        descriptor_binding_partially_bound: 1,
        descriptor_binding_sampled_image_update_after_bind: 1,
        timeline_semaphore: 1,
        buffer_device_address: 1,
        scalar_block_layout: 1,
        runtime_descriptor_array: 1,
        shader_sampled_image_array_non_uniform_indexing: 1,
        storage_buffer8_bit_access: 1,
        shader_int8: 1,
        ..Default::default()
    };
    let mut features13 = vk::PhysicalDeviceVulkan13Features {
        dynamic_rendering: 1,
        synchronization2: 1,
        ..Default::default()
    };
    // OpenGL NDC from -1 to 1 on depth, instead of 0 to 1
    // let mut depth_clip_control_feature = vk::PhysicalDeviceDepthClipControlFeaturesEXT {
    //     depth_clip_control: 1,
    //     ..Default::default()
    // };
    let mut features2 = vk::PhysicalDeviceFeatures2::builder()
        .features(features)
        .push_next(&mut features12)
        .push_next(&mut features13)
        // .push_next(&mut depth_clip_control_feature)
        .build();

    let priorities = [1.0];

    let queue_info = vk::DeviceQueueCreateInfo::builder()
        .queue_family_index(queue_family_index)
        .queue_priorities(&priorities)
        .build();

    let device_create_info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(std::slice::from_ref(&queue_info))
        .enabled_extension_names(&device_extension_names_raw)
        .push_next(&mut features2)
        .build();

    log::info!("initializing Device...");
    let device: ash::Device = unsafe {
        instance
            .create_device(physical_device, &device_create_info, None)
            .expect("couldn't create the device!")
    };
    log::info!("device initialized!");
    // Return initialized device
    device
}

pub fn make_instance(
    entry: &ash::Entry,
    extensions: &[*const i8],
    is_debug_enabled: bool,
    is_validation_layer_enabled: bool,
) -> ash::Instance {
    let app_name = c"rend-vk";

    let mut layers_names_raw = vec![];

    let validation_layer_name = c"VK_LAYER_KHRONOS_validation";
    if is_debug_enabled && is_validation_layer_enabled {
        layers_names_raw.push(validation_layer_name.as_ptr());
    }

    let mut instance_extensions = extensions.to_vec();
    if is_debug_enabled {
        instance_extensions.push(DebugUtils::name().as_ptr());
    }

    let appinfo = vk::ApplicationInfo::builder()
        .application_name(app_name)
        .application_version(0)
        .engine_name(app_name)
        .engine_version(0)
        .api_version(vk::make_api_version(0, 1, 3, 0));

    let mut create_info = vk::InstanceCreateInfo::builder()
        .application_info(&appinfo)
        .enabled_layer_names(&layers_names_raw)
        .enabled_extension_names(&instance_extensions);

    let enabled_validation_features = [vk::ValidationFeatureEnableEXT::DEBUG_PRINTF];
    let mut validation_features_ext = vk::ValidationFeaturesEXT::builder()
        .enabled_validation_features(&enabled_validation_features)
        .build();

    if is_debug_enabled {
        create_info = create_info.push_next(&mut validation_features_ext);
    }

    log::info!("initializing Instance...");
    let instance: ash::Instance = unsafe {
        entry
            .create_instance(&create_info, None)
            .expect("instance creation error!")
    };
    log::info!("instance initialized!");
    // Return initialized instance
    instance
}

pub fn select_physical_device(
    instance: &ash::Instance,
    surface_extension: &khr::Surface,
    window_surface: vk::SurfaceKHR,
) -> (vk::PhysicalDevice, String, u32) {
    let devices = unsafe {
        instance
            .enumerate_physical_devices()
            .expect("Physical device error")
    };
    let mut tmp: Vec<_> = devices
        .iter()
        .map(|pdevice| {
            let properties = unsafe { instance.get_physical_device_properties(*pdevice) };
            let is_discrete = vk::PhysicalDeviceType::DISCRETE_GPU == properties.device_type;
            let tmp_bytes: Vec<_> = properties
                .device_name
                .into_iter()
                .map(|b| b as u8)
                .collect();
            let device_name = CStr::from_bytes_until_nul(&tmp_bytes).unwrap();
            let supports_graphic_and_surface = unsafe {
                instance
                    .get_physical_device_queue_family_properties(*pdevice)
                    .iter()
                    .enumerate()
                    .find_map(|(index, info)| {
                        let supports_graphic_and_surface =
                            info.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                                && surface_extension
                                    .get_physical_device_surface_support(
                                        *pdevice,
                                        index as u32,
                                        window_surface,
                                    )
                                    .unwrap();
                        if supports_graphic_and_surface {
                            Some((
                                *pdevice,
                                device_name.to_str().unwrap().to_owned(),
                                index as u32,
                            ))
                        } else {
                            None
                        }
                    })
            };
            (is_discrete, supports_graphic_and_surface)
        })
        .collect();
    tmp.sort_by(|a, b| {
        // Prefer discrete devices
        if a.0 && !b.0 {
            return std::cmp::Ordering::Less;
        }
        if !a.0 && b.0 {
            return std::cmp::Ordering::Greater;
        }
        // Prefer devices with graphics queue and surface support
        if a.1.is_some() && b.1.is_none() {
            return std::cmp::Ordering::Less;
        }
        if a.1.is_none() && b.1.is_some() {
            return std::cmp::Ordering::Greater;
        }
        // Otherwise determine equal
        std::cmp::Ordering::Equal
    });
    // Just pick the first and use it
    tmp.into_iter()
        .find_map(|e| e.1)
        .expect("couldn't find a suitable physical device!")
}

fn make_test_triangle(buffer_allocator: &mut DeviceAllocator) -> MeshBuffer {
    #[allow(dead_code)]
    #[derive(Clone, Debug, Copy)]
    struct Attrib3f {
        pub values: [f32; 3],
    }
    #[allow(dead_code)]
    #[derive(Clone, Debug, Copy)]
    struct Attrib2f {
        pub values: [f32; 2],
    }
    let vertices = [
        Attrib3f {
            values: [-1.0, 1.0, 0.0],
        },
        Attrib3f {
            values: [1.0, 1.0, 0.0],
        },
        Attrib3f {
            values: [0.0, -1.0, 0.0],
        },
    ];
    let tex_coords = [
        Attrib2f { values: [0.0, 0.0] },
        Attrib2f { values: [1.0, 0.0] },
        Attrib2f { values: [1.0, 1.0] },
    ];
    let normals = [
        Attrib3f {
            values: [0.0, 1.0, 0.0],
        },
        Attrib3f {
            values: [1.0, 1.0, 0.0],
        },
        Attrib3f {
            values: [1.0, 0.0, 0.0],
        },
    ];

    fn alloc_and_copy<T: std::marker::Copy>(
        elements: &[T],
        buffer_allocator: &mut DeviceAllocator,
    ) -> DeviceSlice {
        let buffer = buffer_allocator
            .alloc(std::mem::size_of_val(elements) as u64)
            .expect("couldn't allocate index buffer");
        let mut slice = unsafe {
            Align::new(
                buffer.addr,
                align_of::<u32>() as u64,
                buffer_allocator.alignment(),
            )
        };
        slice.copy_from_slice(elements);
        buffer
    }

    let vertex_buffer = alloc_and_copy(&vertices, buffer_allocator);
    let normal_buffer = alloc_and_copy(&normals, buffer_allocator);
    let tex_coord_buffer = alloc_and_copy(&tex_coords, buffer_allocator);

    MeshBuffer {
        vertices: vertex_buffer,
        indices: DeviceSlice::empty(),
        tex_coords: tex_coord_buffer,
        normals: normal_buffer,
        count: vertices.len() as u32,
    }
}
