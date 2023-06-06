use std::mem::{align_of, size_of};

use glam::{Mat4, Vec3};

#[derive(Copy, Clone)]
pub enum TaskKind {
    MeshStatic,
    MeshAnimated,
    LightDir,
    LightPoint,
    LightSpot,
    ShadowDir,
    ShadowPoint,
    ShadowSpot,
    ShadowDirStatic,
    ShadowPointStatic,
    ShadowSpotStatic,
    ShadowDirAnimated,
    ShadowPointAnimated,
    ShadowSpotAnimated,
    WireframeStatic,
    Skybox,
    Sky,
    Fullscreen,
    Nuklear,
}

impl TaskKind {
    pub const MAX_VALUE: u8 = unsafe { std::mem::transmute(TaskKind::Nuklear) };
    pub const MAX_SIZE: usize = Self::MAX_VALUE as usize;
    pub const MAX_LEN: usize = Self::MAX_SIZE + 1;

    pub fn of_u32(v: u32) -> Self {
        if v > (Self::MAX_VALUE as u32) {
            panic!()
        } else {
            unsafe { std::mem::transmute(v as u8) }
        }
    }
}

#[derive(Clone, Copy, strum_macros::Display)]
#[repr(u8)]
pub enum ResourceKind {
    Transform = 0,
    Material = 1,
    DirLight = 2,
    Frustum = 3,
    ViewRay = 4,
    PointLight = 5,
    SpotLight = 6,
    Joint = 7,
    Sky = 8,
    StaticShadow = 9,
    TransformExtra = 10,
}

impl ResourceKind {
    pub const MAX_VALUE: u8 = unsafe { std::mem::transmute(ResourceKind::TransformExtra) };
    pub const MAX_SIZE: usize = Self::MAX_VALUE as usize;
    pub const MAX_LEN: usize = Self::MAX_SIZE + 1;

    fn mask(self) -> u32 {
        !(u32::MAX << self as u32)
    }

    pub fn of_u8(v: u8) -> Self {
        if v > Self::MAX_VALUE {
            panic!()
        } else {
            unsafe { std::mem::transmute(v) }
        }
    }

    pub fn of_u32(v: u32) -> Self {
        if v > (Self::MAX_VALUE as u32) {
            panic!()
        } else {
            unsafe { std::mem::transmute(v as u8) }
        }
    }

    pub fn of_usize(v: usize) -> Self {
        if v > (Self::MAX_VALUE as usize) {
            panic!()
        } else {
            unsafe { std::mem::transmute(v as u8) }
        }
    }

    pub fn to_u8(self) -> u8 {
        self as u8
    }

    pub fn to_u32(self) -> u32 {
        self as u32
    }

    pub fn to_usize(self) -> usize {
        self as usize
    }

    pub fn resource_align(self) -> usize {
        match self {
            ResourceKind::Transform => align_of::<Transform>(),
            ResourceKind::Material => align_of::<Material>(),
            ResourceKind::DirLight => align_of::<DirLight>(),
            ResourceKind::Frustum => align_of::<Frustum>(),
            ResourceKind::ViewRay => align_of::<ViewRay>(),
            ResourceKind::PointLight => align_of::<PointLight>(),
            ResourceKind::SpotLight => align_of::<SpotLight>(),
            ResourceKind::Joint => align_of::<Joint>(),
            ResourceKind::Sky => align_of::<Sky>(),
            ResourceKind::StaticShadow => align_of::<StaticShadow>(),
            ResourceKind::TransformExtra => align_of::<TransformExtra>(),
        }
    }

    pub fn resource_size(&self) -> usize {
        match self {
            ResourceKind::Transform => size_of::<Transform>(),
            ResourceKind::Material => size_of::<Material>(),
            ResourceKind::DirLight => size_of::<DirLight>(),
            ResourceKind::Frustum => size_of::<Frustum>(),
            ResourceKind::ViewRay => size_of::<ViewRay>(),
            ResourceKind::PointLight => size_of::<PointLight>(),
            ResourceKind::SpotLight => size_of::<SpotLight>(),
            ResourceKind::Joint => size_of::<Joint>(),
            ResourceKind::Sky => size_of::<Sky>(),
            ResourceKind::StaticShadow => size_of::<StaticShadow>(),
            ResourceKind::TransformExtra => size_of::<TransformExtra>(),
        }
    }
}

impl IntoDeviceBuffer for Transform {
    fn into_device(&self, dst: *mut std::ffi::c_void) {
        let slice = unsafe {
            std::slice::from_raw_parts_mut(
                dst as *mut f32,
                size_of::<Transform>() / size_of::<f32>(),
            )
        };
        self.mvp.write_cols_to_slice(slice);
        // self.mv.write_cols_to_slice(slice);
    }
}

pub trait IntoDeviceBuffer {
    fn into_device(&self, dst: *mut std::ffi::c_void);
}

#[derive(Clone)]
pub struct Transform {
    pub mvp: Mat4,
    pub mv: Mat4,
}
#[derive(Clone)]
pub struct TransformExtra {
    pub prev_mvp: Mat4,
}
#[derive(Clone)]
pub struct Material {
    pub diffuse_handle: u32,
    pub normal_handle: u32,
    pub glow_handle: u32,
}
#[derive(Clone)]
pub struct DirLight {
    pub view_dir: Vec3,
    pub intensity: f32,
    pub ambient_intensity: f32,
    pub sky_color: Vec3,
    pub ground_color: Vec3,
    pub color: Vec3,
    pub inv_view_shadow_proj: Mat4,
}
#[derive(Clone)]
pub struct PointLight {
    pub radius: f32,
    pub intensity: f32,
    pub color: Vec3,
}
#[derive(Clone)]
pub struct SpotLight {
    pub cos_cutoff_rad: f32,
    pub sin_cutoff_rad: f32,
    pub range: f32,
    pub inv_range: f32,
    pub intensity: f32,
    pub color: Vec3,
}

#[derive(Clone)]
pub struct Frustum {}
#[derive(Clone)]
pub struct ViewRay {}
#[derive(Clone)]
pub struct Joint {}
#[derive(Clone)]
pub struct StaticShadow {}
#[derive(Clone)]
pub struct Sky {}

pub enum ResourceWrapper {
    Transform(Vec<Transform>),
    Material(Vec<Material>),
    DirLight(Vec<DirLight>),
    Frustum(Vec<Frustum>),
    ViewRay(Vec<ViewRay>),
    PointLight(Vec<PointLight>),
    SpotLight(Vec<SpotLight>),
    Joint(Vec<Joint>),
    Sky(Vec<Sky>),
    StaticShadow(Vec<StaticShadow>),
    TransformExtra(Vec<TransformExtra>),
}

pub trait WrapResource<T> {
    fn wrapper_for(res: &[T]) -> ResourceWrapper {
        panic!("{}::def::invalid", std::any::type_name::<T>())
    }
}

impl WrapResource<Transform> for ResourceWrapper {
    fn wrapper_for(res: &[Transform]) -> ResourceWrapper {
        ResourceWrapper::Transform(res.to_vec())
    }
}

impl WrapResource<TransformExtra> for ResourceWrapper {
    fn wrapper_for(res: &[TransformExtra]) -> ResourceWrapper {
        ResourceWrapper::TransformExtra(res.to_vec())
    }
}

impl WrapResource<Material> for ResourceWrapper {
    fn wrapper_for(res: &[Material]) -> ResourceWrapper {
        ResourceWrapper::Material(res.to_vec())
    }
}

impl WrapResource<DirLight> for ResourceWrapper {
    fn wrapper_for(res: &[DirLight]) -> ResourceWrapper {
        ResourceWrapper::DirLight(res.to_vec())
    }
}

// public final byte type;
// public final int meshId;
// public final int instanceCount;
// public final int vertexCount;
// public final int baseVertex;
// /**
//  * In bytes.
//  */
// public final int indicesOffset;
// public final int primitive;
pub struct RenderTask {
    pub kind: TaskKind,
    pub mesh_id: u32,
    pub instance_count: u32,
    pub vertex_count: u32,
    pub base_vertex: u32,
    pub indices_offset: u32,
    pub resources: [ResourceWrapper; ResourceKind::MAX_LEN],
}

pub fn resource_array() -> [ResourceWrapper; ResourceKind::MAX_LEN] {
    return [
        ResourceWrapper::Transform(Vec::new()),
        ResourceWrapper::Material(Vec::new()),
        ResourceWrapper::DirLight(Vec::new()),
        ResourceWrapper::Frustum(Vec::new()),
        ResourceWrapper::ViewRay(Vec::new()),
        ResourceWrapper::PointLight(Vec::new()),
        ResourceWrapper::SpotLight(Vec::new()),
        ResourceWrapper::Joint(Vec::new()),
        ResourceWrapper::Sky(Vec::new()),
        ResourceWrapper::StaticShadow(Vec::new()),
        ResourceWrapper::TransformExtra(Vec::new()),
    ];
}
