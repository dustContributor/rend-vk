use std::{alloc::Layout, ffi::CStr, mem::align_of};

use ash::{
    extensions::{
        ext::{self, DebugUtils},
        khr,
    },
    util::Align,
    vk, Entry,
};

use crate::{
    buffer::{DeviceAllocator, DeviceSlice},
    context::{self, ExtensionContext, VulkanContext},
    debug::{self, DebugContext},
    pipeline::{self, Pipeline},
    render_task::{RenderTask, TaskKind},
    shader, swapchain,
};

struct Mesh {
    vertices: DeviceSlice,
    colors: DeviceSlice,
    indices: DeviceSlice,
    indices_count: u32,
}

pub struct Renderer {
    pub pipeline: Pipeline,
    pub batches_by_task_type: Vec<Vec<RenderTask>>,
    pub swapchain_context: swapchain::SwapchainContext,
    pub vulkan_context: context::VulkanContext,
    pub debug_context: Option<debug::DebugContext>,
    pub buffer_allocator: DeviceAllocator,
    pub descriptor_allocator: DeviceAllocator,

    present_queue: vk::Queue,

    pool: vk::CommandPool,
    draw_command_buffer: vk::CommandBuffer,
    setup_command_buffer: vk::CommandBuffer,

    present_complete_semaphore: vk::Semaphore,
    rendering_complete_semaphore: vk::Semaphore,
    pass_timeline_semaphore: vk::Semaphore,

    draw_commands_reuse_fence: vk::Fence,
    setup_commands_reuse_fence: vk::Fence,

    test_triangle: Mesh,

    current_frame: u64,
}

impl Renderer {
    pub fn destroy(&mut self) {
        log::trace!("destroying renderer...");
        self.pipeline.destroy(&self.vulkan_context.device);
        for e in [&self.buffer_allocator, &self.descriptor_allocator] {
            e.destroy(&self.vulkan_context.device);
        }
        unsafe {
            let destroy_semaphore = |s| self.vulkan_context.device.destroy_semaphore(s, None);
            let destroy_fence = |s| self.vulkan_context.device.destroy_fence(s, None);
            self.vulkan_context.device.device_wait_idle().unwrap();
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

    pub fn add_task_to_queue(&mut self, task: RenderTask) {
        if let Some(batch) = self.batches_by_task_type.get_mut(task.kind as usize) {
            batch.push(task)
        }
    }

    pub fn render(&mut self) {
        unsafe {
            let (present_index, _) = self
                .vulkan_context
                .extension
                .swapchain
                .acquire_next_image(
                    self.swapchain_context.swapchain,
                    std::u64::MAX,
                    self.present_complete_semaphore,
                    vk::Fence::null(),
                )
                .unwrap();
            let default_attachment =
                self.swapchain_context.attachments[present_index as usize].clone();
            self.record_submit_commandbuffer(
                self.draw_command_buffer,
                self.draw_commands_reuse_fence,
                self.present_queue,
                &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT],
                &[self.present_complete_semaphore],
                &[self.rendering_complete_semaphore],
                |draw_command_buffer| {
                    for stage in &self.pipeline.stages {
                        let wait_value = [self.current_frame * self.pipeline.stages.len() as u64
                            + stage.index as u64];
                        let pass_timeline_semaphores = [self.pass_timeline_semaphore];
                        let wait_info = vk::SemaphoreWaitInfo::builder()
                            .values(&wait_value)
                            .semaphores(&pass_timeline_semaphores)
                            .build();
                        /*
                         * If validation layers are enabled, don't wait the first frame to avoid
                         * a validation false positive that locks the main thread for a few seconds
                         */
                        if !crate::VALIDATION_LAYER_ENABLED || self.current_frame > 0 {
                            self.vulkan_context
                                .device
                                .wait_semaphores(
                                    &wait_info,
                                    std::time::Duration::from_secs(1).as_nanos() as u64,
                                )
                                .unwrap();
                        }
                        stage.render(
                            &self.vulkan_context,
                            &self.pipeline,
                            draw_command_buffer,
                            &default_attachment,
                            |device, command_buffer| {
                                device.cmd_bind_vertex_buffers(
                                    command_buffer,
                                    shader::ATTRIB_LOC_POSITION,
                                    &[self.buffer_allocator.buffer.buffer],
                                    &[self.test_triangle.vertices.offset],
                                );
                                device.cmd_bind_vertex_buffers(
                                    command_buffer,
                                    shader::ATTRIB_LOC_COLOR,
                                    &[self.buffer_allocator.buffer.buffer],
                                    &[self.test_triangle.colors.offset],
                                );
                                device.cmd_bind_index_buffer(
                                    command_buffer,
                                    self.buffer_allocator.buffer.buffer,
                                    self.test_triangle.indices.offset,
                                    vk::IndexType::UINT32,
                                );
                                device.cmd_draw_indexed(
                                    command_buffer,
                                    self.test_triangle.indices_count,
                                    1,
                                    0,
                                    0,
                                    1,
                                );
                            },
                        );
                        let signal_value = ((self.current_frame + 1)
                            * self.pipeline.stages.len() as u64)
                            + stage.index as u64;
                        let pass_semaphore_signal_info = [vk::SemaphoreSubmitInfo::builder()
                            .semaphore(self.pass_timeline_semaphore)
                            .stage_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE)
                            .value(signal_value)
                            .build()];
                        let signal_submit_infos = [vk::SubmitInfo2::builder()
                            .signal_semaphore_infos(&pass_semaphore_signal_info)
                            .build()];
                        self.vulkan_context
                            .device
                            .queue_submit2(
                                self.present_queue,
                                &signal_submit_infos,
                                vk::Fence::null(),
                            )
                            .unwrap();
                    }
                },
            );
            let wait_semaphors = [self.rendering_complete_semaphore];
            let swapchains = [self.swapchain_context.swapchain];
            let image_indices = [present_index];
            let present_info = vk::PresentInfoKHR::builder()
                .wait_semaphores(&wait_semaphors)
                .swapchains(&swapchains)
                .image_indices(&image_indices);
            self.vulkan_context
                .extension
                .swapchain
                .queue_present(self.present_queue, &present_info)
                .unwrap();
            self.current_frame += 1;
        }
    }

    fn record_submit_commandbuffer<F: FnOnce(vk::CommandBuffer)>(
        &self,
        command_buffer: vk::CommandBuffer,
        command_buffer_reuse_fence: vk::Fence,
        submit_queue: vk::Queue,
        wait_mask: &[vk::PipelineStageFlags],
        wait_semaphores: &[vk::Semaphore],
        signal_semaphores: &[vk::Semaphore],
        f: F,
    ) {
        unsafe {
            self.vulkan_context
                .device
                .wait_for_fences(&[command_buffer_reuse_fence], true, std::u64::MAX)
                .expect("fence wait failed!");

            self.vulkan_context
                .device
                .reset_fences(&[command_buffer_reuse_fence])
                .expect("fence reset failed!");

            self.vulkan_context
                .device
                .reset_command_buffer(
                    command_buffer,
                    vk::CommandBufferResetFlags::RELEASE_RESOURCES,
                )
                .expect("reset command buffer failed!");

            let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

            self.vulkan_context
                .device
                .begin_command_buffer(command_buffer, &command_buffer_begin_info)
                .expect("begin commandbuffer failed!");
            f(command_buffer);
            self.vulkan_context
                .device
                .end_command_buffer(command_buffer)
                .expect("end command buffer failed!");

            let command_buffers = vec![command_buffer];

            let submit_info = vk::SubmitInfo::builder()
                .wait_semaphores(wait_semaphores)
                .wait_dst_stage_mask(wait_mask)
                .command_buffers(&command_buffers)
                .signal_semaphores(signal_semaphores);

            self.vulkan_context
                .device
                .queue_submit(
                    submit_queue,
                    &[submit_info.build()],
                    command_buffer_reuse_fence,
                )
                .expect("queue submit failed!");
        }
    }
}

// #[no_mangle]
// pub extern "C" fn add_to_queue(
//     owner: u64,
//     kind: u32,
//     mesh_id: u32,
//     instance_count: u32,
//     vertex_count: u32,
//     base_vertex: u32,
//     indices_offset: u32,
//     primitive: u32,
//     resource_bits: u32,
//     resources: u64,
//     resources_len: u32
// ) {
//     let owner = owner as *const Renderer;
//     let resources = resources as *const u8;
//     let slice = unsafe { std::slice::from_raw_parts(resources, resources_len as usize) };
// }

pub fn make_renderer<F>(instance_extensions: &[*const i8], create_surface: F) -> Renderer
where
    F: FnOnce(&ash::Entry, &ash::Instance, *mut vk::SurfaceKHR) -> vk::Result,
{
    log::trace!("entering make_renderer");

    log::trace!("creating entry...");
    let entry = Entry::linked();
    log::trace!("entry created!");
    log::trace!("creating instance...");
    let instance = make_instance(&entry, instance_extensions);
    log::trace!("instance created!");

    let debug_context = if crate::DEBUG_ENABLED {
        Some(DebugContext::new(&entry, &instance))
    } else {
        None
    };

    let debug_utils_ext = if crate::DEBUG_ENABLED {
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
    let (physical_device, queue_family_index) =
        select_physical_device(&instance, &surface_extension, surface);
    log::trace!("physical device selected!");
    log::trace!("creating device...");
    let device = make_device(&instance, physical_device, queue_family_index);
    log::trace!("device created!");

    let swapchain_extension = ash::extensions::khr::Swapchain::new(&instance, &device);
    let descriptor_buffer_ext = ash::extensions::ext::DescriptorBuffer::new(&instance, &device);

    log::trace!("creating command buffers...");
    let present_queue = unsafe { device.get_device_queue(queue_family_index, 0) };

    let pool_create_info = vk::CommandPoolCreateInfo::builder()
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
        .queue_family_index(queue_family_index);

    let pool = unsafe { device.create_command_pool(&pool_create_info, None).unwrap() };

    let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
        .command_buffer_count(2)
        .command_pool(pool)
        .level(vk::CommandBufferLevel::PRIMARY);

    let command_buffers = unsafe {
        device
            .allocate_command_buffers(&command_buffer_allocate_info)
            .unwrap()
    };
    let setup_command_buffer = command_buffers[0];
    let draw_command_buffer = command_buffers[1];
    log::trace!("command buffers created!");

    log::trace!("creating fences...");
    let fence_create_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
    let draw_commands_reuse_fence = unsafe {
        device
            .create_fence(&fence_create_info, None)
            .expect("Create fence failed.")
    };
    let setup_commands_reuse_fence = unsafe {
        device
            .create_fence(&fence_create_info, None)
            .expect("Create fence failed.")
    };
    log::trace!("fences created!");

    log::trace!("creating semaphores...");
    let semaphore_create_info = vk::SemaphoreCreateInfo::default();
    let present_complete_semaphore = unsafe {
        device
            .create_semaphore(&semaphore_create_info, None)
            .unwrap()
    };
    let rendering_complete_semaphore = unsafe {
        device
            .create_semaphore(&semaphore_create_info, None)
            .unwrap()
    };
    let mut timeline_semaphore_type_create_info = vk::SemaphoreTypeCreateInfo::builder()
        .initial_value(0)
        .semaphore_type(vk::SemaphoreType::TIMELINE)
        .build();
    let timeline_semaphore_create_info = vk::SemaphoreCreateInfo::builder()
        .push_next(&mut timeline_semaphore_type_create_info)
        .build();
    let pass_timeline_semaphore = unsafe {
        device
            .create_semaphore(&timeline_semaphore_create_info, None)
            .unwrap()
    };
    log::trace!("semaphores created!");

    let vulkan_context = VulkanContext {
        entry,
        device,
        instance,
        physical_device,
        extension: ExtensionContext {
            descriptor_buffer: descriptor_buffer_ext,
            debug_utils: debug_utils_ext,
            swapchain: swapchain_extension,
            surface: surface_extension,
        },
    };

    log::trace!("creating allocators...");
    let mut allocator = DeviceAllocator::new_general(&vulkan_context, 16 * 1024);
    let mut desc_allocator = DeviceAllocator::new_descriptor(&vulkan_context, 128 * 1024);
    log::trace!("allocators created!");

    log::trace!("creating swapchain...");
    let swapchain_context = swapchain::SwapchainContext::make(&vulkan_context, surface);
    log::trace!("swapchain created!");

    log::trace!("creating pipeline...");
    let pip = pipeline::file::Pipeline::load(
        &vulkan_context,
        &mut allocator,
        &mut desc_allocator,
        swapchain_context.attachments[0].clone(),
        Some("triangle_pipeline.json"),
    );
    log::trace!("pipeline created!");

    log::trace!("creating test triangle...");
    let test_triangle = make_test_triangle(&mut allocator);
    log::trace!("test triangle created!");
    let mut batches_by_task_type = Vec::with_capacity(TaskKind::MAX_SIZE + 1);
    (0..(TaskKind::MAX_SIZE + 1)).for_each(|_| {
        batches_by_task_type.push(Vec::new());
    });
    log::trace!("finishing renderer...");
    let renderer = Renderer {
        pipeline: pip,
        batches_by_task_type,
        debug_context,
        swapchain_context,
        vulkan_context,
        buffer_allocator: allocator,
        descriptor_allocator: desc_allocator,
        draw_command_buffer,
        present_queue,
        setup_command_buffer,
        rendering_complete_semaphore,
        pass_timeline_semaphore,
        present_complete_semaphore,
        setup_commands_reuse_fence,
        draw_commands_reuse_fence,
        pool,
        test_triangle,
        current_frame: 0,
    };
    log::trace!("renderer finished!");
    return renderer;
}

pub fn make_device(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    queue_family_index: u32,
) -> ash::Device {
    let device_extension_names_raw = [
        khr::Swapchain::name().as_ptr(),
        ext::DescriptorBuffer::name().as_ptr(),
    ];
    let features = vk::PhysicalDeviceFeatures {
        shader_clip_distance: 1,
        ..Default::default()
    };
    let mut dynamic_rendering_feature = vk::PhysicalDeviceDynamicRenderingFeatures {
        dynamic_rendering: 1,
        ..Default::default()
    };
    let mut physical_device_address_feature = vk::PhysicalDeviceBufferDeviceAddressFeatures {
        buffer_device_address: 1,
        buffer_device_address_capture_replay: 1,
        ..Default::default()
    };
    let mut descriptor_buffer_feature = vk::PhysicalDeviceDescriptorBufferFeaturesEXT {
        descriptor_buffer: 1,
        ..Default::default()
    };
    let mut synchronization_2_feature = vk::PhysicalDeviceSynchronization2FeaturesKHR {
        synchronization2: 1,
        ..Default::default()
    };
    let mut features2 = vk::PhysicalDeviceFeatures2::builder()
        .features(features)
        .push_next(&mut dynamic_rendering_feature)
        .push_next(&mut descriptor_buffer_feature)
        .push_next(&mut physical_device_address_feature)
        .push_next(&mut synchronization_2_feature);

    let priorities = [1.0];

    let queue_info = vk::DeviceQueueCreateInfo::builder()
        .queue_family_index(queue_family_index)
        .queue_priorities(&priorities);

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
    return device;
}

pub fn make_instance(entry: &ash::Entry, extensions: &[*const i8]) -> ash::Instance {
    let app_name = unsafe { CStr::from_bytes_with_nul_unchecked(b"rend-vk\0") };
    let validation_layer_name =
        unsafe { CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0") };

    let layers_names_raw = if crate::DEBUG_ENABLED && crate::VALIDATION_LAYER_ENABLED {
        vec![validation_layer_name.as_ptr()]
    } else {
        vec![]
    };
    let mut instance_extensions = extensions.to_vec();
    if crate::DEBUG_ENABLED {
        instance_extensions.push(DebugUtils::name().as_ptr());
    }

    let appinfo = vk::ApplicationInfo::builder()
        .application_name(app_name)
        .application_version(0)
        .engine_name(app_name)
        .engine_version(0)
        .api_version(vk::make_api_version(0, 1, 3, 0));

    let create_info = vk::InstanceCreateInfo::builder()
        .application_info(&appinfo)
        .enabled_layer_names(&layers_names_raw)
        .enabled_extension_names(&instance_extensions);

    log::info!("initializing Instance...");
    let instance: ash::Instance = unsafe {
        entry
            .create_instance(&create_info, None)
            .expect("instance creation error!")
    };
    log::info!("instance initialized!");
    return instance;
}

pub fn select_physical_device(
    instance: &ash::Instance,
    surface_extension: &khr::Surface,
    window_surface: vk::SurfaceKHR,
) -> (vk::PhysicalDevice, u32) {
    let devices = unsafe {
        instance
            .enumerate_physical_devices()
            .expect("Physical device error")
    };
    devices
        .iter()
        .find_map(|pdevice| {
            let properties = unsafe { instance.get_physical_device_properties(*pdevice) };
            let is_discrete = vk::PhysicalDeviceType::DISCRETE_GPU == properties.device_type;
            if !is_discrete {
                return None;
            }
            unsafe {
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
                            Some((*pdevice, index as u32))
                        } else {
                            None
                        }
                    })
            }
        })
        .expect("Couldn't find a suitable physical device!")
}

fn make_test_triangle(buffer_allocator: &mut DeviceAllocator) -> Mesh {
    #[derive(Clone, Debug, Copy)]
    struct Attrib {
        values: [f32; 3],
    }

    let indices = [0u32, 1, 2];
    let vertices = [
        Attrib {
            values: [-1.0, 1.0, 0.0],
        },
        Attrib {
            values: [1.0, 1.0, 0.0],
        },
        Attrib {
            values: [0.0, -1.0, 0.0],
        },
    ];
    let colors = [
        Attrib {
            values: [0.0, 1.0, 1.0],
        },
        Attrib {
            values: [0.0, 0.0, 1.0],
        },
        Attrib {
            values: [1.0, 0.0, 0.0],
        },
    ];
    unsafe {
        let index_buffer = buffer_allocator
            .alloc(std::mem::size_of_val(&indices) as u64)
            .expect("couldn't allocate index buffer");
        let mut index_slice = Align::new(
            index_buffer.addr,
            align_of::<u32>() as u64,
            buffer_allocator.buffer.alignment,
        );
        index_slice.copy_from_slice(&indices);
        let vertex_buffer = buffer_allocator
            .alloc((vertices.len() * std::mem::size_of::<Attrib>()) as u64)
            .expect("couldn't allocate vertex buffer");
        let mut vertex_slice = Align::new(
            vertex_buffer.addr,
            align_of::<u32>() as u64,
            buffer_allocator.buffer.alignment,
        );
        vertex_slice.copy_from_slice(&vertices);

        let color_buffer = buffer_allocator
            .alloc((colors.len() * std::mem::size_of::<Attrib>()) as u64)
            .expect("couldn't allocate vertex buffer");
        let mut color_slice = Align::new(
            color_buffer.addr,
            align_of::<u32>() as u64,
            buffer_allocator.buffer.alignment,
        );
        color_slice.copy_from_slice(&colors);

        Mesh {
            colors: color_buffer,
            vertices: vertex_buffer,
            indices: index_buffer,
            indices_count: indices.len() as u32,
        }
    }
}
