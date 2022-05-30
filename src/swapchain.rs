use crate::window::WindowContext;
use ash::extensions::khr::{Surface, Swapchain};
use ash::vk;
use ash::{Device, Entry, Instance};

pub struct SwapchainContext {
    pub surface_loader: Surface,
    swapchain_loader: Option<Swapchain>,

    pub surface: vk::SurfaceKHR,
    pub surface_format: vk::SurfaceFormatKHR,
    pub surface_resolution: vk::Extent2D,
    pub surface_capabilities: vk::SurfaceCapabilitiesKHR,
    pub surface_transform: vk::SurfaceTransformFlagsKHR,

    pub swapchain: vk::SwapchainKHR,
    pub present_mode: vk::PresentModeKHR,
    pub present_images: Vec<vk::Image>,
    pub present_image_views: Vec<vk::ImageView>,
}
// TODO: Not good
impl SwapchainContext {
    pub fn new(window_context: &WindowContext, entry: &Entry, instance: &Instance) -> Self {
        let surface_loader = Surface::new(&entry, &instance);
        let surface =
            unsafe { ash_window::create_surface(&entry, &instance, &window_context.window, None) }
                .unwrap();

        return SwapchainContext {
            surface_loader,
            swapchain_loader: None,
            surface,
            surface_format: Default::default(),
            surface_capabilities: Default::default(),
            surface_resolution: Default::default(),
            surface_transform: Default::default(),
            swapchain: Default::default(),
            present_mode: Default::default(),
            present_images: Default::default(),
            present_image_views: Default::default(),
        };
    }

    pub fn init(
        &mut self,
        pdevice: &vk::PhysicalDevice,
        instance: &Instance,
        device: &Device,
        width_height: (u32, u32),
    ) {
        self.init_format(&pdevice);
        self.init_swapchain_loader(&instance, &device);
        self.init_surface(&pdevice, width_height.0, width_height.1);
        self.init_present_mode(&pdevice);
        self.init_swapchain_loader(&instance, &device);
        self.init_swapchain();
        self.init_present_images(&device);
    }

    fn init_swapchain_loader(&mut self, instance: &Instance, device: &Device) {
        self.swapchain_loader = Some(Swapchain::new(&instance, device));
    }

    fn init_format(&mut self, device: &vk::PhysicalDevice) {
        self.surface_format = unsafe {
            self.surface_loader
                .get_physical_device_surface_formats(*device, self.surface)
                .unwrap()[0]
        };
    }
    fn init_surface(&mut self, device: &vk::PhysicalDevice, width: u32, height: u32) {
        self.surface_format = unsafe {
            self.surface_loader
                .get_physical_device_surface_formats(*device, self.surface)
                .unwrap()[0]
        };

        self.surface_capabilities = unsafe {
            self.surface_loader
                .get_physical_device_surface_capabilities(*device, self.surface)
                .unwrap()
        };

        self.surface_resolution = match self.surface_capabilities.current_extent.width {
            std::u32::MAX => vk::Extent2D { width, height },
            _ => self.surface_capabilities.current_extent,
        };
        self.surface_transform = if self
            .surface_capabilities
            .supported_transforms
            .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            self.surface_capabilities.current_transform
        };
    }

    pub fn desired_image_count(&self) -> u32 {
        let desired_image_count = self.surface_capabilities.min_image_count + 1;
        if self.surface_capabilities.max_image_count > 0
            && desired_image_count > self.surface_capabilities.max_image_count
        {
            return self.surface_capabilities.max_image_count;
        }
        desired_image_count
    }

    pub fn swapchain_loader(&self) -> &Swapchain {
        self.swapchain_loader.as_ref().unwrap()
    }

    fn init_present_mode(&mut self, device: &vk::PhysicalDevice) {
        let present_modes = unsafe {
            self.surface_loader
                .get_physical_device_surface_present_modes(*device, self.surface)
                .unwrap()
        };
        self.present_mode = present_modes
            .iter()
            .cloned()
            .find(|&mode| mode == vk::PresentModeKHR::FIFO_RELAXED)
            .unwrap_or(vk::PresentModeKHR::FIFO);
    }

    fn init_swapchain(&mut self) {
        let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(self.surface)
            .min_image_count(self.desired_image_count())
            .image_color_space(self.surface_format.color_space)
            .image_format(self.surface_format.format)
            .image_extent(self.surface_resolution)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(self.surface_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(self.present_mode)
            .clipped(true)
            .image_array_layers(1);

        self.swapchain = unsafe {
            self.swapchain_loader()
                .create_swapchain(&swapchain_create_info, None)
                .unwrap()
        };
    }

    fn init_present_images(&mut self, device: &Device) {
        self.present_images = unsafe {
            self.swapchain_loader()
                .get_swapchain_images(self.swapchain)
                .unwrap()
        };
        self.present_image_views = self
            .present_images
            .iter()
            .map(|&image| {
                let create_view_info = vk::ImageViewCreateInfo::builder()
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(self.surface_format.format)
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
                    .image(image);
                unsafe { device.create_image_view(&create_view_info, None).unwrap() }
            })
            .collect();
    }

    pub fn destroy(&self, device: &Device) {
        for &image_view in self.present_image_views.iter() {
            unsafe {
                device.destroy_image_view(image_view, None);
            }
        }
        unsafe {
            self.swapchain_loader()
                .destroy_swapchain(self.swapchain, None);
            self.surface_loader.destroy_surface(self.surface, None);
        }
    }
}
