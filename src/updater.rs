use crate::{
    buffer::{DeviceAllocator, DeviceSlice},
    shader_resource::{MultiResource, SingleResource},
};

fn alloc_and_copy_into<T>(mem: &DeviceAllocator, src: &[T], count: u32) -> DeviceSlice {
    if count as usize != src.len() {
        panic!(
            "expected {} resources of type {}, found {}",
            count,
            std::any::type_name::<T>(),
            src.len()
        );
    }
    let per_item_size = std::mem::size_of::<T>() as u64;
    let total_size = per_item_size * count as u64;
    let device = mem.alloc(total_size).unwrap();
    let src = src.as_ptr() as *const u8;
    let dst = device.addr as *mut u8;
    unsafe {
        std::ptr::copy_nonoverlapping(src, dst, total_size as usize);
    }
    device
}

fn copy_into<T>(src: *const T, dst: &DeviceSlice, offset: u64) -> u64 {
    let per_item_size = std::mem::size_of::<T>();
    let src = src as *const u8;
    let dst = dst.addr as *mut u8;
    unsafe {
        std::ptr::copy_nonoverlapping(src, dst, per_item_size);
    }
    offset + per_item_size as u64
}

pub fn alloc_and_fill_multi(
    mem: &DeviceAllocator,
    resource: &MultiResource,
    instance_count: u32,
) -> DeviceSlice {
    match resource {
        MultiResource::Transform(e) => alloc_and_copy_into(mem, e, instance_count),
        MultiResource::Material(e) => alloc_and_copy_into(mem, e, instance_count),
        MultiResource::DirLight(e) => alloc_and_copy_into(mem, e, instance_count),
        MultiResource::Frustum(e) => alloc_and_copy_into(mem, e, instance_count),
        MultiResource::ViewRay(e) => alloc_and_copy_into(mem, e, instance_count),
        MultiResource::PointLight(e) => alloc_and_copy_into(mem, e, instance_count),
        MultiResource::SpotLight(e) => alloc_and_copy_into(mem, e, instance_count),
        MultiResource::Joint(e) => alloc_and_copy_into(mem, e, instance_count),
        MultiResource::Sky(e) => alloc_and_copy_into(mem, e, instance_count),
        MultiResource::StaticShadow(e) => alloc_and_copy_into(mem, e, instance_count),
        MultiResource::TransformExtra(e) => alloc_and_copy_into(mem, e, instance_count),
    }
}

pub fn fill_single(src: &SingleResource, dst: &DeviceSlice, offset: u64) -> u64 {
    match src {
        SingleResource::Transform(e) => copy_into(e, dst, offset),
        SingleResource::Material(e) => copy_into(e, dst, offset),
        SingleResource::DirLight(e) => copy_into(e, dst, offset),
        SingleResource::Frustum(e) => copy_into(e, dst, offset),
        SingleResource::ViewRay(e) => copy_into(e, dst, offset),
        SingleResource::PointLight(e) => copy_into(e, dst, offset),
        SingleResource::SpotLight(e) => copy_into(e, dst, offset),
        SingleResource::Joint(e) => copy_into(e, dst, offset),
        SingleResource::Sky(e) => copy_into(e, dst, offset),
        SingleResource::StaticShadow(e) => copy_into(e, dst, offset),
        SingleResource::TransformExtra(e) => copy_into(e, dst, offset),
    }
}
