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
