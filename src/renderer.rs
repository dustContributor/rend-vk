use std::{alloc::Layout, ffi::CStr};

use ash::{
    extensions::{
        ext::{self, DebugUtils},
        khr,
    },
    vk, Entry,
};

use crate::{
    buffer::DeviceAllocator,
    context::{self, ExtensionContext, VulkanContext},
    debug::{self, DebugContext},
    pipeline::{self, Pipeline},
    // render_task::{RenderTask},
    swapchain,
};

pub struct Renderer {
    pub pipeline: Pipeline,
    // pub batches_by_task_type: Vec<Vec<RenderTask>>,
    pub swapchain_context: swapchain::SwapchainContext,
    pub vulkan_context: context::VulkanContext,
    pub debug_context: Option<debug::DebugContext>,
}

impl Renderer {
    // pub fn add_to_queue(&self, task: RenderTask) {}
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
// test.Testing.make_renderer
#[no_mangle]
pub extern "C" fn Java_test_Testing_init(_unused_jnienv: usize, _unused_jclazz: usize) {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    log_panics::init();
}

#[no_mangle]
pub extern "C" fn Java_test_Testing_make_1renderer(
    _unused_jnienv: usize,
    _unused_jclazz: usize,
    window: u64,
    instance_extensions: u64,
    instance_extensions_len: u64,
    glfw_create_window_surface: u64,
) -> u64 {
    log::trace!("entering make_renderer");
    /*
     * VkResult glfwCreateWindowSurface (
     * VkInstance instance,
     * GLFWwindow *window,
     * const VkAllocationCallbacks *allocator,
     * VkSurfaceKHR *surface)
     */
    let glfw_create_window_surface = unsafe {
        std::mem::transmute::<
            _,
            extern "C" fn(vk::Instance, u64, u64, *mut vk::SurfaceKHR) -> vk::Result,
        >(glfw_create_window_surface as *const ())
    };
    log::trace!("creating entry...");
    let entry = Entry::linked();
    log::trace!("entry created!");
    let instance_extensions = unsafe {
        if instance_extensions_len == 0 {
            &[]
        } else {
            std::slice::from_raw_parts(
                instance_extensions as *const *const i8,
                instance_extensions_len as usize,
            )
        }
    };
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
    let create_surface_result = glfw_create_window_surface(instance.handle(), window, 0, surface);
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
    let present_mode = swapchain::present_mode(&vulkan_context, surface);
    let surface_extent = swapchain::surface_extent(&vulkan_context, surface, 0, 0);
    let surface_format = swapchain::surface_format(&vulkan_context, surface);
    let swapchain = swapchain::swapchain(&vulkan_context, surface, surface_extent);
    let swapchain_attachments =
        swapchain::attachments(&vulkan_context, surface, swapchain, surface_extent);
    log::trace!("swapchain created!");

    let swapchain_context = swapchain::SwapchainContext {
        present_mode,
        surface,
        surface_extent,
        surface_format,
        swapchain,
        attachments: swapchain_attachments,
    };

    log::trace!("creating pipeline...");
    let pip = pipeline::file::Pipeline::load(
        &vulkan_context,
        &mut allocator,
        &mut desc_allocator,
        swapchain_context.attachments[0].clone(),
        Some("triangle_pipeline.json"),
    );
    log::trace!("pipeline created!");

    log::trace!("finishing renderer...");
    let renderer = Renderer {
        pipeline: pip,
        // batches_by_task_type: Vec::new(),
        debug_context,
        swapchain_context,
        vulkan_context,
    };
    let boxed = Box::from(renderer);
    let ptr = Box::into_raw(boxed) as u64;
    log::trace!("renderer finished!");
    return ptr;
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

    log::info!("Initializing Device...");
    let device: ash::Device = unsafe {
        instance
            .create_device(physical_device, &device_create_info, None)
            .expect("Couldn't create the device!")
    };
    log::info!("Device initialized!");
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

    log::info!("Initializing Instance...");
    let instance: ash::Instance = unsafe {
        entry
            .create_instance(&create_info, None)
            .expect("Instance creation error")
    };
    log::info!("Instance initialized!");
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
