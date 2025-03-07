use ash::vk;
use serde::Deserialize;

use super::state::*;
use crate::{format, shader_resource::ResourceKind, UsedAsIndex};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pipeline {
    pub targets: Vec<Target>,
    pub programs: Vec<Program>,
    pub passes: Vec<PipelineStep>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub name: String,
    pub format: format::Format,
    pub width: U32OrF32,
    pub height: U32OrF32,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentInput {
    pub name: String,
    pub sampler: DescOption<Sampler>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Copy, Clone)]
pub struct Sampler {
    pub filter: Filtering,
    pub wrap_mode: WrapMode,
    pub compare_func: CompareFunc,
    #[serde(default)]
    pub anisotropy: u8,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderPass {
    pub name: String,
    pub program: String,
    pub depth_stencil: Option<String>,
    pub batch: crate::render_task::TaskKind,
    pub outputs: Vec<String>,
    pub inputs: Vec<AttachmentInput>,
    pub per_pass_updaters: Vec<UpdaterKind>,
    pub per_instance_updaters: Vec<UpdaterKind>,
    pub state: State,
    #[serde(default)]
    pub is_disabled: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Copy, Clone)]
pub struct BlitRect {
    pub x: U32OrF32,
    pub y: U32OrF32,
    pub width: U32OrF32,
    pub height: U32OrF32,
}

impl BlitRect {
    pub fn to_vk(&self, window_width: f32, window_height: f32) -> [vk::Offset3D; 2] {
        let x = self.x.scale(window_width);
        let y = self.y.scale(window_height);
        let width = self.width.scale(window_width);
        let height = self.height.scale(window_height);
        [
            vk::Offset3D {
                z: 0,
                x: x as i32,
                y: y as i32,
            },
            vk::Offset3D {
                z: 1,
                x: width as i32,
                y: height as i32,
            },
        ]
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(Copy, Clone)]
pub enum BlitAttribute {
    Color,
    Depth,
    Stencil,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlitPass {
    pub name: String,
    pub input: String,
    pub input_rect: BlitRect,
    pub output: String,
    pub output_rect: BlitRect,
    pub filter: Filtering,
    pub attributes: Vec<BlitAttribute>,
    #[serde(default)]
    pub is_disabled: bool,
}

impl BlitPass {
    pub fn to_vk(&self, window_width: f32, window_height: f32) -> vk::ImageBlit {
        let aspect_flags = self
            .attributes
            .iter()
            .map(|e| match e {
                BlitAttribute::Color => vk::ImageAspectFlags::COLOR,
                BlitAttribute::Depth => vk::ImageAspectFlags::DEPTH,
                BlitAttribute::Stencil => vk::ImageAspectFlags::STENCIL,
            })
            .reduce(|acc, e| acc | e)
            .expect("missing attributes to blit!");
        let layer = vk::ImageSubresourceLayers {
            mip_level: 0,
            base_array_layer: 0,
            layer_count: 1,
            aspect_mask: aspect_flags,
        };
        vk::ImageBlit {
            dst_subresource: layer,
            src_subresource: layer,
            src_offsets: self.input_rect.to_vk(window_width, window_height),
            dst_offsets: self.output_rect.to_vk(window_width, window_height),
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PipelineStep {
    Render(RenderPass),
    Blit(BlitPass),
}

impl PipelineStep {
    pub fn is_disabled(&self) -> bool {
        match self {
            PipelineStep::Render(p) => p.is_disabled,
            PipelineStep::Blit(p) => p.is_disabled,
        }
    }
    pub fn name(&self) -> &str {
        match self {
            PipelineStep::Render(p) => &p.name,
            PipelineStep::Blit(p) => &p.name,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(Copy, Clone)]
pub enum UpdaterKind {
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
    View = 11,
    Timing = 12,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub writing: DescOption<WriteDesc>,
    pub depth: DescOption<DepthDesc>,
    pub scissor: DescOption<ScissorDesc>,
    pub viewport: DescOption<ViewportDesc>,
    pub stencil: DescOption<StencilDesc>,
    pub triangle: DescOption<TriangleDesc>,
    pub blending: DescOption<BlendDesc>,
    pub clearing: DescOption<ClearDesc>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Program {
    pub name: String,
    #[serde(default)]
    pub vertex: String,
    #[serde(default)]
    pub fragment: String,
    #[serde(default)]
    pub geometry: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Copy, Clone)]
pub struct StencilDesc {
    pub func: CompareFunc,
    pub ref_value: u32,
    pub read_mask: u32,
    pub fail_op: StencilFunc,
    pub depth_fail_op: StencilFunc,
    pub pass_op: StencilFunc,
    #[serde(skip)]
    pub disabled: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Copy, Clone)]
pub struct ScissorDesc {
    pub x: U32OrF32,
    pub y: U32OrF32,
    pub width: U32OrF32,
    pub height: U32OrF32,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Copy, Clone)]
pub struct ViewportDesc {
    pub x: U32OrF32,
    pub y: U32OrF32,
    pub width: U32OrF32,
    pub height: U32OrF32,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Copy, Clone)]
pub struct DepthDesc {
    pub func: CompareFunc,
    pub range_start: f32,
    pub range_end: f32,
    pub testing: bool,
    pub clamping: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Copy, Clone)]
pub struct TriangleDesc {
    pub front_face: WindingOrder,
    pub cull_face: PolygonFace,
    pub polygon_mode: PolygonMode,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Copy, Clone)]
pub struct WriteDesc {
    pub color_mask: u32,
    pub depth: bool,
    pub stencil: bool,
}
impl WriteDesc {
    pub fn depth_or_stencil(self) -> bool {
        self.depth || self.stencil
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Copy, Clone)]
pub struct BlendDesc {
    #[serde(skip)]
    pub disabled: bool,
    pub src_factor: BlendFactor,
    pub dst_factor: BlendFactor,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Copy, Clone)]
pub struct ClearDesc {
    pub color: Option<u32>,
    pub depth: Option<f32>,
    pub stencil: Option<u32>,
}
#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(Copy, Clone, Eq, PartialEq, Hash, strum_macros::Display)]
#[repr(u8)]
pub enum Filtering {
    Linear,
    Nearest,
}

const MAX_FILTERING: u8 = Filtering::Nearest.to_u8();
impl UsedAsIndex<MAX_FILTERING> for Filtering {}

impl Filtering {
    pub fn to_vk(self) -> vk::Filter {
        match self {
            Self::Linear => vk::Filter::LINEAR,
            Self::Nearest => vk::Filter::NEAREST,
        }
    }
    pub fn to_vk_mip_map(self) -> vk::SamplerMipmapMode {
        match self {
            Self::Linear => vk::SamplerMipmapMode::LINEAR,
            Self::Nearest => vk::SamplerMipmapMode::NEAREST,
        }
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

    pub const fn to_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(Copy, Clone, Eq, PartialEq, Hash, strum_macros::Display)]
#[repr(u8)]
pub enum WrapMode {
    Repeat,
    MirroredRepeat,
    ClampToEdge,
}

const MAX_WRAP_MODE: u8 = WrapMode::ClampToEdge.to_u8();
impl crate::UsedAsIndex<MAX_WRAP_MODE> for WrapMode {}

impl WrapMode {
    pub fn to_vk(self) -> vk::SamplerAddressMode {
        match self {
            Self::ClampToEdge => vk::SamplerAddressMode::CLAMP_TO_EDGE,
            Self::Repeat => vk::SamplerAddressMode::REPEAT,
            Self::MirroredRepeat => vk::SamplerAddressMode::MIRRORED_REPEAT,
        }
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

    pub const fn to_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(Copy, Clone, Eq, PartialEq, Hash, strum_macros::Display)]
#[repr(u8)]
pub enum CompareFunc {
    None,
    Always,
    Never,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
    Equal,
    NotEqual,
}

const MAX_COMPARE_FUNC: u8 = CompareFunc::NotEqual.to_u8();
impl crate::UsedAsIndex<MAX_COMPARE_FUNC> for CompareFunc {}

impl CompareFunc {
    pub fn to_vk(self) -> vk::CompareOp {
        match self {
            CompareFunc::Never => vk::CompareOp::NEVER,
            CompareFunc::Less => vk::CompareOp::LESS,
            CompareFunc::Equal => vk::CompareOp::EQUAL,
            CompareFunc::LessOrEqual => vk::CompareOp::LESS_OR_EQUAL,
            CompareFunc::Greater => vk::CompareOp::GREATER,
            CompareFunc::NotEqual => vk::CompareOp::NOT_EQUAL,
            CompareFunc::GreaterOrEqual => vk::CompareOp::GREATER_OR_EQUAL,
            CompareFunc::Always => vk::CompareOp::ALWAYS,
            CompareFunc::None => panic!("'none' is a marker unsupported compare func!"),
        }
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

    pub const fn to_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
#[derive(Copy, Clone)]
pub enum U32OrF32 {
    U32(u32),
    F32(f32),
}

impl U32OrF32 {
    fn scale(self, size: f32) -> f32 {
        match self {
            U32OrF32::U32(v) => v as f32,
            U32OrF32::F32(v) => size * v,
        }
    }
}

const DEFAULT_DEPTH_CLEAR_VALUE: f32 = 1.0;
const DEFAULT_STENCIL_CLEAR_VALUE: u32 = 0;
const DEFAULT_COLOR_CLEAR_VALUE: u32 = 0;

impl UpdaterKind {
    pub const fn to_resource_kind(self) -> ResourceKind {
        ResourceKind::of_u32(self as u32)
    }
}

impl Predefined<Sampler> for Sampler {
    fn def() -> Sampler {
        Self {
            anisotropy: 1,
            compare_func: CompareFunc::None,
            filter: Filtering::Linear,
            wrap_mode: WrapMode::ClampToEdge,
        }
    }
}

impl Predefined<TriangleDesc> for TriangleDesc {
    fn def() -> TriangleDesc {
        Self {
            front_face: WindingOrder::Ccw,
            cull_face: PolygonFace::Back,
            polygon_mode: PolygonMode::Fill,
        }
    }
}

impl Predefined<ScissorDesc> for ScissorDesc {
    fn def() -> ScissorDesc {
        Self {
            x: U32OrF32::U32(0),
            y: U32OrF32::U32(0),
            width: U32OrF32::F32(1.0),
            height: U32OrF32::F32(1.0),
        }
    }
}

impl Predefined<ViewportDesc> for ViewportDesc {
    fn def() -> ViewportDesc {
        Self {
            x: U32OrF32::U32(0),
            y: U32OrF32::U32(0),
            width: U32OrF32::F32(1.0),
            height: U32OrF32::F32(1.0),
        }
    }
}

impl Predefined<StencilDesc> for StencilDesc {
    fn no() -> StencilDesc {
        Self {
            func: CompareFunc::Always,
            ref_value: 0,
            read_mask: 1,
            fail_op: StencilFunc::Keep,
            depth_fail_op: StencilFunc::Keep,
            pass_op: StencilFunc::Keep,
            disabled: true,
        }
    }
}

impl Predefined<DepthDesc> for DepthDesc {
    fn def() -> DepthDesc {
        Self {
            func: CompareFunc::LessOrEqual,
            range_start: 0.0,
            range_end: 1.0,
            testing: true,
            clamping: false,
        }
    }

    fn no() -> DepthDesc {
        Self {
            func: CompareFunc::LessOrEqual,
            range_start: 0.0,
            range_end: 1.0,
            testing: false,
            clamping: false,
        }
    }
}

impl Predefined<BlendDesc> for BlendDesc {
    fn yes() -> BlendDesc {
        Self {
            disabled: false,
            src_factor: BlendFactor::One,
            dst_factor: BlendFactor::One,
        }
    }
    fn no() -> BlendDesc {
        BlendDesc {
            disabled: true,
            src_factor: BlendFactor::One,
            dst_factor: BlendFactor::One,
        }
    }
}

impl Predefined<ClearDesc> for ClearDesc {
    fn def() -> ClearDesc {
        Self {
            color: Some(DEFAULT_COLOR_CLEAR_VALUE),
            depth: Some(DEFAULT_DEPTH_CLEAR_VALUE),
            stencil: None,
        }
    }
    fn yes() -> ClearDesc {
        Self {
            color: Some(DEFAULT_COLOR_CLEAR_VALUE),
            depth: Some(DEFAULT_DEPTH_CLEAR_VALUE),
            stencil: Some(DEFAULT_STENCIL_CLEAR_VALUE),
        }
    }
    fn no() -> ClearDesc {
        Self {
            color: None,
            depth: None,
            stencil: None,
        }
    }
}

impl Predefined<WriteDesc> for WriteDesc {
    fn def() -> WriteDesc {
        Self {
            color_mask: 0xFFFFFFFF,
            depth: true,
            stencil: false,
        }
    }
    fn yes() -> WriteDesc {
        Self {
            color_mask: 0xFFFFFFFF,
            depth: true,
            stencil: true,
        }
    }
    fn no() -> WriteDesc {
        Self {
            color_mask: 0,
            depth: false,
            stencil: false,
        }
    }
}

pub trait Predefined<T> {
    fn def() -> T {
        panic!("{}::def::invalid", std::any::type_name::<T>())
    }
    fn yes() -> T {
        panic!("{}::yes::invalid", std::any::type_name::<T>())
    }
    fn no() -> T {
        panic!("{}::no::invalid", std::any::type_name::<T>())
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
#[derive(Clone)]
pub enum DescOption<T> {
    Predefined(OptionPredefined),
    Specific(String),
    Configured(T),
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(Copy, Clone)]
pub enum OptionPredefined {
    Default,
    No,
    Yes,
}

pub trait DescHandler<T>
where
    T: Predefined<T>,
{
    fn handle_specific(desc: &String) -> T {
        panic!(
            "{}::handle_specific::invalid({})",
            std::any::type_name::<T>(),
            desc
        )
    }
    fn handle_option(desc: DescOption<T>) -> T {
        let desc = match desc {
            DescOption::Predefined(v) => match v {
                OptionPredefined::Default => T::def(),
                OptionPredefined::No => T::no(),
                OptionPredefined::Yes => T::yes(),
            },
            DescOption::Specific(v) => Self::handle_specific(&v),
            DescOption::Configured(v) => v,
        };
        return desc;
    }
}

impl DescHandler<Sampler> for Pipeline {
    // Empty.
}

impl DescHandler<StencilDesc> for Pipeline {
    // Empty.
}

impl DescHandler<WriteDesc> for Pipeline {
    fn handle_specific(desc: &String) -> WriteDesc {
        match desc.as_str() {
            "COLOR" => WriteDesc {
                color_mask: 0xFFFFFFFF,
                depth: false,
                stencil: false,
            },
            "DEPTH" => WriteDesc {
                color_mask: 0,
                depth: true,
                stencil: false,
            },
            "STENCIL" => WriteDesc {
                color_mask: 0,
                depth: false,
                stencil: true,
            },
            _ => panic!("invalid {desc}"),
        }
    }
}
impl DescHandler<TriangleDesc> for Pipeline {
    // Empty.
}

impl DescHandler<BlendDesc> for Pipeline {
    // Empty.
}

impl DescHandler<DepthDesc> for Pipeline {
    // Empty.
}

impl DescHandler<ScissorDesc> for Pipeline {
    // Empty.
}

impl DescHandler<ViewportDesc> for Pipeline {
    // Empty.
}

impl DescHandler<ClearDesc> for Pipeline {
    fn handle_specific(desc: &String) -> ClearDesc {
        match desc.as_str() {
            "COLOR" => ClearDesc {
                color: Some(DEFAULT_COLOR_CLEAR_VALUE),
                depth: None,
                stencil: None,
            },
            "DEPTH" => ClearDesc {
                color: None,
                depth: Some(DEFAULT_DEPTH_CLEAR_VALUE),
                stencil: None,
            },
            "STENCIL" => ClearDesc {
                color: None,
                depth: None,
                stencil: Some(DEFAULT_STENCIL_CLEAR_VALUE),
            },
            _ => panic!("invalid {desc}"),
        }
    }
}

impl BlendDesc {
    pub fn to_vk(
        &self,
        attachment_count: u32,
    ) -> (
        Vec<vk::PipelineColorBlendAttachmentState>,
        vk::PipelineColorBlendStateCreateInfo,
    ) {
        let attachments: Vec<_> = (0..attachment_count)
            .map(|_| vk::PipelineColorBlendAttachmentState {
                blend_enable: if self.disabled { 0 } else { 1 },
                src_color_blend_factor: self.src_factor.to_vk(),
                dst_color_blend_factor: self.dst_factor.to_vk(),
                color_blend_op: vk::BlendOp::ADD,
                src_alpha_blend_factor: vk::BlendFactor::ZERO,
                dst_alpha_blend_factor: vk::BlendFactor::ZERO,
                alpha_blend_op: vk::BlendOp::ADD,
                // TODO: Apply writing.color_mask ?
                color_write_mask: vk::ColorComponentFlags::R
                    | vk::ColorComponentFlags::G
                    | vk::ColorComponentFlags::B
                    | vk::ColorComponentFlags::A,
                ..Default::default()
            })
            .collect();
        let info = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .attachments(&attachments)
            .build();
        (attachments, info)
    }
}

impl StencilDesc {
    pub fn to_vk(&self) -> vk::StencilOpState {
        vk::StencilOpState {
            fail_op: self.fail_op.to_vk(),
            pass_op: self.pass_op.to_vk(),
            depth_fail_op: self.depth_fail_op.to_vk(),
            compare_op: self.func.to_vk(),
            compare_mask: self.read_mask,
            reference: self.ref_value,
            ..Default::default()
        }
    }
}

impl DepthDesc {
    pub fn to_vk(
        &self,
        stencil: vk::StencilOpState,
        writing: &WriteDesc,
    ) -> vk::PipelineDepthStencilStateCreateInfo {
        vk::PipelineDepthStencilStateCreateInfo {
            depth_test_enable: if self.testing { 1 } else { 0 },
            depth_write_enable: if writing.depth { 1 } else { 0 },
            depth_compare_op: self.func.to_vk(),
            front: stencil,
            back: stencil,
            ..Default::default()
        }
    }
}

impl ViewportDesc {
    pub fn to_vk(&self, depth: &DepthDesc, window_width: f32, window_height: f32) -> vk::Viewport {
        vk::Viewport {
            x: match self.x {
                U32OrF32::U32(v) => v as f32,
                U32OrF32::F32(v) => window_width * v,
            },
            y: match self.y {
                U32OrF32::U32(v) => v as f32,
                U32OrF32::F32(v) => window_height * v,
            },
            width: match self.width {
                U32OrF32::U32(v) => v as f32,
                U32OrF32::F32(v) => window_width * v,
            },
            height: match self.height {
                U32OrF32::U32(v) => v as f32,
                U32OrF32::F32(v) => window_height * v,
            },
            min_depth: depth.range_start,
            max_depth: depth.range_end,
            ..Default::default()
        }
    }
}

impl ScissorDesc {
    pub fn to_vk(&self, window_width: f32, window_height: f32) -> vk::Rect2D {
        vk::Rect2D {
            offset: vk::Offset2D {
                x: match self.x {
                    U32OrF32::U32(v) => v as i32,
                    U32OrF32::F32(v) => (window_width * v).ceil() as i32,
                },
                y: match self.y {
                    U32OrF32::U32(v) => v as i32,
                    U32OrF32::F32(v) => (window_height * v).ceil() as i32,
                },
            },
            extent: Pipeline::extent_of(self.width, self.height, window_width, window_height),
            ..Default::default()
        }
    }
}

impl ClearDesc {
    pub fn to_vk_color(&self) -> Option<vk::ClearValue> {
        self.color.and_then(|e| {
            Some(vk::ClearValue {
                color: vk::ClearColorValue {
                    // Convolutedw way to separate a RGBA u32 into a vec4
                    float32: e
                        .to_ne_bytes()
                        .into_iter()
                        .map(|v| (v as f32) / 255.0)
                        .collect::<Vec<_>>()
                        .try_into()
                        .unwrap(),
                },
            })
        })
    }

    pub fn to_vk_depth_stencil(&self) -> Option<vk::ClearValue> {
        if self.depth.is_some() || self.stencil.is_some() {
            Some(vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: self.depth.unwrap_or(0.0),
                    stencil: self.stencil.unwrap_or(0),
                    ..Default::default()
                },
            })
        } else {
            None
        }
    }
}

impl TriangleDesc {
    pub fn to_vk(&self) -> vk::PipelineRasterizationStateCreateInfo {
        vk::PipelineRasterizationStateCreateInfo {
            front_face: self.front_face.to_vk(),
            cull_mode: self.cull_face.to_vk(),
            polygon_mode: self.polygon_mode.to_vk(),
            line_width: 1.0,
            ..Default::default()
        }
    }
}
