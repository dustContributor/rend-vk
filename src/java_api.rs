use bitvec::{prelude::Msb0, view::BitView};

use crate::{
    render_task::{
        self, DirLight, Material, ResourceKind, ResourceWrapper, TaskKind, Transform,
        TransformExtra, WrapResource,
    },
    renderer::{self, Renderer},
};

#[no_mangle]
pub extern "C" fn Java_test_Testing_make_1renderer(
    _unused_jnienv: usize,
    _unused_jclazz: usize,
    window: u64,
    instance_extensions: u64,
    instance_extensions_len: u64,
    glfw_create_window_surface: u64,
) -> u64 {
    renderer::make_renderer(
        window,
        instance_extensions,
        instance_extensions_len,
        glfw_create_window_surface,
    )
}

#[no_mangle]
pub extern "C" fn Java_test_Testing_init(_unused_jnienv: usize, _unused_jclazz: usize) {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    log_panics::init();
}

#[no_mangle]
pub extern "C" fn Java_test_Testing_align_of(
    _unused_jnienv: usize,
    _unused_jclazz: usize,
    kind: u32,
) -> u64 {
    let kind = ResourceKind::of_u32(kind);
    kind.resource_align() as u64
}

#[no_mangle]
pub extern "C" fn Java_test_Testing_size_of(
    _unused_jnienv: usize,
    _unused_jclazz: usize,
    kind: u32,
) -> u64 {
    let kind = ResourceKind::of_u32(kind);
    kind.resource_size() as u64
}

#[no_mangle]
pub extern "C" fn Java_test_Testing_render(
    _unused_jnienv: usize,
    _unused_jclazz: usize,
    renderer: u64,
) {
    let renderer = unsafe { Box::from_raw(renderer as *mut Renderer) };
    renderer.render();
    Box::leak(renderer);
}

#[no_mangle]
pub extern "C" fn Java_test_Testing_add_task_to_queue(
    _unused_jnienv: usize,
    _unused_jclazz: usize,
    renderer: u64,
    kind: u32,
    mesh_id: u32,
    instance_count: u32,
    vertex_count: u32,
    base_vertex: u32,
    indices_offset: u32,
    resource_bits: u32,
    resources: u64,
    resources_len: u64,
) {
    let mut renderer = unsafe { Box::from_raw(renderer as *mut Renderer) };
    let kind = TaskKind::of_u32(kind);
    let data =
        unsafe { std::slice::from_raw_parts(resources as *const u8, resources_len as usize) };
    let resources = unpack_render_task_resources(data, resource_bits, instance_count);
    let task = render_task::RenderTask {
        kind,
        resources,
        base_vertex,
        indices_offset,
        instance_count,
        mesh_id,
        vertex_count,
    };
    renderer.add_task_to_queue(task);
    Box::leak(renderer);
}

fn unpack_render_task_resources(
    data: &[u8],
    resource_bits: u32,
    instances: u32,
) -> [ResourceWrapper; 11] {
    let instances = instances as usize;
    let resource_bits = resource_bits.view_bits::<Msb0>();
    let mut offset = 0usize;
    let mut resource_array = render_task::resource_array();
    for b in resource_bits.iter_ones() {
        let kind = ResourceKind::of_usize(b);
        let (wrapper, next_end) = match kind {
            ResourceKind::Transform => unpack_resources::<Transform>(offset, instances, data),
            ResourceKind::Material => unpack_resources::<Material>(offset, instances, data),
            ResourceKind::DirLight => unpack_resources::<DirLight>(offset, instances, data),
            // ResourceKind::Frustum => unpack_resources::<Frustum>(start, end, data),
            // ResourceKind::ViewRay => unpack_resources::<ViewRay>(start, end, data),
            // ResourceKind::PointLight => unpack_resources::<PointLight>(start, end, data),
            // ResourceKind::SpotLight => unpack_resources::<SpotLight>(start, end, data),
            // ResourceKind::Joint => unpack_resources::<Joint>(start, end, data),
            // ResourceKind::Sky => unpack_resources::<Sky>(start, end, data),
            // ResourceKind::StaticShadow => unpack_resources::<StaticShadow>(start, end, data),
            ResourceKind::TransformExtra => {
                unpack_resources::<TransformExtra>(offset, instances, data)
            }
            _ => panic!("unrecognized resource kind {}", kind),
        };
        offset = next_end;
        resource_array[b] = wrapper;
    }
    return resource_array;
}

fn unpack_resources<T>(start: usize, count: usize, data: &[u8]) -> (ResourceWrapper, usize)
where
    ResourceWrapper: WrapResource<T>,
{
    let (prefix, total, _) = unsafe { data[start..].align_to::<T>() };
    if prefix.len() > 0 {
        panic!("misaligned struct array!");
    }
    if total.len() != count {
        panic!(
            "unexpected resource count! expected {}, got {}",
            count,
            total.len()
        );
    }
    let items = &total[..count];
    let next_end = items.as_ptr_range().end as usize - data.as_ptr() as usize;
    let wrapper = ResourceWrapper::wrapper_for(items);
    return (wrapper, next_end);
}
