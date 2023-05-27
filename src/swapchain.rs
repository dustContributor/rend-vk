use ash::vk;

use crate::{context::VulkanContext, pipeline::attachment::Attachment};

pub struct SwapchainContext {
    pub surface: vk::SurfaceKHR,
    pub surface_format: vk::SurfaceFormatKHR,
    pub surface_extent: vk::Extent2D,
    pub swapchain: vk::SwapchainKHR,
    pub present_mode: vk::PresentModeKHR,
    pub attachments: Vec<Attachment>,
}

impl SwapchainContext {
    pub fn make(vulkan_context: &VulkanContext, surface: vk::SurfaceKHR) -> Self {
        let present_mode = present_mode(&vulkan_context, surface);
        let surface_extent = surface_extent(&vulkan_context, surface, 0, 0);
        let surface_format = surface_format(&vulkan_context, surface);
        let swapchain = swapchain(&vulkan_context, surface, surface_extent);
        let swapchain_attachments =
            attachments(&vulkan_context, surface, swapchain, surface_extent);
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
                    ..Default::default()
                })
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                    ..Default::default()
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
) -> vk::SwapchainKHR {
    let surface_format = surface_format(ctx, surface);
    let present_mode = present_mode(ctx, surface);
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
    unsafe {
        ctx.extension
            .surface
            .get_physical_device_surface_formats(ctx.physical_device, surface)
            .unwrap()[0]
    }
}

pub fn present_mode(ctx: &VulkanContext, surface: vk::SurfaceKHR) -> vk::PresentModeKHR {
    let present_modes = unsafe {
        ctx.extension
            .surface
            .get_physical_device_surface_present_modes(ctx.physical_device, surface)
            .unwrap()
    };
    present_modes
        .iter()
        .cloned()
        .find(|&mode| mode == vk::PresentModeKHR::FIFO_RELAXED)
        .unwrap_or(vk::PresentModeKHR::FIFO)
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
