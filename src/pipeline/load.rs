use ash::vk::{self, DescriptorType, ShaderStageFlags};

use std::{
    collections::{HashMap, HashSet},
    process::Command,
};

use super::{
    barrier_gen::BarrierGen,
    descriptor::DescriptorBuffer,
    file::*,
    sampler::{Sampler, SamplerKey},
};
use crate::shader;
use crate::texture::MipMap;
use crate::{buffer::DeviceAllocator, pipeline::attachment::Attachment, renderer::Renderer};
use crate::{context::VulkanContext, texture};

impl Pipeline {
    pub fn read(name: Option<&str>) -> Self {
        let name = name.unwrap_or("pipeline.json");
        let file = std::fs::File::open(name)
            .expect(format!("failed opening the pipeline at {}", name).as_str());
        return serde_json::from_reader(file)
            .expect(format!("couldn't parse the pipeline at {}", name).as_str());
    }

    pub fn load(
        ctx: &VulkanContext,
        descriptor_mem: &mut DeviceAllocator,
        default_attachment: Attachment,
        is_validation_layer_enabled: bool,
        name: Option<&str>,
    ) -> crate::pipeline::Pipeline {
        let pip = Self::read(name);
        let barrier_gen = BarrierGen::new(&pip.passes);
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
            let name = src_out.0;
            // Some flags so the various macros work
            let args = [
                &format!("shader/{}", name),
                "-V",
                "-DIS_VULKAN=1",
                "-DIS_EXTERNAL_COMPILER=1",
                "--glsl-version",
                "460",
                "-o",
                &src_out.1,
            ];
            log::info!("compiling shader {} with args {:?}...", name, args);
            let res = Command::new("glslangValidator")
                .args(args)
                .spawn()
                .expect(format!("Failed to start {}", &name).as_str())
                // TODO: Could launch all of these these concurrently and wait for them all.
                .wait();
            if res.is_err() {
                panic!(
                    "Error compiling shader {}, error {}",
                    name,
                    res.unwrap_err()
                )
            }
            match res {
                Err(e) => {
                    panic!("Error compiling shader {}, error {}", name, e)
                }
                Ok(e) if !e.success() => {
                    panic!("Failed compiling shader {}, error {}", name, e);
                }
                _ => {}
            }
            log::info!("shader {} compiled!", name);
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
                        &ctx.device,
                        f.name.clone(),
                        load_shader(&f.vertex),
                        load_shader(&f.fragment),
                        load_shader(&f.geometry),
                    ),
                )
            })
            .collect();
        let window_width = default_attachment.extent.width;
        let window_height = default_attachment.extent.height;
        let mut attachments_by_name: HashMap<_, _> = pip
            .targets
            .iter()
            .map(|f| {
                let extent =
                    Self::extent_of(f.width, f.height, window_width as f32, window_height as f32);
                let texture = texture::make(
                    &ctx,
                    0,
                    f.name.clone(),
                    &[MipMap {
                        width: extent.width,
                        height: extent.height,
                        ..Default::default()
                    }],
                    f.format,
                    true,
                    None,
                );

                ctx.try_set_debug_name(&format!("{}_{}", f.name, "image"), texture.image);
                ctx.try_set_debug_name(&format!("{}_{}", f.name, "memory"), texture.memory);
                ctx.try_set_debug_name(&format!("{}_{}", f.name, "view"), texture.view);
                return (
                    &f.name,
                    Attachment {
                        name: f.name.clone(),
                        format: f.format,
                        vk_format: f.format.to_vk(),
                        image: texture.image,
                        memory: texture.memory,
                        view: texture.view,
                        extent,
                        descriptor_offset: 0,
                        descriptor_index: 0,
                    },
                );
            })
            .collect();
        let default_attachment_name = Attachment::DEFAULT_NAME.to_string();
        // Default attachment is provided by the caller since it depends on the swapchain.
        attachments_by_name.insert(&default_attachment_name, default_attachment);
        // If there are no inputs whatsoever, just use a dummy one sized buffer.
        let enabled_passes: Vec<_> = pip.passes.iter().filter(|e| !e.is_disabled()).collect();
        let image_descriptors = Self::image_desc_buffer(ctx, descriptor_mem);
        let mut sampler_descriptors =
            Self::sampler_desc_buffer(ctx, descriptor_mem, Renderer::MAX_SAMPLERS);

        let mut samplers_by_key: HashMap<SamplerKey, Sampler> = HashMap::new();

        let mut stages = Vec::<_>::with_capacity(enabled_passes.len());
        let mut stage_index = 0u32;
        for (passi, pass) in enabled_passes.iter().enumerate() {
            let render_pass;
            match pass {
                PipelineStep::Render(tmp) => render_pass = tmp,
                PipelineStep::Blit(_) => continue,
            };
            let writing = Self::handle_option(render_pass.state.writing.clone());
            let depth = Self::handle_option(render_pass.state.depth.clone());
            let blending = Self::handle_option(render_pass.state.blending.clone());
            let stencil = Self::handle_option(render_pass.state.stencil.clone());
            let viewport = Self::handle_option(render_pass.state.viewport.clone());
            let scissor = Self::handle_option(render_pass.state.scissor.clone());
            let triangle = Self::handle_option(render_pass.state.triangle.clone());
            let clearing = Self::handle_option(render_pass.state.clearing.clone());
            let stencil_op_state = stencil.to_vk();
            let depth_stencil_state = depth.to_vk(stencil_op_state, &writing);
            let viewports = [viewport.to_vk(&depth, window_width as f32, window_height as f32)];
            let scissors = [scissor.to_vk(window_width as f32, window_height as f32)];
            let viewport_scissor_state = vk::PipelineViewportStateCreateInfo::builder()
                .scissors(&scissors)
                .viewports(&viewports);
            let rasterization_state = triangle.to_vk();
            let depth_stencil_attachment = match &render_pass.depth_stencil {
                Some(name) => Some(attachments_by_name.get(&name.to_string()).expect(&format!(
                    "depth stencil attachment {} missing for pass {}!",
                    name, render_pass.name
                ))),
                _ => None,
            };
            let binding_descs = [];
            let attrib_descs = [];
            let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::builder()
                .vertex_binding_descriptions(&binding_descs)
                .vertex_attribute_descriptions(&attrib_descs);
            let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo {
                topology: vk::PrimitiveTopology::TRIANGLE_LIST,
                ..Default::default()
            };

            let dynamic_state_info =
                vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&[]);
            // TODO: Check why if depth output isn't placed last, VVL errors get reported
            let attachment_outputs: Vec<_> = render_pass
                .outputs
                .iter()
                .map(|e| {
                    attachments_by_name
                        .get(e)
                        .expect(&format!("output attachment {e} missing!"))
                        .clone()
                })
                .collect();
            let attachment_inputs: Vec<_> = render_pass
                .inputs
                .iter()
                .map(|e| {
                    attachments_by_name
                        .get(&e.name)
                        .expect(&format!("input attachment {} missing!", e.name))
                        .clone()
                })
                .collect();
            let attachment_samplers: Vec<_> = render_pass
                .inputs
                .iter()
                .map(|i| {
                    let key = SamplerKey {
                        filter: i.sampler,
                        wrap_mode: WrapMode::ClampToEdge,
                        anisotropy: 1u8,
                    };
                    match samplers_by_key.get(&key) {
                        Some(s) => s.clone(),
                        None => {
                            let name = format!("sampler_{}", i.name);
                            let smp = Sampler::of_key(ctx, name, key, samplers_by_key.len() as u8);
                            samplers_by_key.insert(key, smp.clone());
                            smp
                        }
                    }
                })
                .collect();
            let attachment_output_formats: Vec<_> =
                attachment_outputs.iter().map(|e| e.vk_format).collect();
            // We only need blend state for color attachments, ignoring depth/stencil
            let (_attachments, blend_state) =
                blending.to_vk(attachment_output_formats.len() as u32);

            let mut rendering_pipeline_info = {
                let mut b = vk::PipelineRenderingCreateInfo::builder()
                    .color_attachment_formats(&attachment_output_formats);
                if writing.stencil || !stencil.disabled {
                    let att = depth_stencil_attachment.expect(&format!(
                        "stencil attachment for writing/testing not set for pass {}!",
                        render_pass.name
                    ));
                    b = b.stencil_attachment_format(att.vk_format);
                }
                if writing.depth || depth.testing {
                    let att = depth_stencil_attachment.expect(&format!(
                        "depth attachment for writing/testing not set for pass {}!",
                        render_pass.name
                    ));
                    b = b.depth_attachment_format(att.vk_format);
                }
                b.build()
            };

            let multisample_state = vk::PipelineMultisampleStateCreateInfo {
                rasterization_samples: vk::SampleCountFlags::TYPE_1,
                ..Default::default()
            };
            let shader_stages = shader_programs_by_name
                .get(&render_pass.program)
                .expect(&format!("program {} missing!", render_pass.program))
                .shaders
                .iter()
                .map(|e| e.info)
                .collect::<Vec<_>>();

            let mut attachment_descriptors = (render_pass.inputs.len() > 0).then(|| {
                Box::new(Self::attachment_image_desc_buffer(
                    ctx,
                    descriptor_mem,
                    &render_pass.name,
                    render_pass.inputs.len() as u32,
                ))
            });

            let clear_color_value = clearing.to_vk_color();
            let clear_depth_stencil_value = clearing.to_vk_depth_stencil();
            let make_attachment_descriptor = |e: (&Attachment, &Sampler)| -> Attachment {
                let desc = vk::DescriptorImageInfo::builder()
                    .image_layout(vk::ImageLayout::READ_ONLY_OPTIMAL)
                    .image_view(e.0.view)
                    .sampler(e.1.sampler)
                    .build();
                let (descriptor_offset, descriptor_index) = attachment_descriptors
                    .as_mut()
                    .unwrap()
                    .place_image_sampler(0, desc, &ctx.extension.descriptor_buffer);
                Attachment {
                    descriptor_offset,
                    descriptor_index,
                    ..e.0.clone()
                }
            };
            // Final passes have special rendering attachment info hanlding on render.
            let default_attachment_index = render_pass
                .outputs
                .iter()
                .position(|e| Attachment::DEFAULT_NAME == e);

            // Generate attachment structs with the proper descriptor index/offset
            let inputs: Vec<_> = attachment_inputs
                .iter()
                .zip(attachment_samplers.iter())
                .map(make_attachment_descriptor)
                .collect();

            let make_rendering_attachment_info = |e: &Attachment| vk::RenderingAttachmentInfo {
                image_view: e.view,
                image_layout: vk::ImageLayout::ATTACHMENT_OPTIMAL,
                load_op: if e.format.has_depth_or_stencil() {
                    clear_depth_stencil_value
                        .map_or(vk::AttachmentLoadOp::LOAD, |_| vk::AttachmentLoadOp::CLEAR)
                } else {
                    clear_color_value
                        .map_or(vk::AttachmentLoadOp::LOAD, |_| vk::AttachmentLoadOp::CLEAR)
                },
                clear_value: if e.format.has_depth_or_stencil() {
                    clear_depth_stencil_value.unwrap_or_default()
                } else {
                    clear_color_value.unwrap_or_default()
                },
                store_op: vk::AttachmentStoreOp::STORE,
                ..Default::default()
            };

            let attachment_rendering: Vec<_> = attachment_outputs
                .iter()
                .map(make_rendering_attachment_info)
                .collect();
            let depth_stencil_rendering = match depth_stencil_attachment {
                Some(att) => Some(make_rendering_attachment_info(att)),
                None => None,
            };
            /*
             * Add the depth-stencil attachment to the output list if present,
             * this way proper barriers for writing/testing will be generated if
             * the attachment is read from in a previous pass as an input.
             */
            let mut outputs_for_barriers = attachment_outputs.clone();
            if writing.depth || writing.stencil {
                if let Some(att) = depth_stencil_attachment {
                    outputs_for_barriers.push(att.clone())
                };
            }
            let image_barriers =
                barrier_gen.gen_image_barriers_for(passi, &inputs, &outputs_for_barriers);
            let mut set_layouts = vec![sampler_descriptors.layout, image_descriptors.layout];
            if let Some(d) = &attachment_descriptors {
                set_layouts.push(d.layout)
            }
            let pipeline_layout = unsafe {
                let push_constant_ranges = [vk::PushConstantRange::builder()
                    .offset(0)
                    .size(128)
                    .stage_flags(ShaderStageFlags::ALL_GRAPHICS)
                    .build()];
                let info = vk::PipelineLayoutCreateInfo::builder()
                    .set_layouts(&set_layouts)
                    .push_constant_ranges(&push_constant_ranges)
                    .build();
                ctx.device.create_pipeline_layout(&info, None)
            }
            .unwrap();
            let graphic_pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
                .flags(vk::PipelineCreateFlags::DESCRIPTOR_BUFFER_EXT)
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
                .push_next(&mut rendering_pipeline_info)
                .build();

            let graphics_pipelines = unsafe {
                ctx.device.create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    &[graphic_pipeline_info],
                    None,
                )
            }
            .expect("Unable to create graphics pipeline");
            let graphics_pipeline = graphics_pipelines[0];

            ctx.try_set_debug_name(&render_pass.name, graphics_pipeline);
            ctx.try_set_debug_name(&render_pass.name, pipeline_layout);

            if let Some(d) = &mut attachment_descriptors {
                // If there are any input descriptors, write them into device memory
                d.into_device()
            }
            stages.push(crate::pipeline::stage::Stage {
                name: render_pass.name.clone(),
                is_validation_layer_enabled,
                rendering: super::stage::Rendering {
                    attachments: attachment_rendering,
                    depth_stencil: depth_stencil_rendering,
                    default_attachment_index,
                },
                task_kind: render_pass.batch,
                pipeline: graphics_pipeline,
                layout: pipeline_layout,
                per_instance_updaters: render_pass
                    .per_instance_updaters
                    .iter()
                    .map(|e| e.to_resource_kind())
                    .collect(),
                per_pass_updaters: render_pass
                    .per_pass_updaters
                    .iter()
                    .map(|e| e.to_resource_kind())
                    .collect(),
                inputs,
                outputs: attachment_outputs,
                index: stage_index,
                is_final: default_attachment_index.is_some(),
                image_barriers,
                attachment_descriptors,
                reserved_buffers: Vec::new(),
            });
            // Increment for next stage
            stage_index += 1;
        }
        for shader in shader_programs_by_name
            .into_values()
            .flat_map(|e| e.shaders)
        {
            // No longer need them.
            unsafe { ctx.device.destroy_shader_module(shader.info.module, None) };
        }

        //  Place all sampler descriptors into the descriptor buffer and write to the GPU
        let mut positioned_samplers = samplers_by_key.values().collect::<Vec<_>>();
        positioned_samplers.sort_by(|a, b| a.position.cmp(&b.position));
        for sampler in positioned_samplers {
            sampler_descriptors.place_sampler_at(
                0,
                sampler.position as u32,
                sampler.sampler,
                &ctx.extension.descriptor_buffer,
            );
        }
        sampler_descriptors.into_device();
        // image_descriptors.into_device();

        return crate::pipeline::Pipeline {
            stages,
            attachments: attachments_by_name.into_values().collect(),
            image_descriptors,
            sampler_descriptors,
            samplers_by_key,
        };
    }

    pub fn image_desc_buffer(ctx: &VulkanContext, mem: &mut DeviceAllocator) -> DescriptorBuffer {
        let desc_buffer = DescriptorBuffer::of(
            ctx,
            mem,
            "images".to_string(),
            DescriptorType::SAMPLED_IMAGE,
            1024,
            1,
            true,
        );
        desc_buffer
    }

    pub fn attachment_image_desc_buffer(
        ctx: &VulkanContext,
        mem: &mut DeviceAllocator,
        prefix: &str,
        size: u32,
    ) -> DescriptorBuffer {
        let name = format!("{}_attachments", prefix);
        let desc_buffer = DescriptorBuffer::of(
            ctx,
            mem,
            name,
            DescriptorType::COMBINED_IMAGE_SAMPLER,
            size,
            1,
            false,
        );
        desc_buffer
    }

    pub fn sampler_desc_buffer(
        ctx: &VulkanContext,
        mem: &mut DeviceAllocator,
        size: u32,
    ) -> DescriptorBuffer {
        let desc_buffer = DescriptorBuffer::of(
            ctx,
            mem,
            "samplers".to_string(),
            DescriptorType::SAMPLER,
            size,
            1,
            false,
        );
        desc_buffer
    }

    pub fn extent_of(
        opt_width: U32OrF32,
        opt_height: U32OrF32,
        ref_width: f32,
        ref_height: f32,
    ) -> vk::Extent2D {
        vk::Extent2D {
            width: match opt_width {
                U32OrF32::U32(v) => v,
                U32OrF32::F32(v) => (ref_width * v).ceil() as u32,
            },
            height: match opt_height {
                U32OrF32::U32(v) => v,
                U32OrF32::F32(v) => (ref_height * v).ceil() as u32,
            },
        }
    }
}
