#[derive(Clone)]
pub struct VulkanContext {
    pub instance: ash::Instance,
    pub device: ash::Device,
    pub physical_device: ash::vk::PhysicalDevice,
    pub extensions: ExtensionContext,
}

#[derive(Clone)]
pub struct ExtensionContext {
    pub descriptor_buffer: ash::extensions::ext::DescriptorBuffer,
    pub debug_utils: Option<ash::extensions::ext::DebugUtils>,
}
