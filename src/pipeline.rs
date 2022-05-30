use ash::vk;
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    io::Seek,
    process::Command,
};

use crate::shader;

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
    pub format: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pass {
    pub name: String,
    pub program: String,
    pub batch: String,
    pub outputs: Vec<String>,
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

trait DescHandler<T>
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

impl Pipeline {
    pub fn load(
        device: ash::Device,
        window_format: vk::Format,
        window_width: u32,
        window_height: u32,
        name: Option<&str>,
    ) -> Self {
        let name = name.unwrap_or("pipeline.json");
        let file = std::fs::File::open(name).expect("failed opening the pipeline");
        let pip: Pipeline = serde_json::from_reader(file).expect("couldn't parse the pipeline");

        let shaders_by_name: HashMap<_, _> = pip
            .programs
            .iter()
            .map(|p| vec![&p.fragment, &p.vertex, &p.geometry])
            .flatten()
            .filter(|f| !f.is_empty())
            // Same shader could be used in multiple programs.
            .collect::<HashSet<_>>()
            .iter()
            .map(|f| (format!("shader/{f}"), format!("shader/{f}.spv")))
            .collect();

        for src_out in &shaders_by_name {
            Command::new("glslangValidator")
                .args([&src_out.0, "-V", "-o", &src_out.1])
                .spawn()
                .expect(format!("failed to compile {}", &src_out.0).as_str());
        }

        let load_shader = |name: &String| {
            shaders_by_name.get(name).map(|v| {
                (
                    v.clone(),
                    std::fs::File::open(v).expect(format!("failed opening {v}").as_str()),
                )
            })
        };

        let shader_programs = pip
            .programs
            .iter()
            .map(|f| {
                shader::ShaderProgram::new(
                    &device,
                    f.name.clone(),
                    load_shader(&f.vertex),
                    load_shader(&f.fragment),
                    load_shader(&f.geometry),
                )
            })
            .collect::<Vec<_>>();

        for pass in &pip.passes {
            let writing = Self::handle_option(pass.state.writing.clone());
            let depth = Self::handle_option(pass.state.depth.clone());
            let blending = Self::handle_option(pass.state.blending.clone());
            let stencil = Self::handle_option(pass.state.stencil.clone());
            let viewport = Self::handle_option(pass.state.viewport.clone());
            let scissor = Self::handle_option(pass.state.scissor.clone());
            let triangle = Self::handle_option(pass.state.triangle.clone());
            let clearing = Self::handle_option(pass.state.clearing.clone());
            let blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
                .logic_op(vk::LogicOp::CLEAR)
                .attachments(&[vk::PipelineColorBlendAttachmentState {
                    blend_enable: if blending.disabled { 0 } else { 1 },
                    src_color_blend_factor: blending.src_factor.to_vk(),
                    dst_color_blend_factor: blending.dst_factor.to_vk(),
                    color_blend_op: vk::BlendOp::ADD,
                    src_alpha_blend_factor: vk::BlendFactor::ZERO,
                    dst_alpha_blend_factor: vk::BlendFactor::ZERO,
                    alpha_blend_op: vk::BlendOp::ADD,
                    color_write_mask: vk::ColorComponentFlags::RGBA,
                }])
                .build();
            let stencil_op_state = vk::StencilOpState {
                fail_op: stencil.fail_op.to_vk(),
                pass_op: stencil.pass_op.to_vk(),
                depth_fail_op: stencil.depth_fail_op.to_vk(),
                compare_op: stencil.func.to_vk(),
                compare_mask: stencil.read_mask,
                reference: stencil.ref_value,
                ..Default::default()
            };
            let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo {
                depth_test_enable: if depth.testing { 1 } else { 0 },
                depth_write_enable: if writing.depth { 1 } else { 0 },
                depth_compare_op: depth.func.to_vk(),
                front: stencil_op_state,
                back: stencil_op_state,
                max_depth_bounds: depth.range_end,
                min_depth_bounds: depth.range_start,
                ..Default::default()
            };
            let viewports = [vk::Viewport {
                x: match viewport.x {
                    U32OrF32::U32(v) => v as f32,
                    U32OrF32::F32(v) => window_width as f32 * v,
                },
                y: match viewport.y {
                    U32OrF32::U32(v) => v as f32,
                    U32OrF32::F32(v) => window_height as f32 * v,
                },
                width: match viewport.width {
                    U32OrF32::U32(v) => v as f32,
                    U32OrF32::F32(v) => window_width as f32 * v,
                },
                height: match viewport.height {
                    U32OrF32::U32(v) => v as f32,
                    U32OrF32::F32(v) => window_height as f32 * v,
                },
                min_depth: 0.0,
                max_depth: 1.0,
            }];
            let scissors = [vk::Rect2D {
                offset: vk::Offset2D {
                    x: match scissor.x {
                        U32OrF32::U32(v) => v as i32,
                        U32OrF32::F32(v) => (window_width as f32 * v).ceil() as i32,
                    },
                    y: match scissor.y {
                        U32OrF32::U32(v) => v as i32,
                        U32OrF32::F32(v) => (window_height as f32 * v).ceil() as i32,
                    },
                },
                extent: vk::Extent2D {
                    width: match scissor.width {
                        U32OrF32::U32(v) => v,
                        U32OrF32::F32(v) => (window_width as f32 * v).ceil() as u32,
                    },
                    height: match scissor.height {
                        U32OrF32::U32(v) => v,
                        U32OrF32::F32(v) => (window_height as f32 * v).ceil() as u32,
                    },
                },
            }];
            let viewport_scissor_state = vk::PipelineViewportStateCreateInfo::builder()
                .scissors(&scissors)
                .viewports(&viewports);
            let rasterization_state = vk::PipelineRasterizationStateCreateInfo {
                front_face: triangle.front_face.to_vk(),
                cull_mode: triangle.cull_face.to_vk(),
                polygon_mode: triangle.polygon_mode.to_vk(),
                line_width: 1.0,
                ..Default::default()
            };
            let color_values = clearing
                .color
                .unwrap_or(0)
                .to_ne_bytes()
                .into_iter()
                .map(|v| (v as f32) / 255.0)
                .collect::<Vec<_>>();
            let clear_color_value = vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: color_values.try_into().unwrap(),
                },
            };
            let clear_depth_stencil_value = vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: clearing.depth.unwrap_or(1.0),
                    stencil: clearing.stencil.unwrap_or(0),
                },
            };
            let vertex_input_binding_descriptions = [
                (
                    vk::VertexInputBindingDescription {
                        stride: (std::mem::size_of::<f32>() * 3) as u32,
                        input_rate: vk::VertexInputRate::VERTEX,
                        binding: crate::shader::ATTRIB_LOC_POSITION,
                    },
                    vk::VertexInputAttributeDescription {
                        location: crate::shader::ATTRIB_LOC_POSITION,
                        binding: crate::shader::ATTRIB_LOC_POSITION,
                        format: vk::Format::R32G32B32_SFLOAT,
                        offset: 0,
                    },
                ),
                (
                    vk::VertexInputBindingDescription {
                        stride: (std::mem::size_of::<f32>() * 3) as u32,
                        input_rate: vk::VertexInputRate::VERTEX,
                        binding: crate::shader::ATTRIB_LOC_NORMAL,
                    },
                    vk::VertexInputAttributeDescription {
                        location: crate::shader::ATTRIB_LOC_NORMAL,
                        binding: crate::shader::ATTRIB_LOC_NORMAL,
                        format: vk::Format::R32G32B32_SFLOAT,
                        offset: 0,
                    },
                ),
                (
                    vk::VertexInputBindingDescription {
                        stride: (std::mem::size_of::<u8>() * 4) as u32,
                        input_rate: vk::VertexInputRate::VERTEX,
                        binding: crate::shader::ATTRIB_LOC_COLOR,
                    },
                    vk::VertexInputAttributeDescription {
                        location: crate::shader::ATTRIB_LOC_COLOR,
                        binding: crate::shader::ATTRIB_LOC_COLOR,
                        format: vk::Format::R8G8B8A8_UINT,
                        offset: 0,
                    },
                ),
                (
                    vk::VertexInputBindingDescription {
                        stride: (std::mem::size_of::<f32>() * 2) as u32,
                        input_rate: vk::VertexInputRate::VERTEX,
                        binding: crate::shader::ATTRIB_LOC_TEXCOORD,
                    },
                    vk::VertexInputAttributeDescription {
                        location: crate::shader::ATTRIB_LOC_TEXCOORD,
                        binding: crate::shader::ATTRIB_LOC_TEXCOORD,
                        format: vk::Format::R32G32_SFLOAT,
                        offset: 0,
                    },
                ),
                (
                    vk::VertexInputBindingDescription {
                        stride: (std::mem::size_of::<u32>() * 1) as u32,
                        input_rate: vk::VertexInputRate::INSTANCE,
                        binding: crate::shader::ATTRIB_LOC_INSTANCE_ID,
                    },
                    vk::VertexInputAttributeDescription {
                        location: crate::shader::ATTRIB_LOC_INSTANCE_ID,
                        binding: crate::shader::ATTRIB_LOC_INSTANCE_ID,
                        format: vk::Format::R32_UINT,
                        offset: 0,
                    },
                ),
            ];
            let binding_descs = vertex_input_binding_descriptions
                .iter()
                .map(|f| f.0)
                .collect::<Vec<_>>();
            let attrib_descs = vertex_input_binding_descriptions
                .iter()
                .map(|f| f.1)
                .collect::<Vec<_>>();
            let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::builder()
                .vertex_binding_descriptions(&binding_descs)
                .vertex_attribute_descriptions(&attrib_descs);
            let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo {
                topology: vk::PrimitiveTopology::TRIANGLE_LIST,
                ..Default::default()
            };

            let dynamic_state = [];
            let dynamic_state_info =
                vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_state);

            let mut rendering_pipeline_info = vk::PipelineRenderingCreateInfo::builder()
                .color_attachment_formats(&[window_format])
                .depth_attachment_format(vk::Format::D16_UNORM)
                .build();

            let pipeline_layout = unsafe {
                device.create_pipeline_layout(&vk::PipelineLayoutCreateInfo::default(), None)
            }
            .unwrap();
            let multisample_state = vk::PipelineMultisampleStateCreateInfo {
                rasterization_samples: vk::SampleCountFlags::TYPE_1,
                ..Default::default()
            };
            let shader_stages = shader_programs
                .iter()
                .find(|s| s.name == pass.program)
                .unwrap()
                .shaders
                .iter()
                .map(|e| e.info)
                .collect::<Vec<_>>();
            let graphic_pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
                .stages(&shader_stages)
                .vertex_input_state(&vertex_input_state_info)
                .input_assembly_state(&vertex_input_assembly_state_info)
                .viewport_state(&viewport_scissor_state)
                .rasterization_state(&rasterization_state)
                .multisample_state(&multisample_state)
                .depth_stencil_state(&depth_stencil_state)
                .color_blend_state(&blend_state)
                .dynamic_state(&dynamic_state_info)
                .layout(pipeline_layout)
                .push_next(&mut rendering_pipeline_info);

            let graphics_pipelines = unsafe {
                device.create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    &[graphic_pipeline_info.build()],
                    None,
                )
            }
            .expect("Unable to create graphics pipeline");
        }

        return pip;
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
