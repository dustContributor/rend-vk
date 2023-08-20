use std::{
    collections::HashMap,
    hash::Hash,
    mem::{align_of, size_of},
};

use glam::{Mat4, Vec3};

use crate::UsedAsIndex;

#[derive(PartialEq, Eq, Clone, Copy, strum_macros::Display, Hash)]
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
    pub const fn mask(self) -> u32 {
        !(u32::MAX << self as u32)
    }

    pub const fn of_u8(v: u8) -> Self {
        if v > Self::MAX_VALUE {
            panic!()
        } else {
            unsafe { std::mem::transmute(v) }
        }
    }

    pub const fn of_u32(v: u32) -> Self {
        if v > (Self::MAX_VALUE as u32) {
            panic!()
        } else {
            unsafe { std::mem::transmute(v as u8) }
        }
    }

    pub const fn of_usize(v: usize) -> Self {
        if v > (Self::MAX_VALUE as usize) {
            panic!()
        } else {
            unsafe { std::mem::transmute(v as u8) }
        }
    }

    pub const fn to_u8(self) -> u8 {
        self as u8
    }

    pub const fn to_u32(self) -> u32 {
        self as u32
    }

    pub const fn to_usize(self) -> usize {
        self as usize
    }

    pub const fn resource_align(self) -> usize {
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

    pub const fn resource_size(&self) -> usize {
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

const MAX_RESOURCE_KIND: u8 = ResourceKind::TransformExtra.to_u8();
impl UsedAsIndex<MAX_RESOURCE_KIND> for ResourceKind {}

#[derive(Clone)]
#[repr(C)]
pub struct Transform {
    pub mvp: Mat4,
    pub mv: Mat4,
}
#[derive(Clone)]
#[repr(C)]
pub struct TransformExtra {
    pub prev_mvp: Mat4,
}
#[derive(Clone)]
#[repr(C)]
pub struct Material {
    pub shininess: f32,
    pub diffuse_handle: u32,
    pub normal_handle: u32,
    pub glow_handle: u32,
}
#[derive(Clone)]
#[repr(C)]
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
#[repr(C)]
pub struct PointLight {
    pub radius: f32,
    pub intensity: f32,
    pub color: Vec3,
}
#[derive(Clone)]
#[repr(C)]
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

pub enum MultiResource {
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

pub enum SingleResource {
    Transform(Transform),
    Material(Material),
    DirLight(DirLight),
    Frustum(Frustum),
    ViewRay(ViewRay),
    PointLight(PointLight),
    SpotLight(SpotLight),
    Joint(Joint),
    Sky(Sky),
    StaticShadow(StaticShadow),
    TransformExtra(TransformExtra),
}

pub fn resources_by_kind_map() -> HashMap<ResourceKind, MultiResource> {
    HashMap::new()
}

pub trait WrapResource<T> {
    fn single_wrapper_for(_: &[T]) -> SingleResource {
        panic!("{}::def::invalid", std::any::type_name::<T>())
    }
    fn multi_wrapper_for(_: &[T]) -> MultiResource {
        panic!("{}::def::invalid", std::any::type_name::<T>())
    }
}
// TODO: Better way to do this?
impl WrapResource<Transform> for Transform {
    fn multi_wrapper_for(res: &[Transform]) -> MultiResource {
        MultiResource::Transform(res.to_vec())
    }
    fn single_wrapper_for(res: &[Transform]) -> SingleResource {
        SingleResource::Transform(res[0].clone())
    }
}
impl WrapResource<Material> for Material {
    fn multi_wrapper_for(res: &[Material]) -> MultiResource {
        MultiResource::Material(res.to_vec())
    }
    fn single_wrapper_for(res: &[Material]) -> SingleResource {
        SingleResource::Material(res[0].clone())
    }
}
impl WrapResource<DirLight> for DirLight {
    fn multi_wrapper_for(res: &[DirLight]) -> MultiResource {
        MultiResource::DirLight(res.to_vec())
    }
    fn single_wrapper_for(res: &[DirLight]) -> SingleResource {
        SingleResource::DirLight(res[0].clone())
    }
}
impl WrapResource<Frustum> for Frustum {
    fn multi_wrapper_for(res: &[Frustum]) -> MultiResource {
        MultiResource::Frustum(res.to_vec())
    }
    fn single_wrapper_for(res: &[Frustum]) -> SingleResource {
        SingleResource::Frustum(res[0].clone())
    }
}
impl WrapResource<ViewRay> for ViewRay {
    fn multi_wrapper_for(res: &[ViewRay]) -> MultiResource {
        MultiResource::ViewRay(res.to_vec())
    }
    fn single_wrapper_for(res: &[ViewRay]) -> SingleResource {
        SingleResource::ViewRay(res[0].clone())
    }
}
impl WrapResource<PointLight> for PointLight {
    fn multi_wrapper_for(res: &[PointLight]) -> MultiResource {
        MultiResource::PointLight(res.to_vec())
    }
    fn single_wrapper_for(res: &[PointLight]) -> SingleResource {
        SingleResource::PointLight(res[0].clone())
    }
}
impl WrapResource<SpotLight> for SpotLight {
    fn multi_wrapper_for(res: &[SpotLight]) -> MultiResource {
        MultiResource::SpotLight(res.to_vec())
    }
    fn single_wrapper_for(res: &[SpotLight]) -> SingleResource {
        SingleResource::SpotLight(res[0].clone())
    }
}
impl WrapResource<Joint> for Joint {
    fn multi_wrapper_for(res: &[Joint]) -> MultiResource {
        MultiResource::Joint(res.to_vec())
    }
    fn single_wrapper_for(res: &[Joint]) -> SingleResource {
        SingleResource::Joint(res[0].clone())
    }
}
impl WrapResource<Sky> for Sky {
    fn multi_wrapper_for(res: &[Sky]) -> MultiResource {
        MultiResource::Sky(res.to_vec())
    }
    fn single_wrapper_for(res: &[Sky]) -> SingleResource {
        SingleResource::Sky(res[0].clone())
    }
}
impl WrapResource<StaticShadow> for StaticShadow {
    fn multi_wrapper_for(res: &[StaticShadow]) -> MultiResource {
        MultiResource::StaticShadow(res.to_vec())
    }
    fn single_wrapper_for(res: &[StaticShadow]) -> SingleResource {
        SingleResource::StaticShadow(res[0].clone())
    }
}
impl WrapResource<TransformExtra> for TransformExtra {
    fn multi_wrapper_for(res: &[TransformExtra]) -> MultiResource {
        MultiResource::TransformExtra(res.to_vec())
    }
    fn single_wrapper_for(res: &[TransformExtra]) -> SingleResource {
        SingleResource::TransformExtra(res[0].clone())
    }
}
