use ash::vk;

use std::{
    collections::{HashMap, HashSet},
    process::Command,
};

use super::file::*;
use crate::pipeline::Attachment;
use crate::shader;

impl Pipeline {
    pub fn read(name: Option<&str>) -> Self {
        let name = name.unwrap_or("pipeline.json");
        let file = std::fs::File::open(name).expect("failed opening the pipeline");
        return serde_json::from_reader(file).expect("couldn't parse the pipeline");
    }

    pub fn load(
        device: &ash::Device,
        memory_type_index: u32,
        window_format: vk::Format,
        window_width: u32,
        window_height: u32,
        name: Option<&str>,
    ) -> crate::pipeline::Pipeline {
        let pip = Self::read(name);
        let shaders_by_name: HashMap<_, _> = pip
            .programs
            .iter()
            .map(|p| vec![&p.fragment, &p.vertex, &p.geometry])
            .flatten()
            .filter(|f| !f.is_empty())
            // Same shader could be used in multiple programs.
            .collect::<HashSet<_>>()
            .iter()
            .map(|f| (f.clone(), format!("shader/{f}.spv")))
            .collect();
        for src_out in &shaders_by_name {
            Command::new("glslangValidator")
                .args([&format!("shader/{}", src_out.0), "-V", "-o", &src_out.1])
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
        let shader_programs_by_name: HashMap<_, _> = pip
            .programs
            .iter()
            .map(|f| {
                (
                    &f.name,
                    shader::ShaderProgram::new(
                        &device,
                        f.name.clone(),
                        load_shader(&f.vertex),
                        load_shader(&f.fragment),
                        load_shader(&f.geometry),
                    ),
                )
            })
            .collect();
        let attachments_by_name: HashMap<_, _> = pip
            .targets
            .iter()
            .map(|f| {
                let extent = Self::extent_of(f.width, f.height, window_width, window_height).into();
                let format = f.format.to_vk();
                let texture_create_info = vk::ImageCreateInfo {
                    image_type: vk::ImageType::TYPE_2D,
                    format,
                    extent,
                    mip_levels: 1,
                    array_layers: 1,
                    samples: vk::SampleCountFlags::TYPE_1,
                    tiling: vk::ImageTiling::OPTIMAL,
                    usage: if f.format.has_depth() {
                        vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT
                    } else {
                        vk::ImageUsageFlags::COLOR_ATTACHMENT
                    } | vk::ImageUsageFlags::SAMPLED,
                    sharing_mode: vk::SharingMode::EXCLUSIVE,
                    ..Default::default()
                };
                let image = unsafe { device.create_image(&texture_create_info, None) }.unwrap();
                let texture_memory_req = unsafe { device.get_image_memory_requirements(image) };
                let texture_allocate_info = vk::MemoryAllocateInfo {
                    allocation_size: texture_memory_req.size,
                    memory_type_index,
                    ..Default::default()
                };
                let memory = unsafe {
                    device
                        .allocate_memory(&texture_allocate_info, None)
                        .expect("failed image memory alloc")
                };
                let image_view_info = vk::ImageViewCreateInfo::builder()
                    .subresource_range(
                        vk::ImageSubresourceRange::builder()
                            .aspect_mask(
                                if f.format.has_depth() {
                                    vk::ImageAspectFlags::DEPTH
                                } else {
                                    vk::ImageAspectFlags::NONE
                                } | if f.format.has_stencil() {
                                    vk::ImageAspectFlags::STENCIL
                                } else {
                                    vk::ImageAspectFlags::NONE
                                } | if f.format.has_depth() || f.format.has_stencil() {
                                    vk::ImageAspectFlags::NONE
                                } else {
                                    vk::ImageAspectFlags::COLOR
                                },
                            )
                            .level_count(1)
                            .layer_count(1)
                            .build(),
                    )
                    .image(image)
                    .format(format)
                    .view_type(vk::ImageViewType::TYPE_2D);
                let view = unsafe {
                    device
                        .bind_image_memory(image, memory, 0)
                        .expect("failed image memory bind");
                    device
                        .create_image_view(&image_view_info, None)
                        .expect("failed image view")
                };
                return (
                    &f.name,
                    Attachment {
                        name: f.name.clone(),
                        format: f.format,
                        vk_format: format,
                        image,
                        memory,
                        view,
                    },
                );
            })
            .collect();
        let mut stages = Vec::<_>::with_capacity(pip.passes.len());
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
                extent: Self::extent_of(scissor.width, scissor.height, window_width, window_height),
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

            let vertex_input_binding_descriptions = Self::default_vertex_inputs();
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

            let dynamic_state_info =
                vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&[]);

            let color_attachment_formats: Vec<_> = pass
                .outputs
                .iter()
                .map(|e| {
                    attachments_by_name
                        .get(e)
                        .map_or_else(
                            || {
                                if e == Attachment::DEFAULT_NAME {
                                    // Default attachment uses the window format.
                                    Some(window_format)
                                } else {
                                    // Output attachment not found, fail.
                                    None
                                }
                            },
                            |e| Some(e.vk_format),
                        )
                        .expect(&format!("color attachment missing: {e}"))
                })
                .collect::<Vec<vk::Format>>();

            let mut rendering_pipeline_info = {
                let mut b = vk::PipelineRenderingCreateInfo::builder()
                    .color_attachment_formats(&color_attachment_formats);
                if writing.depth {
                    let depth_name = Attachment::DEPTH_NAME.to_string();
                    // Depth is special cased.
                    b = b.depth_attachment_format(
                        attachments_by_name.get(&depth_name).unwrap().vk_format,
                    );
                }
                b.build()
            };

            let pipeline_layout = unsafe {
                device.create_pipeline_layout(&vk::PipelineLayoutCreateInfo::default(), None)
            }
            .unwrap();
            let multisample_state = vk::PipelineMultisampleStateCreateInfo {
                rasterization_samples: vk::SampleCountFlags::TYPE_1,
                ..Default::default()
            };
            let shader_stages = shader_programs_by_name
                .get(&pass.program)
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
            let graphics_pipeline = graphics_pipelines[0];

            let clear_color_value = clearing.color.and_then(|e| {
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
            });
            let clear_depth_stencil_value =
                if clearing.depth.is_some() || clearing.stencil.is_some() {
                    Some(vk::ClearValue {
                        depth_stencil: vk::ClearDepthStencilValue {
                            depth: clearing.depth.unwrap_or(0.0),
                            stencil: clearing.stencil.unwrap_or(0),
                        },
                    })
                } else {
                    None
                };
            let get_attachment = |e: &String| -> Option<Attachment> {
                attachments_by_name.get(&e).map_or_else(
                    || {
                        if e == Attachment::DEFAULT_NAME {
                            // Skip over the default attachment.
                            None
                        } else {
                            // Name was specified and it isn't the default, fail.
                            panic!("missing attachment: {e}")
                        }
                    },
                    |e| Some(e.clone()),
                )
            };
            // Final passes have special rendering attachment info hanlding on render.
            let is_final = pass
                .outputs
                .iter()
                .find(|&e| Attachment::DEFAULT_NAME == e)
                .is_some();
            let inputs: Vec<_> = pass.inputs.iter().map(get_attachment).flatten().collect();
            let outputs: Vec<_> = pass.outputs.iter().map(get_attachment).flatten().collect();
            let pre_rendering_color: Vec<_> = outputs
                .iter()
                .map(|e| vk::RenderingAttachmentInfo {
                    image_view: e.view,
                    image_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    load_op: clear_color_value.map_or(vk::AttachmentLoadOp::DONT_CARE, |_| {
                        vk::AttachmentLoadOp::CLEAR
                    }),
                    store_op: vk::AttachmentStoreOp::STORE,
                    clear_value: clear_color_value.unwrap_or_default(),
                    ..Default::default()
                })
                .collect();

            let pre_rendering_builder =
                vk::RenderingInfo::builder().color_attachments(&pre_rendering_color);
            let pre_rendering = match outputs.iter().find(|e| e.format.has_depth()) {
                Some(depth) => {
                    // Only support depth only, or depth+stencil. No stencil only.
                    let depth_info = vk::RenderingAttachmentInfo {
                        image_view: depth.view,
                        image_layout: if depth.format.has_stencil() {
                            vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL
                        } else {
                            vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL
                        },
                        load_op: clear_depth_stencil_value
                            .map_or(vk::AttachmentLoadOp::DONT_CARE, |_| {
                                vk::AttachmentLoadOp::CLEAR
                            }),
                        clear_value: clear_depth_stencil_value.unwrap_or_default(),
                        ..Default::default()
                    };
                    pre_rendering_builder.depth_attachment(&depth_info).build()
                }
                _ => pre_rendering_builder.build(),
            };

            stages.push(crate::pipeline::Stage {
                name: pass.name.clone(),
                pre_rendering,
                batch: pass.batch,
                pipeline: graphics_pipeline,
                layout: pipeline_layout,
                updaters: pass.updaters.clone(),
                inputs,
                outputs,
                is_final,
            });
        }
        for shader in shader_programs_by_name
            .into_values()
            .flat_map(|e| e.shaders)
        {
            // No longer need them.
            unsafe { device.destroy_shader_module(shader.info.module, None) };
        }

        return crate::pipeline::Pipeline {
            stages,
            attachments: attachments_by_name.into_values().collect(),
        };
    }

    fn extent_of(
        opt_width: U32OrF32,
        opt_height: U32OrF32,
        ref_width: u32,
        ref_height: u32,
    ) -> vk::Extent2D {
        vk::Extent2D {
            width: match opt_width {
                U32OrF32::U32(v) => v,
                U32OrF32::F32(v) => (ref_width as f32 * v).ceil() as u32,
            },
            height: match opt_height {
                U32OrF32::U32(v) => v,
                U32OrF32::F32(v) => (ref_height as f32 * v).ceil() as u32,
            },
        }
    }

    fn default_vertex_inputs() -> Vec<(
        vk::VertexInputBindingDescription,
        vk::VertexInputAttributeDescription,
    )> {
        vec![
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
        ]
    }
}
