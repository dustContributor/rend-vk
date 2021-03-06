use ash::vk;
use serde::Deserialize;

use super::state::*;
use crate::{format, render};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pipeline {
    pub targets: Vec<Target>,
    pub programs: Vec<Program>,
    pub passes: Vec<Pass>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub name: String,
    pub group: String,
    pub format: format::Format,
    pub width: U32OrF32,
    pub height: U32OrF32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pass {
    pub name: String,
    pub program: String,
    pub batch: render::BatchType,
    pub outputs: Vec<String>,
    pub inputs: Vec<String>,
    pub updaters: Vec<String>,
    pub state: State,
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
            color: Some(0),
            depth: Some(0.0),
            stencil: None,
        }
    }
    fn yes() -> ClearDesc {
        Self {
            color: Some(0),
            depth: Some(0.0),
            stencil: Some(0),
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

#[derive(Deserialize)]
#[serde(untagged)]
#[derive(Copy, Clone)]
pub enum U32OrF32 {
    U32(u32),
    F32(f32),
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
    // Empty.
}

impl BlendDesc {
    pub fn to_vk(&self) -> vk::PipelineColorBlendStateCreateInfo {
        vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op(vk::LogicOp::CLEAR)
            .attachments(&[vk::PipelineColorBlendAttachmentState {
                blend_enable: if self.disabled { 0 } else { 1 },
                src_color_blend_factor: self.src_factor.to_vk(),
                dst_color_blend_factor: self.dst_factor.to_vk(),
                color_blend_op: vk::BlendOp::ADD,
                src_alpha_blend_factor: vk::BlendFactor::ZERO,
                dst_alpha_blend_factor: vk::BlendFactor::ZERO,
                alpha_blend_op: vk::BlendOp::ADD,
                color_write_mask: vk::ColorComponentFlags::RGBA,
            }])
            .build()
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
            max_depth_bounds: self.range_end,
            min_depth_bounds: self.range_start,
            ..Default::default()
        }
    }
}

impl ViewportDesc {
    pub fn to_vk(&self, window_width: f32, window_height: f32) -> vk::Viewport {
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
            min_depth: 0.0,
            max_depth: 1.0,
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
