use std::{
    collections::HashMap,
    mem::size_of,
    sync::atomic::{AtomicBool, Ordering},
};

use ash::vk;
use bitvec::view::BitView;

use crate::{
    format::Format,
    pipeline::{
        file::{CompareFunc, Filtering, WrapMode},
        sampler::SamplerKey,
    },
    pos_mul,
    render_task::{self, TaskKind},
    renderer::{self, MeshBuffer, Renderer},
    shader_resource::*,
    texture::{MipMap, Texture},
};

// Prevent calling init twice just in case
static INITIALIZED: AtomicBool = AtomicBool::new(false);
// Convenience definitions
const JNI_FALSE: u8 = 0;
const JNI_TRUE: u8 = 1;

const MISSING_SAMPLER_ID: u8 = u8::MAX;

trait ToJava<T> {
    fn to_java(&self) -> T;
}

#[derive(Clone, Copy, Debug)]
#[repr(C, packed(4))]
pub struct JavaMesh {
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

impl ToJava<JavaMesh> for MeshBuffer {
    fn to_java(&self) -> JavaMesh {
        JavaMesh {
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

#[derive(Clone, Copy, Debug)]
#[repr(C, packed(4))]
pub struct JavaMipMap {
    pub index: u32,
    pub width: u32,
    pub height: u32,
    pub size: u32,
    pub offset: u32,
}

impl ToJava<JavaMipMap> for MipMap {
    fn to_java(&self) -> JavaMipMap {
        JavaMipMap {
            index: self.index,
            width: self.width,
            height: self.height,
            size: self.size,
            offset: self.offset,
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C, packed(4))]
pub struct JavaTexture {
    pub width: u32,
    pub height: u32,
    pub mip_map_count: u32,
    pub staging: u64,
    pub staging_len: u32,
}

impl ToJava<JavaTexture> for Texture {
    fn to_java(&self) -> JavaTexture {
        let staging_buffer = if let Some(buff) = &self.staging {
            (buff.addr as u64, buff.size as u32)
        } else {
            (0, 0)
        };
        JavaTexture {
            width: self.extent().width,
            height: self.extent().height,
            mip_map_count: self.mip_map_count(),
            staging: staging_buffer.0,
            staging_len: staging_buffer.1,
        }
    }
}

fn to_renderer(addr: u64) -> Box<Renderer> {
    unsafe { Box::from_raw(addr as *mut Renderer) }
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
    instance_extensions_len: u32,
    glfw_create_window_surface: u64,
    is_vsync_enabled: u8,
    is_debug_enabled: u8,
    is_validation_layer_enabled: u8,
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
            extern "C" fn(vk::Instance, u64, u64, *const vk::SurfaceKHR) -> vk::Result,
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
    let renderer = renderer::make_renderer(
        is_vsync_enabled == JNI_TRUE,
        is_debug_enabled == JNI_TRUE,
        is_validation_layer_enabled == JNI_TRUE,
        instance_extensions,
        |_, instance, surface| glfw_create_window_surface(instance.handle(), window, 0, surface),
    );
    let boxed = Box::from(renderer);
    let ptr = Box::into_raw(boxed) as u64;
    log::trace!("renderer finished!");
    return ptr;
}

#[no_mangle]
pub extern "C" fn Java_game_render_vulkan_RendVkApi_resourceAlignOf(
    _unused_jnienv: usize,
    _unused_jclazz: usize,
    kind: u32,
) -> u32 {
    let kind = ResourceKind::of_u32(kind);
    kind.resource_align() as u32
}

#[no_mangle]
pub extern "C" fn Java_game_render_vulkan_RendVkApi_resourceSizeOf(
    _unused_jnienv: usize,
    _unused_jclazz: usize,
    kind: u32,
) -> u32 {
    let kind = ResourceKind::of_u32(kind);
    kind.resource_size() as u32
}

#[no_mangle]
pub extern "C" fn Java_game_render_vulkan_RendVkApi_getCurrentFrame(
    _unused_jnienv: usize,
    _unused_jclazz: usize,
    renderer: u64,
) -> u64 {
    let renderer = to_renderer(renderer);
    let v = renderer.get_current_frame();
    Box::leak(renderer);
    return v;
}

#[no_mangle]
pub extern "C" fn Java_game_render_vulkan_RendVkApi_render(
    _unused_jnienv: usize,
    _unused_jclazz: usize,
    renderer: u64,
) {
    let mut renderer = to_renderer(renderer);
    renderer.render();
    Box::leak(renderer);
}

#[no_mangle]
pub extern "C" fn Java_game_render_vulkan_RendVkApi_tryGetSampler(
    _unused_jnienv: usize,
    _unused_jclazz: usize,
    renderer: u64,
    filter: u8,
    wrap_mode: u8,
    compare_func: u8,
    anisotropy: u8,
) -> u8 {
    let renderer = to_renderer(renderer);
    let sampler = renderer.try_get_sampler(SamplerKey {
        filter: Filtering::of_u8(filter),
        wrap_mode: WrapMode::of_u8(wrap_mode),
        compare_func: CompareFunc::of_u8(compare_func),
        anisotropy,
    });
    Box::leak(renderer);
    match sampler {
        Some(id) => id.try_into().unwrap(),
        None => MISSING_SAMPLER_ID,
    }
}

#[no_mangle]
pub extern "C" fn Java_game_render_vulkan_RendVkApi_getSampler(
    _unused_jnienv: usize,
    _unused_jclazz: usize,
    renderer: u64,
    filter: u8,
    wrap_mode: u8,
    compare_func: u8,
    anisotropy: u8,
) -> u8 {
    let mut renderer = to_renderer(renderer);
    let sampler = renderer.get_sampler(SamplerKey {
        filter: Filtering::of_u8(filter),
        wrap_mode: WrapMode::of_u8(wrap_mode),
        compare_func: CompareFunc::of_u8(compare_func),
        anisotropy,
    });
    Box::leak(renderer);
    sampler.try_into().unwrap()
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
    let mut renderer = to_renderer(renderer);
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
    let renderer = to_renderer(renderer);
    let mesh = renderer
        .fetch_mesh(id)
        .unwrap_or_else(|| panic!("couldn't find mesh with id {}", id));
    let dest = unsafe { std::slice::from_raw_parts_mut(dest as *mut JavaMesh, 1) };
    dest[0] = mesh.to_java();
    Box::leak(renderer);
}

#[no_mangle]
pub extern "C" fn Java_game_render_vulkan_RendVkApi_genTexture(
    _unused_jnienv: usize,
    _unused_jclazz: usize,
    renderer: u64,
    format: u32,
    mip_maps: u64,
    mip_maps_len: u32,
    name: u64,
    name_len: u32,
    staging_size: u32,
) -> u32 {
    let mut renderer = to_renderer(renderer);
    let mip_map_count = mip_maps_len / size_of::<JavaMipMap>() as u32;
    let expected_mip_map_size = size_of::<JavaMipMap>() as u32 * mip_map_count;
    assert!(
        expected_mip_map_size == mip_maps_len,
        "mip_maps_len can't hold an exact count of mip maps!"
    );
    let name = if name_len > 0 {
        let name_chars =
            unsafe { std::slice::from_raw_parts(name as *const u8, name_len as usize) };
        std::str::from_utf8(name_chars).expect("invalid name utf8 string!")
    } else {
        "java_texture"
    };
    let mip_maps: Vec<_> = unsafe {
        std::slice::from_raw_parts(mip_maps as *const JavaMipMap, mip_map_count as usize)
    }
    .iter()
    .map(|e| MipMap {
        width: e.width,
        height: e.height,
        index: e.index,
        offset: e.offset,
        size: e.size,
    })
    .collect();
    let texture_id = renderer.gen_texture(
        name.to_string(),
        Format::of_u32(format),
        &mip_maps,
        staging_size,
    );
    Box::leak(renderer);
    return texture_id;
}

#[no_mangle]
pub extern "C" fn Java_game_render_vulkan_RendVkApi_fetchTexture(
    _unused_jnienv: usize,
    _unused_jclazz: usize,
    renderer: u64,
    id: u32,
    dest: u64,
) {
    let renderer = to_renderer(renderer);
    let texture = renderer
        .fetch_texture(id)
        .unwrap_or_else(|| panic!("couldn't find texture with id {}", id));
    let dest = unsafe { std::slice::from_raw_parts_mut(dest as *mut JavaTexture, 1) };
    dest[0] = texture.to_java();
    Box::leak(renderer);
}

#[no_mangle]
pub extern "C" fn Java_game_render_vulkan_RendVkApi_fetchTextureMipMaps(
    _unused_jnienv: usize,
    _unused_jclazz: usize,
    renderer: u64,
    id: u32,
    dest: u64,
) {
    let renderer = to_renderer(renderer);
    let texture = renderer
        .fetch_texture(id)
        .unwrap_or_else(|| panic!("couldn't find texture with id {}", id));
    let dest = unsafe {
        std::slice::from_raw_parts_mut(
            dest as *mut JavaMipMap,
            size_of::<JavaMipMap>() * texture.mip_map_count() as usize,
        )
    };
    for (i, item) in texture.mip_maps.iter().enumerate() {
        dest[i] = item.to_java();
    }
    Box::leak(renderer);
}

#[no_mangle]
pub extern "C" fn Java_game_render_vulkan_RendVkApi_queueTextureForUploading(
    _unused_jnienv: usize,
    _unused_jclazz: usize,
    renderer: u64,
    id: u32,
) {
    let mut renderer = to_renderer(renderer);
    renderer.queue_texture_for_uploading(id);
    Box::leak(renderer);
}

#[no_mangle]
pub extern "C" fn Java_game_render_vulkan_RendVkApi_isTextureUploaded(
    _unused_jnienv: usize,
    _unused_jclazz: usize,
    renderer: u64,
    id: u32,
) -> u8 {
    let renderer = to_renderer(renderer);
    let is_uploaded = renderer.is_texture_uploaded(id);
    Box::leak(renderer);
    return if is_uploaded { JNI_TRUE } else { JNI_FALSE };
}

#[no_mangle]
pub extern "C" fn Java_game_render_vulkan_RendVkApi_placeShaderResource(
    _unused_jnienv: usize,
    _unused_jclazz: usize,
    renderer: u64,
    kind: u32,
    resource: u64,
    resource_len: u32,
) {
    let mut renderer = to_renderer(renderer);
    let kind = ResourceKind::of_u32(kind);
    let data = unsafe { std::slice::from_raw_parts(resource as *const u8, resource_len as usize) };
    let (resource, _) = match kind {
        ResourceKind::Transform => unpack_single_resource::<Transform>(data),
        ResourceKind::Material => unpack_single_resource::<Material>(data),
        ResourceKind::DirLight => unpack_single_resource::<DirLight>(data),
        ResourceKind::Frustum => unpack_single_resource::<Frustum>(data),
        ResourceKind::ViewRay => unpack_single_resource::<ViewRay>(data),
        ResourceKind::PointLight => unpack_single_resource::<PointLight>(data),
        ResourceKind::SpotLight => unpack_single_resource::<SpotLight>(data),
        ResourceKind::Joint => unpack_single_resource::<Joint>(data),
        ResourceKind::Sky => unpack_single_resource::<Sky>(data),
        ResourceKind::StaticShadow => unpack_single_resource::<StaticShadow>(data),
        ResourceKind::TransformExtra => unpack_single_resource::<TransformExtra>(data),
        ResourceKind::View => unpack_single_resource::<View>(data),
        ResourceKind::Timing => unpack_single_resource::<Timing>(data),
    };
    renderer.place_shader_resource(kind, resource);
    Box::leak(renderer);
}

#[no_mangle]
pub extern "C" fn Java_game_render_vulkan_RendVkApi_addTaskToQueue(
    _unused_jnienv: usize,
    _unused_jclazz: usize,
    renderer: u64,
    kind: u32,
    mesh_id: u32,
    parent_id: u32,
    instance_count: u32,
    vertex_count: u32,
    indices_offset: u32,
    resource_bits: u32,
    resources: u64,
    resources_len: u32,
) {
    let mut renderer = to_renderer(renderer);
    let kind = TaskKind::of_u32(kind);
    let data =
        unsafe { std::slice::from_raw_parts(resources as *const u8, resources_len as usize) };
    let resources = unpack_render_task_resources(data, resource_bits, instance_count);
    let task = render_task::RenderTask {
        kind,
        resources,
        instance_count,
        vertex_count,
        indices_offset,
        mesh_buffer_id: mesh_id,
    };
    renderer.add_task_to_queue(task, parent_id);
    Box::leak(renderer);
}

fn unpack_render_task_resources(
    data: &[u8],
    resource_bits: u32,
    instances: u32,
) -> HashMap<ResourceKind, MultiResource> {
    let instances = instances as usize;
    let resource_bits = resource_bits.view_bits::<bitvec::order::Lsb0>();
    let mut offset = 0usize;
    let mut resources_by_kind = HashMap::with_capacity(resource_bits.count_ones());
    for b in resource_bits.iter_ones() {
        let kind = ResourceKind::of_usize(b);
        let (wrapper, next_end) = match kind {
            ResourceKind::Transform => unpack_multi_resource::<Transform>(offset, instances, data),
            ResourceKind::Material => unpack_multi_resource::<Material>(offset, instances, data),
            ResourceKind::DirLight => unpack_multi_resource::<DirLight>(offset, instances, data),
            // ResourceKind::Frustum => unpack_multi_resources::<Frustum>(start, end, data),
            // ResourceKind::ViewRay => unpack_multi_resources::<ViewRay>(start, end, data),
            ResourceKind::PointLight => {
                unpack_multi_resource::<PointLight>(offset, instances, data)
            }
            // ResourceKind::SpotLight => unpack_multi_resources::<SpotLight>(start, end, data),
            // ResourceKind::Joint => unpack_multi_resources::<Joint>(start, end, data),
            // ResourceKind::Sky => unpack_multi_resources::<Sky>(start, end, data),
            ResourceKind::StaticShadow => {
                unpack_multi_resource::<StaticShadow>(offset, instances, data)
            }
            ResourceKind::TransformExtra => {
                unpack_multi_resource::<TransformExtra>(offset, instances, data)
            }
            _ => panic!("unrecognized resource kind {}", kind),
        };
        offset = next_end;
        resources_by_kind.insert(kind, wrapper);
    }
    return resources_by_kind;
}

fn unpack_single_resource<T>(data: &[u8]) -> (SingleResource, usize)
where
    T: WrapResource<T>,
{
    let (res, next_end) = unpack_resource::<T>(0, 1, data);
    (T::single_wrapper_for(res), next_end)
}

fn unpack_multi_resource<T>(start: usize, count: usize, data: &[u8]) -> (MultiResource, usize)
where
    T: WrapResource<T>,
{
    let (res, next_end) = unpack_resource::<T>(start, count, data);
    (T::multi_wrapper_for(res), next_end)
}

fn unpack_resource<T>(start: usize, count: usize, data: &[u8]) -> (&[T], usize)
where
    T: WrapResource<T>,
{
    let start_aligned = pos_mul(core::mem::align_of::<T>(), start);
    let slice_aligned = &data[start_aligned..];

    assert!(
        slice_aligned.len() >= count * std::mem::size_of::<T>(),
        "unexpected resource {} count! expected {count}, got only {}",
        std::any::type_name::<T>(),
        slice_aligned.len() / std::mem::size_of::<T>()
    );

    let items = unsafe { std::slice::from_raw_parts(slice_aligned.as_ptr().cast::<T>(), count) };

    let next_end = items.as_ptr_range().end as usize - data.as_ptr() as usize;
    return (items, next_end);
}
