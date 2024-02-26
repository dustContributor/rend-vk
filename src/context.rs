use std::{any::TypeId, collections::HashMap};

use ash::vk;

#[derive(Clone)]
pub struct VulkanContext {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub device: ash::Device,
    pub physical_device: ash::vk::PhysicalDevice,
    pub memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub extension: ExtensionContext,
}

#[derive(Clone)]
pub struct ExtensionContext {
    pub descriptor_buffer: ash::extensions::ext::DescriptorBuffer,
    pub debug_utils: Option<ash::extensions::ext::DebugUtils>,
    pub swapchain: ash::extensions::khr::Swapchain,
    pub surface: ash::extensions::khr::Surface,
}

impl VulkanContext {
    pub fn try_begin_debug_label(&self, command_buffer: vk::CommandBuffer, name: &str) -> bool {
        self.extension.try_begin_debug_label(command_buffer, name)
    }

    pub fn try_end_debug_label(&self, command_buffer: vk::CommandBuffer) -> bool {
        self.extension.try_end_debug_label(command_buffer)
    }

    pub fn try_set_debug_name<T: 'static>(&self, name: &str, obj: T) -> bool
    where
        T: vk::Handle,
    {
        self.extension.try_set_debug_name(&self.device, name, obj)
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
    pub fn try_begin_debug_label(&self, command_buffer: vk::CommandBuffer, name: &str) -> bool {
        if !self.debug_utils.is_some() {
            // Assume no debug utils means debug isn't enabled
            return false;
        }
        let dbg = self.debug_utils.as_ref().unwrap();
        let c_name = std::ffi::CString::new(name).unwrap();
        let label = vk::DebugUtilsLabelEXT::builder()
            .label_name(&c_name)
            .build();
        unsafe {
            dbg.cmd_begin_debug_utils_label(command_buffer, &label);
        }
        true
    }

    pub fn try_end_debug_label(&self, command_buffer: vk::CommandBuffer) -> bool {
        if !self.debug_utils.is_some() {
            // Assume no debug utils means debug isn't enabled
            return false;
        }
        let dbg = self.debug_utils.as_ref().unwrap();
        unsafe {
            dbg.cmd_end_debug_utils_label(command_buffer);
        }
        true
    }

    pub fn try_set_debug_name<T: 'static>(&self, device: &ash::Device, name: &str, obj: T) -> bool
    where
        T: vk::Handle,
    {
        if !self.debug_utils.is_some() {
            // Assume no debug utils means debug isn't enabled
            return false;
        }
        let dbg = self.debug_utils.as_ref().unwrap();
        let c_name = std::ffi::CString::new(name).unwrap();
        let type_id = TypeId::of::<T>();
        let object_type = OBJECT_TYPES_BY_TYPE_ID.get(&type_id).unwrap().clone();
        let name_info = vk::DebugUtilsObjectNameInfoEXT::builder()
            .object_type(object_type)
            .object_handle(vk::Handle::as_raw(obj))
            .object_name(&c_name)
            .build();
        unsafe {
            dbg.set_debug_utils_object_name(device.handle(), &name_info)
                .unwrap();
        }
        true
    }
}

lazy_static! {
    static ref OBJECT_TYPES_BY_TYPE_ID: HashMap<TypeId, vk::ObjectType> = {
        [
            (TypeId::of::<vk::Instance>(), vk::ObjectType::INSTANCE),
            (
                TypeId::of::<vk::PhysicalDevice>(),
                vk::ObjectType::PHYSICAL_DEVICE,
            ),
            (TypeId::of::<vk::Device>(), vk::ObjectType::DEVICE),
            (TypeId::of::<vk::Queue>(), vk::ObjectType::QUEUE),
            (TypeId::of::<vk::Semaphore>(), vk::ObjectType::SEMAPHORE),
            (
                TypeId::of::<vk::CommandBuffer>(),
                vk::ObjectType::COMMAND_BUFFER,
            ),
            (TypeId::of::<vk::Fence>(), vk::ObjectType::FENCE),
            (
                TypeId::of::<vk::DeviceMemory>(),
                vk::ObjectType::DEVICE_MEMORY,
            ),
            (TypeId::of::<vk::Buffer>(), vk::ObjectType::BUFFER),
            (TypeId::of::<vk::Image>(), vk::ObjectType::IMAGE),
            (TypeId::of::<vk::Event>(), vk::ObjectType::EVENT),
            (TypeId::of::<vk::QueryPool>(), vk::ObjectType::QUERY_POOL),
            (TypeId::of::<vk::BufferView>(), vk::ObjectType::BUFFER_VIEW),
            (TypeId::of::<vk::ImageView>(), vk::ObjectType::IMAGE_VIEW),
            (
                TypeId::of::<vk::ShaderModule>(),
                vk::ObjectType::SHADER_MODULE,
            ),
            (
                TypeId::of::<vk::PipelineCache>(),
                vk::ObjectType::PIPELINE_CACHE,
            ),
            (
                TypeId::of::<vk::PipelineLayout>(),
                vk::ObjectType::PIPELINE_LAYOUT,
            ),
            (TypeId::of::<vk::RenderPass>(), vk::ObjectType::RENDER_PASS),
            (TypeId::of::<vk::Pipeline>(), vk::ObjectType::PIPELINE),
            (
                TypeId::of::<vk::DescriptorSetLayout>(),
                vk::ObjectType::DESCRIPTOR_SET_LAYOUT,
            ),
            (TypeId::of::<vk::Sampler>(), vk::ObjectType::SAMPLER),
            (
                TypeId::of::<vk::DescriptorPool>(),
                vk::ObjectType::DESCRIPTOR_POOL,
            ),
            (
                TypeId::of::<vk::DescriptorSet>(),
                vk::ObjectType::DESCRIPTOR_SET,
            ),
            (TypeId::of::<vk::Framebuffer>(), vk::ObjectType::FRAMEBUFFER),
            (
                TypeId::of::<vk::CommandPool>(),
                vk::ObjectType::COMMAND_POOL,
            ),
            (
                TypeId::of::<vk::SamplerYcbcrConversion>(),
                vk::ObjectType::SAMPLER_YCBCR_CONVERSION,
            ),
            (
                TypeId::of::<vk::DescriptorUpdateTemplate>(),
                vk::ObjectType::DESCRIPTOR_UPDATE_TEMPLATE,
            ),
            (TypeId::of::<vk::SurfaceKHR>(), vk::ObjectType::SURFACE_KHR),
            (
                TypeId::of::<vk::SwapchainKHR>(),
                vk::ObjectType::SWAPCHAIN_KHR,
            ),
            (TypeId::of::<vk::DisplayKHR>(), vk::ObjectType::DISPLAY_KHR),
            (
                TypeId::of::<vk::DisplayModeKHR>(),
                vk::ObjectType::DISPLAY_MODE_KHR,
            ),
            (
                TypeId::of::<vk::DebugReportCallbackEXT>(),
                vk::ObjectType::DEBUG_REPORT_CALLBACK_EXT,
            ),
            (
                TypeId::of::<vk::IndirectCommandsLayoutNV>(),
                vk::ObjectType::INDIRECT_COMMANDS_LAYOUT_NV,
            ),
            (
                TypeId::of::<vk::DebugUtilsMessengerEXT>(),
                vk::ObjectType::DEBUG_UTILS_MESSENGER_EXT,
            ),
            (
                TypeId::of::<vk::ValidationCacheEXT>(),
                vk::ObjectType::VALIDATION_CACHE_EXT,
            ),
            (
                TypeId::of::<vk::AccelerationStructureNV>(),
                vk::ObjectType::ACCELERATION_STRUCTURE_NV,
            ),
            (
                TypeId::of::<vk::AccelerationStructureKHR>(),
                vk::ObjectType::ACCELERATION_STRUCTURE_KHR,
            ),
            (
                TypeId::of::<vk::PerformanceConfigurationINTEL>(),
                vk::ObjectType::PERFORMANCE_CONFIGURATION_INTEL,
            ),
            (
                TypeId::of::<vk::DeferredOperationKHR>(),
                vk::ObjectType::DEFERRED_OPERATION_KHR,
            ),
            (
                TypeId::of::<vk::PrivateDataSlot>(),
                vk::ObjectType::PRIVATE_DATA_SLOT,
            ),
            (
                TypeId::of::<vk::OpticalFlowSessionNV>(),
                vk::ObjectType::OPTICAL_FLOW_SESSION_NV,
            ),
        ]
        .iter()
        .map(|e| e.clone())
        .collect()
    };
}
