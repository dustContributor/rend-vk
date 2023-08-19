use std::{collections::HashMap, hash::Hash};

use crate::shader_resource::{ResourceKind, MultiResource};
use crate::UsedAsIndex;

#[derive(Copy, Clone, Eq, PartialEq, Hash, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[repr(u8)]
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
}

const MAX_TASK_KIND: u8 = TaskKind::Nuklear.to_u8();
impl crate::UsedAsIndex<MAX_TASK_KIND> for TaskKind {}

pub struct RenderTask {
    pub kind: TaskKind,
    pub mesh_buffer_id: u32,
    pub instance_count: u32,
    pub resources: HashMap<ResourceKind, MultiResource>,
}
