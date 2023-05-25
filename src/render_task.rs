use glam::{Mat3, Mat4, Vec3};

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

pub enum ResourceKind {
    Mvp = 0,
    ModelView = 1,
    PrevMvp = 2,
    FMat3 = 3,
    ViewPos = 4,
    ViewDir = 5,
    Color = 6,
    FVec3 = 7,
    Animation = 8,
    Material = 9,
    DirLight = 10,
    PointLight = 11,
    SpotLight = 12,
    ShadowLightId = 13,
    ShadowMatrix = 14,
    Sky = 15,
}

impl ResourceKind {
    fn mask(self) -> u32 {
        !(u32::MAX << self as u32)
    }
}
pub struct Transform {
    pub mvp: Mat4,
    pub mv: Mat4,
}
pub struct TransformExtra {
    pub prev_mvp: Mat4,
}
pub struct Material {
    pub diffuse_handle: u32,
    pub normal_handle: u32,
    pub glow_handle: u32,
}
pub struct DirLight {
    pub view_dir: Vec3,
    pub intensity: f32,
    pub ambient_intensity: f32,
    pub sky_color: Vec3,
    pub ground_color: Vec3,
    pub color: Vec3,
    pub inv_view_shadow_proj: Mat4,
}
pub struct PointLight {
    pub radius: f32,
    pub intensity: f32,
    pub color: Vec3,
}
pub struct SpotLight {
    pub cos_cutoff_rad: f32,
    pub sin_cutoff_rad: f32,
    pub range: f32,
    pub inv_range: f32,
    pub intensity: f32,
    pub color: Vec3,
}

pub enum ResourceWrapper {
    Mvp(Vec<Mat4>),
    ModelView(Vec<Mat4>),
    PrevMvp(Vec<Mat4>),
    FMat3(Vec<Mat3>),
    ViewPos(Vec<Vec3>),
    ViewDir(Vec<Vec3>),
    Color(Vec<Vec3>),
    FVec3(Vec<Vec3>),
    Animation,
    Material(Vec<Material>),
    DirLight(Vec<DirLight>),
    PointLight(Vec<PointLight>),
    SpotLight(Vec<SpotLight>),
    ShadowLightId,
    ShadowMatrix,
    Sky,
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
    pub resources: Vec<ResourceWrapper>,
}
