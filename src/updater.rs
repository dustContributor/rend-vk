use crate::{
    buffer::{DeviceAllocator, DeviceSlice},
    shader_resource::{MultiResource, SingleResource},
};

fn copy_into<T>(mem: &DeviceAllocator, src: &[T], count: u32) -> DeviceSlice {
    if count as usize != src.len() {
        panic!(
            "expected {} resources of type {}, found {}",
            count,
            std::any::type_name::<T>(),
            src.len()
        );
    }
    copy_ptr_into(mem, src.as_ptr(), count)
}

fn copy_ptr_into<T>(mem: &DeviceAllocator, src: *const T, count: u32) -> DeviceSlice {
    let per_item_size = std::mem::size_of::<T>() as u64;
    let total_size = per_item_size * count as u64;
    let device = mem.alloc(total_size).unwrap();
    let src = src as *const u8;
    let dst = device.addr as *mut u8;
    unsafe {
        std::ptr::copy_nonoverlapping(src, dst, total_size as usize);
    }
    device
}

pub fn alloc_and_fill_multi(
    mem: &DeviceAllocator,
    resource: &MultiResource,
    instance_count: u32,
) -> DeviceSlice {
    match resource {
        MultiResource::Transform(e) => copy_into(mem, e, instance_count),
        MultiResource::Material(e) => copy_into(mem, e, instance_count),
        MultiResource::DirLight(e) => copy_into(mem, e, instance_count),
        MultiResource::Frustum(e) => copy_into(mem, e, instance_count),
        MultiResource::ViewRay(e) => copy_into(mem, e, instance_count),
        MultiResource::PointLight(e) => copy_into(mem, e, instance_count),
        MultiResource::SpotLight(e) => copy_into(mem, e, instance_count),
        MultiResource::Joint(e) => copy_into(mem, e, instance_count),
        MultiResource::Sky(e) => copy_into(mem, e, instance_count),
        MultiResource::StaticShadow(e) => copy_into(mem, e, instance_count),
        MultiResource::TransformExtra(e) => copy_into(mem, e, instance_count),
    }
}

pub fn alloc_and_fill_single(mem: &DeviceAllocator, resource: &SingleResource) -> DeviceSlice {
    match resource {
        SingleResource::Transform(e) => copy_ptr_into(mem, e, 1),
        SingleResource::Material(e) => copy_ptr_into(mem, e, 1),
        SingleResource::DirLight(e) => copy_ptr_into(mem, e, 1),
        SingleResource::Frustum(e) => copy_ptr_into(mem, e, 1),
        SingleResource::ViewRay(e) => copy_ptr_into(mem, e, 1),
        SingleResource::PointLight(e) => copy_ptr_into(mem, e, 1),
        SingleResource::SpotLight(e) => copy_ptr_into(mem, e, 1),
        SingleResource::Joint(e) => copy_ptr_into(mem, e, 1),
        SingleResource::Sky(e) => copy_ptr_into(mem, e, 1),
        SingleResource::StaticShadow(e) => copy_ptr_into(mem, e, 1),
        SingleResource::TransformExtra(e) => copy_ptr_into(mem, e, 1),
    }
}
