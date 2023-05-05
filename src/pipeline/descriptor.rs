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
        count: u32,
    ) -> Self {
        assert!(count > 0, "Cant have zero sized descriptor buffers!");
        assert!(
            BufferKind::DESCRIPTOR == mem.kind,
            "Allocator with kind {} passed, kind {} needed!",
            mem.kind,
            BufferKind::DESCRIPTOR
        );
        let descriptor_size = Self::size_of(descriptor_type, instance, physical_device);
        let binding = vk::DescriptorSetLayoutBinding {
            descriptor_type,
            descriptor_count: count,
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

    fn size_of(
        descriptor_type: vk::DescriptorType,
        instance: &ash::Instance,
        physical_device: &vk::PhysicalDevice,
    ) -> usize {
        let mut props = vk::PhysicalDeviceDescriptorBufferPropertiesEXT {
            ..Default::default()
        };
        let mut device_props = vk::PhysicalDeviceProperties2::builder()
            .push_next(&mut props)
            .build();
        unsafe { instance.get_physical_device_properties2(*physical_device, &mut device_props) };
        match descriptor_type {
            vk::DescriptorType::SAMPLED_IMAGE => props.sampled_image_descriptor_size,
            vk::DescriptorType::UNIFORM_BUFFER => props.uniform_buffer_descriptor_size,
            vk::DescriptorType::SAMPLER => props.sampler_descriptor_size,
            _ => panic!("Unsupported descriptor type {:?}", descriptor_type),
        }
    }

    pub fn place_at(&mut self, index: u32, data: &[u8]) {
        let offset = index as usize * self.descriptor_size;
        self.local[offset..(offset + self.descriptor_size)].copy_from_slice(data);
    }

    pub fn into_device(&mut self) {
        let mut device_buffer_slice = unsafe {
            ash::util::Align::new(
                self.device.addr,
                self.descriptor_size as u64,
                self.local.len() as u64,
            )
        };
        device_buffer_slice.copy_from_slice(&self.local);
    }
}
