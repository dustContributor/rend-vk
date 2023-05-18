use std::{any::TypeId, collections::HashMap};

use ash::vk;

use crate::DEBUG_ENABLED;

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

impl ExtensionContext {
    pub fn try_set_debug_name<T: 'static>(
        &self,
        device: &ash::Device,
        name: &String,
        obj: T,
    ) -> bool
    where
        T: vk::Handle,
    {
        if !DEBUG_ENABLED {
            return false;
        }
        if !self.debug_utils.is_some() {
            return false;
        }
        let dbg = self.debug_utils.as_ref().unwrap();
        let c_name = std::ffi::CString::new(name.clone()).unwrap();
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
