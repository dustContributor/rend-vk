use ash::vk;
use std::cell::RefCell;
use std::clone::Clone;
use std::marker::Copy;
use std::os::raw::c_void;
use std::rc::Rc;

use crate::context::VulkanContext;

#[derive(Clone)]
pub struct DeviceAllocator {
    inner: Rc<RefCell<InnerDeviceAllocator>>,
    pub buffer: DeviceBuffer,
}

#[derive(Copy, Clone)]
pub struct DeviceSlice {
    pub buffer: vk::Buffer,
    pub size: u64,
    pub offset: u64,
    pub alignment: u64,
    pub addr: *mut c_void,
    pub device_addr: u64,
    pub kind: BufferKind,
}

impl DeviceSlice {
    pub fn empty() -> Self {
        Self {
            buffer: vk::Buffer::null(),
            size: 0,
            offset: 0,
            alignment: 0,
            addr: std::ptr::null_mut(),
            device_addr: 0,
            kind: BufferKind::Undefined,
        }
    }

    pub fn read(&self) -> Vec<u8> {
        let slice =
            unsafe { std::slice::from_raw_parts(self.addr as *const u8, self.size as usize) };
        slice.to_vec()
    }
}

impl DeviceAllocator {
    pub fn new_general(ctx: &VulkanContext, size: u64) -> Self {
        Self::new(ctx, size, BufferKind::General)
    }

    pub fn new_descriptor(ctx: &VulkanContext, size: u64) -> Self {
        Self::new(ctx, size, BufferKind::Descriptor)
    }

    pub fn new(ctx: &VulkanContext, size: u64, kind: BufferKind) -> Self {
        let inner = InnerDeviceAllocator::new(ctx, size, kind);
        let buffer = inner.buffer.clone();
        let refc = Rc::new(RefCell::new(inner));
        Self {
            buffer,
            inner: refc,
        }
    }

    pub fn alloc(&self, size: u64) -> Option<DeviceSlice> {
        self.inner.borrow_mut().alloc(size)
    }

    pub fn free(&self, slice: DeviceSlice) {
        self.inner.borrow_mut().free(slice)
    }

    pub fn destroy(&self, device: &ash::Device) {
        self.inner.borrow().destroy(device)
    }

    pub fn available(&self) -> u64 {
        self.inner.borrow().available()
    }

    pub fn alignment(&self) -> u64 {
        self.inner.borrow().buffer.alignment
    }

    pub fn size(&self) -> u64 {
        self.inner.borrow().buffer.size
    }

    pub fn kind(&self) -> BufferKind {
        self.inner.borrow().buffer.kind
    }

    ///
    /// Just go to town with it if you want
    ///
    pub fn buffer(&self) -> DeviceBuffer {
        self.inner.borrow().buffer.clone()
    }
}

#[derive(Copy, Clone, PartialEq, strum_macros::Display)]
pub enum BufferKind {
    Undefined,
    General,
    Descriptor,
}

impl BufferKind {
    pub fn to_vk_usage_flags(&self) -> vk::BufferUsageFlags {
        use vk::BufferUsageFlags as Buf;
        match self {
            BufferKind::General => {
                Buf::SHADER_DEVICE_ADDRESS
                    | Buf::VERTEX_BUFFER
                    | Buf::INDEX_BUFFER
                    | Buf::STORAGE_BUFFER
                    | Buf::UNIFORM_BUFFER
                    | Buf::TRANSFER_SRC
                    | Buf::TRANSFER_DST
            }
            BufferKind::Descriptor => {
                Buf::SHADER_DEVICE_ADDRESS
                    | Buf::RESOURCE_DESCRIPTOR_BUFFER_EXT
                    | Buf::SAMPLER_DESCRIPTOR_BUFFER_EXT
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Copy, Clone)]
struct Range {
    start: u64,
    end: u64,
}

impl Range {
    fn size(&self) -> u64 {
        self.end - self.start
    }
}

struct InnerDeviceAllocator {
    buffer: DeviceBuffer,
    ranges: Vec<Range>,
}

#[derive(Clone)]
pub struct DeviceBuffer {
    pub size: u64,
    pub alignment: u64,
    pub device_addr: u64,
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub addr: *mut c_void,
    pub type_index: u32,
    pub kind: BufferKind,
}

impl DeviceBuffer {
    // Max alignment a buffer of any type can have
    const MAX_ALIGNMENT: u64 = 256;

    pub fn new(ctx: &VulkanContext, size: u64, kind: BufferKind) -> Self {
        use vk::MemoryPropertyFlags as Mpf;
        let usage_flags = kind.to_vk_usage_flags();
        let mem_flags = Mpf::DEVICE_LOCAL | Mpf::HOST_VISIBLE | Mpf::HOST_COHERENT;
        let buffer_info = vk::BufferCreateInfo {
            size: Self::next_size(size, Self::MAX_ALIGNMENT),
            usage: usage_flags,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        let buffer: vk::Buffer;
        let mem_reqs: vk::MemoryRequirements;
        unsafe {
            buffer = ctx.device.create_buffer(&buffer_info, None).unwrap();
            mem_reqs = ctx.device.get_buffer_memory_requirements(buffer);
        }
        let alignment = if BufferKind::Descriptor == kind {
            /*
             * Descriptor offset alignment may be wider than the actual memory
             * alignment, defensively use the bigger of the two.
             */
            std::cmp::max(
                mem_reqs.alignment,
                Self::get_descriptor_offset_alignment(&ctx.instance, &ctx.physical_device),
            )
        } else {
            mem_reqs.alignment
        };

        let memi = ctx
            .memory_type_index_for(mem_reqs.memory_type_bits, mem_flags)
            .expect("Unable to find suitable memorytype for the buffer");
        let mut mem_flags = vk::MemoryAllocateFlagsInfo {
            flags: vk::MemoryAllocateFlags::DEVICE_ADDRESS,
            ..Default::default()
        };
        let mem_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_reqs.size)
            .memory_type_index(memi)
            .push_next(&mut mem_flags)
            .build();
        let device_addr_info = vk::BufferDeviceAddressInfo {
            buffer,
            ..Default::default()
        };
        let mem: vk::DeviceMemory;
        let addr: *mut c_void;
        let device_addr: u64;
        unsafe {
            mem = ctx.device.allocate_memory(&mem_info, None).unwrap();
            addr = ctx
                .device
                .map_memory(mem, 0, mem_reqs.size, vk::MemoryMapFlags::empty())
                .unwrap();
            ctx.device.bind_buffer_memory(buffer, mem, 0).unwrap();
            device_addr = ctx.device.get_buffer_device_address(&device_addr_info);
        }

        let name = kind.to_string();
        ctx.try_set_debug_name(&name, buffer);

        return Self {
            type_index: memi,
            buffer,
            addr,
            kind,
            device_addr,
            alignment,
            memory: mem,
            size: mem_info.allocation_size,
        };
    }

    fn get_descriptor_offset_alignment(
        instance: &ash::Instance,
        physical_device: &vk::PhysicalDevice,
    ) -> u64 {
        let mut props = vk::PhysicalDeviceDescriptorBufferPropertiesEXT {
            ..Default::default()
        };
        let mut device_props = vk::PhysicalDeviceProperties2::builder()
            .push_next(&mut props)
            .build();
        unsafe { instance.get_physical_device_properties2(*physical_device, &mut device_props) };

        props.descriptor_buffer_offset_alignment
    }

    fn next_size(base: u64, mul: u64) -> u64 {
        let mask = -(mul as i64) as u64;
        (base + (mul - 1)) & mask
    }
}

impl InnerDeviceAllocator {
    fn new(ctx: &VulkanContext, size: u64, kind: BufferKind) -> Self {
        let buffer = DeviceBuffer::new(ctx, size, kind);
        Self::wrap(buffer)
    }

    fn wrap(buffer: DeviceBuffer) -> Self {
        let ranges = vec![Range {
            start: 0,
            end: buffer.size,
        }];
        return Self { buffer, ranges };
    }

    fn alloc(&mut self, size: u64) -> Option<DeviceSlice> {
        let size = DeviceBuffer::next_size(size, self.buffer.alignment);
        let ranges = &mut self.ranges;
        for i in 0..ranges.len() {
            let range = &ranges[i];
            let range_size = range.size();
            if range_size < size {
                continue;
            }
            let old_start = range.start;
            let new_start = old_start + size;
            if new_start == range.end {
                // Took the range
                ranges.remove(i);
            }
            let range = &mut ranges[i];
            range.start = new_start;
            let mut addr = self.buffer.addr;
            let offset;
            unsafe {
                addr = addr.offset(old_start as isize);
                offset = addr.offset_from(self.buffer.addr) as u64;
            }
            let device_addr = self.buffer.device_addr + offset;
            return Some(DeviceSlice {
                buffer: self.buffer.buffer,
                addr,
                size,
                offset,
                alignment: self.buffer.alignment,
                device_addr,
                kind: self.buffer.kind,
            });
        }
        return None;
    }

    fn free(&mut self, slice: DeviceSlice) {
        // | | | | | |
        let slice_start = unsafe { slice.addr.offset(-(self.buffer.addr as isize)) as u64 };
        let slice_end = slice_start + slice.size;
        let mut idx = 0;
        for i in 0..self.ranges.len() {
            idx = i;
            let range = self.ranges[i];
            if range.start <= slice_start {
                continue;
            }
            if range.start == slice_end {
                let mut new_start = slice_start;
                if i > 0 {
                    let prev_range = self.ranges[i - 1];
                    if prev_range.end == slice_start {
                        //  . <- remove
                        // |f|f|o|o|
                        new_start = prev_range.start;
                        idx = i - 1;
                        self.ranges.remove(idx);
                    }
                }
                //  . <- extend backwards
                // |f|o|o|
                let range = &mut self.ranges[idx];
                range.start = new_start;
                return;
            }
            if i != 0 {
                let prev_range = &mut self.ranges[i - 1];
                if prev_range.end == slice_start {
                    //  . <- extend forwards
                    // |f|o|o|
                    prev_range.end = slice_end;
                    return;
                }
                //    . <- insert
                // |o|f|o|
            }
            //  . <- insert
            // |f|o|o|
            break;
        }
        self.ranges.insert(
            idx,
            Range {
                start: slice_start,
                end: slice_end,
            },
        );
    }

    fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_buffer(self.buffer.buffer, None);
            device.free_memory(self.buffer.memory, None);
        }
    }

    fn available(&self) -> u64 {
        self.ranges.iter().map(|r| r.size()).sum()
    }
}
