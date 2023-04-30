use ash::vk;

#[derive(Clone)]
pub struct Attachment {
    pub name: String,
    pub memory: vk::DeviceMemory,
    pub format: crate::format::Format,
    // Keep the equivalent vulkan value for convenience.
    pub vk_format: vk::Format,
    pub image: vk::Image,
    pub view: vk::ImageView,
    pub extent: vk::Extent2D,
}

impl Attachment {
    pub const DEFAULT_NAME: &'static str = "default";
    pub const DEPTH_NAME: &'static str = "depth";

    pub fn default_attachment_of(
        vk_format: vk::Format,
        image: vk::Image,
        image_view: vk::ImageView,
        extent: vk::Extent2D,
    ) -> Attachment {
        Attachment {
            format: crate::format::Format::UNDEFINED,
            vk_format,
            image,
            memory: vk::DeviceMemory::null(),
            name: Attachment::DEFAULT_NAME.to_string(),
            view: image_view,
            extent,
        }
    }

    pub fn is_default(&self) -> bool {
        self.name == Attachment::DEFAULT_NAME
    }
}
