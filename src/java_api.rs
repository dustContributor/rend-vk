use std::sync::atomic::{AtomicBool, Ordering};

use ash::vk;
use bitvec::view::BitView;

use crate::{
    render_task::{
        self, DirLight, Material, ResourceKind, ResourceWrapper, TaskKind, Transform,
        TransformExtra, WrapResource,
    },
    renderer::{self, MeshBuffer, Renderer},
};

// Prevent calling init twice just in case
static INITIALIZED: AtomicBool = AtomicBool::new(false);
// Convenience definitions
const JNI_FALSE: u8 = 0;
const JNI_TRUE: u8 = 1;

#[derive(Clone, Copy)]
#[repr(C, packed(4))]
pub struct MeshAddresses {
    pub vertices: u64,
    pub normals: u64,
    pub tex_coords: u64,
    pub indices: u64,
    pub vertices_len: u32,
    pub normals_len: u32,
    pub tex_coords_len: u32,
    pub indices_len: u32,
    pub count: u32,
}

impl MeshBuffer {
    pub fn to_addresses(&self) -> MeshAddresses {
        MeshAddresses {
            vertices: self.vertices.addr as u64,
            normals: self.normals.addr as u64,
            tex_coords: self.tex_coords.addr as u64,
            indices: self.indices.addr as u64,
            count: self.count,
            vertices_len: self.vertices.size as u32,
            normals_len: self.normals.size as u32,
            tex_coords_len: self.tex_coords.size as u32,
            indices_len: self.indices.size as u32,
        }
    }
}

#[no_mangle]
pub extern "C" fn Java_game_render_vulkan_RendVkApi_init(
    _unused_jnienv: usize,
    _unused_jclazz: usize,
) -> u8 {
    if INITIALIZED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
        log_panics::init();
        return JNI_TRUE;
    }
    return JNI_FALSE;
}

#[no_mangle]
pub extern "C" fn Java_game_render_vulkan_RendVkApi_makeRenderer(
    _unused_jnienv: usize,
    _unused_jclazz: usize,
    window: u64,
    instance_extensions: u64,
    instance_extensions_len: u64,
    glfw_create_window_surface: u64,
) -> u64 {
    /*
     * VkResult glfwCreateWindowSurface (
     * VkInstance instance,
     * GLFWwindow *window,
     * const VkAllocationCallbacks *allocator,
     * VkSurfaceKHR *surface)
     */
    let glfw_create_window_surface = unsafe {
        std::mem::transmute::<
            _,
            extern "C" fn(vk::Instance, u64, u64, *mut vk::SurfaceKHR) -> vk::Result,
        >(glfw_create_window_surface as *const ())
    };
    let instance_extensions: &[*const i8] = if instance_extensions_len == 0 {
        &[]
    } else {
        unsafe {
            std::slice::from_raw_parts(
                instance_extensions as *const *const i8,
                instance_extensions_len as usize,
            )
        }
    };
    let renderer = renderer::make_renderer(instance_extensions, |_, instance, surface| {
        glfw_create_window_surface(instance.handle(), window, 0, surface)
    });
    let boxed = Box::from(renderer);
    let ptr = Box::into_raw(boxed) as u64;
    log::trace!("renderer finished!");
    return ptr;
}

#[no_mangle]
pub extern "C" fn Java_game_render_vulkan_RendVkApi_alignOf(
    _unused_jnienv: usize,
    _unused_jclazz: usize,
    kind: u32,
) -> u64 {
    let kind = ResourceKind::of_u32(kind);
    kind.resource_align() as u64
}

#[no_mangle]
pub extern "C" fn Java_game_render_vulkan_RendVkApi_sizeOf(
    _unused_jnienv: usize,
    _unused_jclazz: usize,
    kind: u32,
) -> u64 {
    let kind = ResourceKind::of_u32(kind);
    kind.resource_size() as u64
}

#[no_mangle]
pub extern "C" fn Java_game_render_vulkan_RendVkApi_render(
    _unused_jnienv: usize,
    _unused_jclazz: usize,
    renderer: u64,
) {
    let mut renderer = unsafe { Box::from_raw(renderer as *mut Renderer) };
    renderer.render();
    Box::leak(renderer);
}

#[no_mangle]
pub extern "C" fn Java_game_render_vulkan_RendVkApi_genMesh(
    _unused_jnienv: usize,
    _unused_jclazz: usize,
    renderer: u64,
    vertices_size: u32,
    normals_size: u32,
    tex_coords_size: u32,
    indices_size: u32,
    count: u32,
) -> u32 {
    let mut renderer = unsafe { Box::from_raw(renderer as *mut Renderer) };
    let mesh_id = renderer.gen_mesh(
        vertices_size,
        normals_size,
        tex_coords_size,
        indices_size,
        count,
    );
    Box::leak(renderer);
    return mesh_id;
}

#[no_mangle]
pub extern "C" fn Java_game_render_vulkan_RendVkApi_fetchMesh(
    _unused_jnienv: usize,
    _unused_jclazz: usize,
    renderer: u64,
    id: u32,
    dest: u64,
) {
    let renderer = unsafe { Box::from_raw(renderer as *mut Renderer) };
    let mesh = renderer
        .fetch_mesh(id)
        .unwrap_or_else(|| panic!("couldn't find mesh with id {}", id));
    let dest = unsafe {
        std::slice::from_raw_parts_mut(
            dest as *mut MeshAddresses,
            std::mem::size_of::<MeshAddresses>(),
        )
    };
    dest[0] = mesh.to_addresses();
    Box::leak(renderer);
}

#[no_mangle]
pub extern "C" fn Java_game_render_vulkan_RendVkApi_addTaskToQueue(
    _unused_jnienv: usize,
    _unused_jclazz: usize,
    renderer: u64,
    kind: u32,
    mesh_id: u32,
    instance_count: u32,
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
        instance_count,
        mesh_buffer_id: mesh_id,
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
    let resource_bits = resource_bits.view_bits::<bitvec::order::Lsb0>();
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
