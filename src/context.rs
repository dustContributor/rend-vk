use ash::vk;

pub struct VulkanContext {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub device: ash::Device,
    pub physical_device: ash::vk::PhysicalDevice,
    pub memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub extension: ExtensionContext,
}

pub struct ExtensionContext {
    pub debug_utils: Option<ash::ext::debug_utils::Device>,
    pub swapchain: ash::khr::swapchain::Device,
    pub surface: ash::khr::surface::Instance,
}

impl VulkanContext {
    pub fn is_debug_enabled(&self) -> bool {
        self.extension.is_debug_enabled()
    }

    pub fn try_begin_debug_label(&self, command_buffer: vk::CommandBuffer, name: &str) -> bool {
        self.extension.try_begin_debug_label(command_buffer, name)
    }

    pub fn try_end_debug_label(&self, command_buffer: vk::CommandBuffer) -> bool {
        self.extension.try_end_debug_label(command_buffer)
    }

    pub fn try_set_debug_name<T>(&self, name: &str, obj: T) -> bool
    where
        T: 'static + vk::Handle,
    {
        self.extension.try_set_debug_name(name, obj)
    }

    pub fn memory_type_index_for(
        &self,
        requirement_bits: u32,
        property_flags: vk::MemoryPropertyFlags,
    ) -> Option<u32> {
        let count = self.memory_properties.memory_type_count;
        self.memory_properties.memory_types[..count as _]
            .iter()
            .enumerate()
            .find(|(index, memory_type)| {
                (1 << index) & requirement_bits != 0
                    && memory_type.property_flags & property_flags == property_flags
            })
            .map(|(index, _memory_type)| index as _)
    }
}

impl ExtensionContext {
    pub fn is_debug_enabled(&self) -> bool {
        // Assume no debug utils means debug isn't enabled
        self.debug_utils.is_some()
    }
    pub fn try_begin_debug_label(&self, command_buffer: vk::CommandBuffer, name: &str) -> bool {
        if !self.is_debug_enabled() {
            return false;
        }
        let dbg = self.debug_utils.as_ref().unwrap();
        let c_name = std::ffi::CString::new(name).unwrap();
        let label = vk::DebugUtilsLabelEXT::default().label_name(&c_name);
        unsafe {
            dbg.cmd_begin_debug_utils_label(command_buffer, &label);
        }
        true
    }

    pub fn try_end_debug_label(&self, command_buffer: vk::CommandBuffer) -> bool {
        if !self.is_debug_enabled() {
            // Assume no debug utils means debug isn't enabled
            return false;
        }
        let dbg = self.debug_utils.as_ref().unwrap();
        unsafe {
            dbg.cmd_end_debug_utils_label(command_buffer);
        }
        true
    }

    pub fn try_set_debug_name<T>(&self, name: &str, obj: T) -> bool
    where
        T: 'static + vk::Handle,
    {
        if !self.is_debug_enabled() {
            return false;
        }
        let dbg = self.debug_utils.as_ref().unwrap();
        let c_name = std::ffi::CString::new(name).unwrap();
        let name_info = vk::DebugUtilsObjectNameInfoEXT::default()
            .object_handle(obj)
            .object_name(&c_name);
        unsafe {
            dbg.set_debug_utils_object_name(&name_info).unwrap();
        }
        true
    }
}
