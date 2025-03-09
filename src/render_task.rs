use std::{collections::HashMap, hash::Hash};

use serde::{Deserialize, Serialize};

use crate::shader_resource::{MultiResource, ResourceKind};
use crate::UsedAsIndex;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[repr(u8)]
pub enum TaskKind {
    Undefined = 0,
    MeshStatic = 1,
    MeshAnimated = 2,
    LightDir = 3,
    LightPoint = 4,
    LightSpot = 5,
    MeshStaticShadowDir = 6,
    MeshStaticShadowPoint = 7,
    MeshStaticShadowSpot = 8,
    WireframeStatic = 9,
    Skybox = 10,
    Sky = 11,
    Fullscreen = 12,
    Nuklear = 13,
}

const MAX_TASK_KIND: u8 = TaskKind::Nuklear.to_u8();
impl crate::UsedAsIndex<MAX_TASK_KIND> for TaskKind {}

impl TaskKind {
    pub const fn of_u8(v: u8) -> Self {
        Self::of_u64(v as u64)
    }

    pub const fn of_u32(v: u32) -> Self {
        Self::of_u64(v as u64)
    }

    pub const fn of_usize(v: usize) -> Self {
        Self::of_u64(v as u64)
    }

    pub const fn of_u64(v: u64) -> Self {
        if v < 1 || v > (Self::MAX_VALUE as u64) {
            panic!("invalid task kind!")
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

    pub const fn to_key(self, parent_id: u32) -> u64 {
        (self.to_u32() as u64) << 32 | parent_id as u64
    }
}

#[derive(Serialize)]
pub struct RenderTask {
    pub kind: TaskKind,
    pub mesh_buffer_id: u32,
    pub instance_count: u32,
    pub vertex_count: u32,
    pub indices_offset: u32,
    pub resources: HashMap<ResourceKind, MultiResource>,
}
