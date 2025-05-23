use ash::vk;

use crate::{context::VulkanContext, format::Format, pipeline::attachment::Attachment};

pub struct SwapchainContext {
    pub surface: vk::SurfaceKHR,
    pub surface_format: vk::SurfaceFormatKHR,
    pub surface_extent: vk::Extent2D,
    pub swapchain: vk::SwapchainKHR,
    pub present_mode: vk::PresentModeKHR,
    pub attachments: Vec<Attachment>,
}

impl SwapchainContext {
    pub fn make(ctx: &VulkanContext, surface: vk::SurfaceKHR, is_vsync_enabled: bool) -> Self {
        let present_mode = present_mode(ctx, surface, is_vsync_enabled);
        let surface_extent = surface_extent(ctx, surface, 1280, 720);
        let surface_format = surface_format(ctx, surface);
        let swapchain = swapchain(ctx, surface, surface_extent, present_mode);
        let swapchain_attachments = attachments(ctx, surface, swapchain, surface_extent);
        ctx.try_set_debug_name("swapchain_main", swapchain);
        Self {
            present_mode,
            surface,
            surface_extent,
            surface_format,
            swapchain,
            attachments: swapchain_attachments,
        }
    }

    pub fn destroy(&self, ctx: &VulkanContext) {
        for att in self.attachments.iter() {
            unsafe {
                ctx.device.destroy_image_view(att.view, None);
            }
        }
        unsafe {
            ctx.extension
                .swapchain
                .destroy_swapchain(self.swapchain, None);
            ctx.extension.surface.destroy_surface(self.surface, None);
        }
    }
}

pub fn attachments(
    ctx: &VulkanContext,
    surface: vk::SurfaceKHR,
    swapchain: vk::SwapchainKHR,
    surface_extent: vk::Extent2D,
) -> Vec<Attachment> {
    let images = unsafe {
        ctx.extension
            .swapchain
            .get_swapchain_images(swapchain)
            .unwrap()
    };
    let surface_format = surface_format(ctx, surface);
    let image_views: Vec<vk::ImageView> = images
        .iter()
        .map(|&image| {
            let create_view_info = vk::ImageViewCreateInfo::builder()
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(surface_format.format)
                .components(vk::ComponentMapping {
                    r: vk::ComponentSwizzle::R,
                    g: vk::ComponentSwizzle::G,
                    b: vk::ComponentSwizzle::B,
                    a: vk::ComponentSwizzle::A,
                })
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .image(image)
                .build();
            unsafe {
                ctx.device
                    .create_image_view(&create_view_info, None)
                    .unwrap()
            }
        })
        .collect();
    images
        .iter()
        .zip(image_views.iter())
        .enumerate()
        .for_each(|(i, (img, view))| {
            ctx.try_set_debug_name(&format!("swapchain_{}_image", i), *img);
            ctx.try_set_debug_name(&format!("swapchain_{}_image_view", i), *view);
        });
    let attachments: Vec<Attachment> = images
        .into_iter()
        .zip(image_views)
        .map(|e| {
            let (image, view) = e;
            Attachment::default_attachment_of(surface_format.format, image, view, surface_extent)
        })
        .collect();

    attachments
}

pub fn swapchain(
    ctx: &VulkanContext,
    surface: vk::SurfaceKHR,
    surface_extent: vk::Extent2D,
    present_mode: vk::PresentModeKHR,
) -> vk::SwapchainKHR {
    let surface_format = surface_format(ctx, surface);
    let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
        .surface(surface)
        .min_image_count(desired_image_count(ctx, surface))
        .image_color_space(surface_format.color_space)
        .image_format(surface_format.format)
        .image_extent(surface_extent)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(present_mode)
        .clipped(true)
        .image_array_layers(1);

    unsafe {
        ctx.extension
            .swapchain
            .create_swapchain(&swapchain_create_info, None)
            .unwrap()
    }
}

pub fn surface_extent(
    ctx: &VulkanContext,
    surface: vk::SurfaceKHR,
    width: u32,
    height: u32,
) -> vk::Extent2D {
    let surface_caps = surface_capabilities(ctx, surface);
    match surface_caps.current_extent.width {
        std::u32::MAX => vk::Extent2D { width, height },
        _ => surface_caps.current_extent,
    }
}

pub fn surface_format(ctx: &VulkanContext, surface: vk::SurfaceKHR) -> vk::SurfaceFormatKHR {
    let formats = unsafe {
        ctx.extension
            .surface
            .get_physical_device_surface_formats(ctx.physical_device, surface)
            .unwrap()
    };
    // Try finding the first SRGB format available
    let srgb = formats
        .iter()
        .find(|e| Format::of_u32(e.format.as_raw() as u32).is_srgb());
    if let Some(fmt) = srgb {
        *fmt
    } else {
        *formats
            .first()
            .expect("couldn't list the device's surface formats!")
    }
}

pub fn present_mode(
    ctx: &VulkanContext,
    surface: vk::SurfaceKHR,
    is_vsync_enabled: bool,
) -> vk::PresentModeKHR {
    let present_modes = unsafe {
        ctx.extension
            .surface
            .get_physical_device_surface_present_modes(ctx.physical_device, surface)
            .unwrap()
    };
    present_modes
        .iter()
        .cloned()
        // if vsync is enabled, prefer dynamic vsync
        .find(|&mode| is_vsync_enabled && mode == vk::PresentModeKHR::FIFO_RELAXED)
        .unwrap_or(if is_vsync_enabled {
            // otherwise default to hard vsync
            vk::PresentModeKHR::FIFO
        } else {
            // no vsync at all
            vk::PresentModeKHR::IMMEDIATE
        })
}

pub fn desired_image_count(ctx: &VulkanContext, surface: vk::SurfaceKHR) -> u32 {
    let surface_caps = surface_capabilities(ctx, surface);
    let desired_image_count = surface_caps.min_image_count + 1;
    if surface_caps.max_image_count > 0 && desired_image_count > surface_caps.max_image_count {
        return surface_caps.max_image_count;
    }
    desired_image_count
}

pub fn surface_capabilities(
    ctx: &VulkanContext,
    surface: vk::SurfaceKHR,
) -> vk::SurfaceCapabilitiesKHR {
    unsafe {
        ctx.extension
            .surface
            .get_physical_device_surface_capabilities(ctx.physical_device, surface)
            .unwrap()
    }
}
