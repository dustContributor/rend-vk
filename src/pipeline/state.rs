use ash::vk;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(Copy, Clone)]
pub enum PolygonFace {
    None,
    Font,
    Back,
    FrontAndBack,
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(Copy, Clone)]
pub enum PolygonMode {
    Fill,
    Line,
    Point,
}
#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(Copy, Clone)]
pub enum WindingOrder {
    Cw,
    Ccw,
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(Copy, Clone)]
pub enum CompareFunc {
    Never,
    Less,
    Equal,
    LessOrEqual,
    Greater,
    NotEqual,
    GreaterOrEqual,
    Always,
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(Copy, Clone)]
pub enum StencilFunc {
    Keep,
    Zero,
    Replace,
    Incr,
    Decr,
    Invert,
    IncrWrap,
    DecrWrap,
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(Copy, Clone)]
pub enum BlendFactor {
    Zero,
    One,
    SrcColor,
    OneMinusSrcColor,
    DstColor,
    OneMinusDstColor,
    SrcAlpha,
    OneMinusSrcAlpha,
    DstAlpha,
    OneMinusDstAlpha,
    ConstantColor,
    OneMinusConstantColor,
    ConstantAlpha,
    OneMinusConstantAlpha,
    SrcAlphaSaturate,
    Src1Color,
    OneMinusSrc1Color,
    Src1Alpha,
    OneMinusSrc1Alpha,
}

impl BlendFactor {
    pub fn to_vk(self) -> vk::BlendFactor {
        match self {
            BlendFactor::Zero => vk::BlendFactor::ZERO,
            BlendFactor::One => vk::BlendFactor::ONE,
            BlendFactor::SrcColor => vk::BlendFactor::SRC_COLOR,
            BlendFactor::OneMinusSrcColor => vk::BlendFactor::ONE_MINUS_SRC_COLOR,
            BlendFactor::DstColor => vk::BlendFactor::DST_COLOR,
            BlendFactor::OneMinusDstColor => vk::BlendFactor::ONE_MINUS_DST_COLOR,
            BlendFactor::SrcAlpha => vk::BlendFactor::SRC_ALPHA,
            BlendFactor::OneMinusSrcAlpha => vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
            BlendFactor::DstAlpha => vk::BlendFactor::DST_ALPHA,
            BlendFactor::OneMinusDstAlpha => vk::BlendFactor::ONE_MINUS_DST_ALPHA,
            BlendFactor::ConstantColor => vk::BlendFactor::CONSTANT_COLOR,
            BlendFactor::OneMinusConstantColor => vk::BlendFactor::ONE_MINUS_CONSTANT_COLOR,
            BlendFactor::ConstantAlpha => vk::BlendFactor::CONSTANT_ALPHA,
            BlendFactor::OneMinusConstantAlpha => vk::BlendFactor::ONE_MINUS_CONSTANT_ALPHA,
            BlendFactor::SrcAlphaSaturate => vk::BlendFactor::SRC_ALPHA_SATURATE,
            BlendFactor::Src1Color => vk::BlendFactor::SRC1_COLOR,
            BlendFactor::OneMinusSrc1Color => vk::BlendFactor::ONE_MINUS_SRC1_COLOR,
            BlendFactor::Src1Alpha => vk::BlendFactor::SRC1_ALPHA,
            BlendFactor::OneMinusSrc1Alpha => vk::BlendFactor::ONE_MINUS_SRC1_ALPHA,
        }
    }
}

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
        }
    }
}

impl StencilFunc {
    pub fn to_vk(self) -> vk::StencilOp {
        match self {
            StencilFunc::Keep => vk::StencilOp::KEEP,
            StencilFunc::Zero => vk::StencilOp::ZERO,
            StencilFunc::Replace => vk::StencilOp::REPLACE,
            StencilFunc::Incr => vk::StencilOp::INCREMENT_AND_CLAMP,
            StencilFunc::Decr => vk::StencilOp::DECREMENT_AND_CLAMP,
            StencilFunc::Invert => vk::StencilOp::INVERT,
            StencilFunc::IncrWrap => vk::StencilOp::INCREMENT_AND_WRAP,
            StencilFunc::DecrWrap => vk::StencilOp::DECREMENT_AND_WRAP,
        }
    }
}

impl WindingOrder {
    pub fn to_vk(self) -> vk::FrontFace {
        match self {
            WindingOrder::Cw => vk::FrontFace::CLOCKWISE,
            WindingOrder::Ccw => vk::FrontFace::COUNTER_CLOCKWISE,
        }
    }
}

impl PolygonFace {
    pub fn to_vk(self) -> vk::CullModeFlags {
        match self {
            PolygonFace::None => vk::CullModeFlags::NONE,
            PolygonFace::Font => vk::CullModeFlags::FRONT,
            PolygonFace::Back => vk::CullModeFlags::BACK,
            PolygonFace::FrontAndBack => vk::CullModeFlags::FRONT_AND_BACK,
        }
    }
}

impl PolygonMode {
    pub fn to_vk(self) -> vk::PolygonMode {
        match self {
            PolygonMode::Fill => vk::PolygonMode::FILL,
            PolygonMode::Line => vk::PolygonMode::LINE,
            PolygonMode::Point => vk::PolygonMode::POINT,
        }
    }
}
