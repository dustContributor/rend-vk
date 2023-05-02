use crate::buffer::{BufferKind, GpuAllocator, GpuSlice};
use ash::vk;

pub struct DescriptorBuffer {
    name: String,
    device: GpuSlice,
    local: Box<[u8]>,
    layout: vk::DescriptorSetLayout,
    descriptor_type: vk::DescriptorType,
    descriptor_size: usize,
}

impl DescriptorBuffer {
    pub fn of(
        instance: &ash::Instance,
        physical_device: &vk::PhysicalDevice,
        device: &ash::Device,
        desc_buffer_instance: &ash::extensions::ext::DescriptorBuffer,
        mem: &mut GpuAllocator,
        name: String,
        descriptor_type: vk::DescriptorType,
    ) -> Self {
        if BufferKind::DESCRIPTOR != mem.kind {
            panic!(
                "Allocator with kind {} passed, kind {} needed!",
                mem.kind,
                BufferKind::DESCRIPTOR
            )
        }
        let mut props = vk::PhysicalDeviceDescriptorBufferPropertiesEXT {
            ..Default::default()
        };
        let mut device_props = vk::PhysicalDeviceProperties2::builder()
            .push_next(&mut props)
            .build();
        unsafe { instance.get_physical_device_properties2(*physical_device, &mut device_props) };
        let descriptor_size = Self::size_for(descriptor_type, &props);
        let binding = vk::DescriptorSetLayoutBinding {
            descriptor_type,
            descriptor_count: 256,
            stage_flags: vk::ShaderStageFlags::ALL_GRAPHICS,
            ..Default::default()
        };
        let bindings = [binding];
        let info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&bindings)
            .flags(vk::DescriptorSetLayoutCreateFlags::DESCRIPTOR_BUFFER_EXT)
            .build();
        let layout = unsafe { device.create_descriptor_set_layout(&info, None) }.unwrap();
        let layout_size = unsafe { desc_buffer_instance.get_descriptor_set_layout_size(layout) };
        let boxed: Box<[u8]> = vec![0; layout_size as usize].into_boxed_slice();
        let buffer = mem.alloc(layout_size).unwrap();
        Self {
            name,
            layout,
            device: buffer,
            local: boxed,
            descriptor_type,
            descriptor_size,
        }
    }

    fn size_for(
        descriptor_type: vk::DescriptorType,
        props: &vk::PhysicalDeviceDescriptorBufferPropertiesEXT,
    ) -> usize {
        match descriptor_type {
            vk::DescriptorType::SAMPLED_IMAGE => props.sampled_image_descriptor_size,
            vk::DescriptorType::UNIFORM_BUFFER => props.uniform_buffer_descriptor_size,
            vk::DescriptorType::SAMPLER => props.sampler_descriptor_size,
            _ => panic!("Unsupported descriptor type {:?}", descriptor_type),
        }
    }
}
