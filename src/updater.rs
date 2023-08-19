use crate::{
    buffer::{DeviceAllocator, DeviceSlice},
    render_task::RenderTask,
    shader_resource::{ResourceKind, ResourceWrapper},
};

fn copy_into<T>(
    mem: &DeviceAllocator,
    src: &Vec<T>,
    count: u32,
    kind: ResourceKind,
) -> DeviceSlice {
    if count as usize != src.len() {
        panic!(
            "expected {} resources of type {}, found {}",
            count,
            std::any::type_name::<T>(),
            src.len()
        );
    }
    let len = kind.resource_size() as u64 * count as u64;
    let device = mem.alloc(len).unwrap();
    let src = src.as_ptr() as *const u8;
    let dst = device.addr as *mut u8;
    unsafe {
        std::ptr::copy_nonoverlapping(src, dst, len as usize);
    }
    device
}

pub fn alloc_and_fill(mem: &DeviceAllocator, task: &RenderTask, kind: ResourceKind) -> DeviceSlice {
    match &task.resources.get(&kind) {
        Some(resource) => match resource {
            ResourceWrapper::Transform(e) => copy_into(mem, e, task.instance_count, kind),
            ResourceWrapper::Material(e) => copy_into(mem, e, task.instance_count, kind),
            ResourceWrapper::DirLight(e) => copy_into(mem, e, task.instance_count, kind),
            ResourceWrapper::Frustum(e) => copy_into(mem, e, task.instance_count, kind),
            ResourceWrapper::ViewRay(e) => copy_into(mem, e, task.instance_count, kind),
            ResourceWrapper::PointLight(e) => copy_into(mem, e, task.instance_count, kind),
            ResourceWrapper::SpotLight(e) => copy_into(mem, e, task.instance_count, kind),
            ResourceWrapper::Joint(e) => copy_into(mem, e, task.instance_count, kind),
            ResourceWrapper::Sky(e) => copy_into(mem, e, task.instance_count, kind),
            ResourceWrapper::StaticShadow(e) => copy_into(mem, e, task.instance_count, kind),
            ResourceWrapper::TransformExtra(e) => copy_into(mem, e, task.instance_count, kind),
        },
        _ => panic!("unknown resource kind {}", kind),
    }
}
