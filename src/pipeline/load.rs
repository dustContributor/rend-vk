use ash::vk::{self, DescriptorType};

use std::{
    collections::{HashMap, HashSet},
    process::Command,
};

use super::{descriptor::DescriptorBuffer, file::*};
use crate::context::VulkanContext;
use crate::pipeline::sampler;
use crate::shader;
use crate::{buffer::DeviceAllocator, pipeline::attachment::Attachment};

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
        device_mem: &mut DeviceAllocator,
        descriptor_mem: &mut DeviceAllocator,
        default_attachment: Attachment,
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
            let name = src_out.0;
            let args = [&format!("shader/{}", name), "-V", "-o", &src_out.1];
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
                let format = f.format.to_vk();
                let texture_create_info = vk::ImageCreateInfo {
                    image_type: vk::ImageType::TYPE_2D,
                    format,
                    extent: extent.into(),
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
                let image = unsafe { ctx.device.create_image(&texture_create_info, None) }.unwrap();
                let texture_memory_req = unsafe { ctx.device.get_image_memory_requirements(image) };
                let texture_allocate_info = vk::MemoryAllocateInfo {
                    allocation_size: texture_memory_req.size,
                    memory_type_index: device_mem.buffer.type_index,
                    ..Default::default()
                };
                let memory = unsafe {
                    ctx.device
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
                    ctx.device
                        .bind_image_memory(image, memory, 0)
                        .expect("failed image memory bind");
                    ctx.device
                        .create_image_view(&image_view_info, None)
                        .expect("failed image view")
                };
                ctx.try_set_debug_name(&f.name, image);
                ctx.try_set_debug_name(&f.name, memory);
                ctx.try_set_debug_name(&f.name, view);
                return (
                    &f.name,
                    Attachment {
                        name: f.name.clone(),
                        format: f.format,
                        vk_format: format,
                        image,
                        memory,
                        view,
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
        let enabled_passes: Vec<_> = pip.passes.into_iter().filter(|e| !e.is_disabled).collect();
        let linear_sampler = sampler::Sampler::of_kind(&ctx, SamplerKind::Linear);
        let nearest_sampler = sampler::Sampler::of_kind(&ctx, SamplerKind::Nearest);
        let mut sampler_descriptors = Self::init_samplers(ctx, descriptor_mem, 2);
        let linear_sampler = sampler::Sampler {
            descriptor_offset: sampler_descriptors
                .place_sampler_at(
                    0,
                    0,
                    linear_sampler.sampler,
                    &ctx.extension.descriptor_buffer,
                )
                .0,
            ..linear_sampler
        };
        let nearest_sampler = sampler::Sampler {
            descriptor_offset: sampler_descriptors
                .place_sampler_at(
                    1,
                    0,
                    nearest_sampler.sampler,
                    &ctx.extension.descriptor_buffer,
                )
                .0,
            ..nearest_sampler
        };
        let mut image_descriptors = Self::init_images(ctx, descriptor_mem);
        let mut stages = Vec::<_>::with_capacity(enabled_passes.len());
        let mut stage_index = 0u32;
        for (passi, pass) in enabled_passes.iter().enumerate() {
            let writing = Self::handle_option(pass.state.writing.clone());
            let depth = Self::handle_option(pass.state.depth.clone());
            let blending = Self::handle_option(pass.state.blending.clone());
            let stencil = Self::handle_option(pass.state.stencil.clone());
            let viewport = Self::handle_option(pass.state.viewport.clone());
            let scissor = Self::handle_option(pass.state.scissor.clone());
            let triangle = Self::handle_option(pass.state.triangle.clone());
            let clearing = Self::handle_option(pass.state.clearing.clone());
            let (_attachments, blend_state) = blending.to_vk();
            let stencil_op_state = stencil.to_vk();
            let depth_stencil_state = depth.to_vk(stencil_op_state, &writing);
            let viewports = [viewport.to_vk(window_width as f32, window_height as f32)];
            let scissors = [scissor.to_vk(window_width as f32, window_height as f32)];
            let viewport_scissor_state = vk::PipelineViewportStateCreateInfo::builder()
                .scissors(&scissors)
                .viewports(&viewports);
            let rasterization_state = triangle.to_vk();

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
                        .expect(&format!("color attachment missing: {e}"))
                        .vk_format
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

            let multisample_state = vk::PipelineMultisampleStateCreateInfo {
                rasterization_samples: vk::SampleCountFlags::TYPE_1,
                ..Default::default()
            };
            let shader_stages = shader_programs_by_name
                .get(&pass.program)
                .expect(format!("Missing program: {}", pass.program).as_str())
                .shaders
                .iter()
                .map(|e| e.info)
                .collect::<Vec<_>>();

            let mut input_descriptors = (pass.inputs.len() > 0).then(|| {
                Box::new(Self::init_inputs(
                    ctx,
                    descriptor_mem,
                    pass.inputs.len() as u32,
                ))
            });

            let clear_color_value = clearing.to_vk_color();
            let clear_depth_stencil_value = clearing.to_vk_depth_stencil();
            let make_attachment_descriptor = |(i, e)| -> Attachment {
                let tmp = attachments_by_name
                    .get(&e)
                    .expect(&format!("Missing input attachment: {e}!"))
                    .clone();
                let desc = vk::DescriptorImageInfo::builder()
                    .image_layout(vk::ImageLayout::READ_ONLY_OPTIMAL)
                    .image_view(tmp.view)
                    .build();
                let (descriptor_offset, descriptor_index) = input_descriptors
                    .as_mut()
                    .unwrap()
                    .place_image(0, desc, &ctx.extension.descriptor_buffer);
                Attachment {
                    descriptor_offset,
                    descriptor_index,
                    ..tmp
                }
            };
            // Final passes have special rendering attachment info hanlding on render.
            let default_attachment_index = pass
                .outputs
                .iter()
                .position(|e| Attachment::DEFAULT_NAME == e);
            let inputs: Vec<_> = pass
                .inputs
                .iter()
                .map(|e| &e.name)
                .enumerate()
                .map(make_attachment_descriptor)
                .collect();
            let outputs: Vec<_> = pass
                .outputs
                .iter()
                .map(|e| -> Attachment {
                    attachments_by_name
                        .get(&e)
                        .expect(&format!("Missing output attachment: {e}!"))
                        .clone()
                })
                .collect();
            let rendering_attachments: Vec<_> = outputs
                .iter()
                .map(|e| vk::RenderingAttachmentInfo {
                    image_view: e.view,
                    image_layout: vk::ImageLayout::ATTACHMENT_OPTIMAL,
                    load_op: clear_color_value
                        .map_or(vk::AttachmentLoadOp::LOAD, |_| vk::AttachmentLoadOp::CLEAR),
                    store_op: vk::AttachmentStoreOp::STORE,
                    clear_value: clear_color_value.unwrap_or_default(),
                    ..Default::default()
                })
                .collect();
            let depth_stencil_rendering = if writing.depth || writing.stencil {
                // Only support depth only, or depth+stencil. No stencil only.
                let depth_name = Attachment::DEPTH_NAME.to_string();
                let depth = attachments_by_name
                    .get(&depth_name)
                    .expect("Missing depth attachment!");
                let depth_info = vk::RenderingAttachmentInfo {
                    image_view: depth.view,
                    image_layout: vk::ImageLayout::ATTACHMENT_OPTIMAL,
                    load_op: clear_depth_stencil_value
                        .map_or(vk::AttachmentLoadOp::DONT_CARE, |_| {
                            vk::AttachmentLoadOp::CLEAR
                        }),
                    clear_value: clear_depth_stencil_value.unwrap_or_default(),
                    ..Default::default()
                };
                Some(depth_info)
            } else {
                None
            };
            let image_barriers =
                Self::gen_image_barriers_for(passi, &inputs, &outputs, &enabled_passes);
            let mut set_layouts = vec![sampler_descriptors.layout];
            if let Some(d) = &input_descriptors {
                set_layouts.push(d.layout)
            }
            let pipeline_layout = unsafe {
                let info = vk::PipelineLayoutCreateInfo::builder()
                    .set_layouts(&set_layouts)
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

            ctx.try_set_debug_name(&pass.name, graphics_pipeline);
            ctx.try_set_debug_name(&pass.name, pipeline_layout);

            if let Some(d) = &mut input_descriptors {
                // If there are any input descriptors, write them into device memory
                d.into_device()
            }
            let mut ubo_descriptors = (pass.updaters.len() > 0).then(|| {
                Box::new(Self::init_ubo_desc_buffer(
                    ctx,
                    descriptor_mem,
                    &pass.name,
                    pass.updaters.len() as u32,
                ))
            });
            stages.push(crate::pipeline::stage::Stage {
                name: pass.name.clone(),
                rendering: super::stage::Rendering {
                    attachments: rendering_attachments,
                    depth_stencil: depth_stencil_rendering,
                    default_attachment_index,
                },
                batch: pass.batch,
                pipeline: graphics_pipeline,
                layout: pipeline_layout,
                updaters: pass.updaters.iter().map(|e| e.to_resource_kind()).collect(),
                ubo_descriptors,
                inputs,
                outputs,
                index: stage_index,
                is_final: default_attachment_index.is_some(),
                image_barriers,
                input_descriptors,
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

        // ubo_descriptors.into_device();
        sampler_descriptors.into_device();
        // image_descriptors.into_device();

        return crate::pipeline::Pipeline {
            stages,
            attachments: attachments_by_name.into_values().collect(),
            linear_sampler,
            nearest_sampler,
            image_descriptors,
            sampler_descriptors,
        };
    }

    pub fn init_ubo_desc_buffer(
        ctx: &VulkanContext,
        mem: &mut DeviceAllocator,
        stage_name: &str,
        bindings: u32,
    ) -> DescriptorBuffer {
        const MAX_SUBSETS: u32 = 64;
        let name = format!("{stage_name}_ubos").to_string();
        let desc_buffer = DescriptorBuffer::of(
            ctx,
            mem,
            name,
            DescriptorType::UNIFORM_BUFFER,
            bindings,
            MAX_SUBSETS,
            false,
        );
        desc_buffer
    }

    pub fn init_images(ctx: &VulkanContext, mem: &mut DeviceAllocator) -> DescriptorBuffer {
        let desc_buffer = DescriptorBuffer::of(
            ctx,
            mem,
            "images".to_string(),
            DescriptorType::SAMPLED_IMAGE,
            1024,
            1,
            true,
        );
        // missing ID/handle generator per image
        desc_buffer
    }

    pub fn init_inputs(
        ctx: &VulkanContext,
        mem: &mut DeviceAllocator,
        size: u32,
    ) -> DescriptorBuffer {
        let desc_buffer = DescriptorBuffer::of(
            ctx,
            mem,
            "targets".to_string(),
            DescriptorType::SAMPLED_IMAGE,
            size,
            1,
            false,
        );
        desc_buffer
    }

    pub fn init_samplers(
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

    fn gen_image_barriers_for(
        currenti: usize,
        inputs: &Vec<Attachment>,
        outputs: &Vec<Attachment>,
        passes: &Vec<Pass>,
    ) -> Vec<vk::ImageMemoryBarrier2> {
        let mut i = currenti;
        let mut barriers: Vec<vk::ImageMemoryBarrier2> = Vec::new();
        fn wrap_around(index: usize, length: usize) -> usize {
            if index == 0 {
                length - 1
            } else {
                index - 1
            }
        }
        for input in inputs {
            if Attachment::DEFAULT_NAME == input.name {
                panic!("Can't read from the default attachment!")
            }
            loop {
                i = wrap_around(i, passes.len());
                if i == currenti {
                    // Looped back to current pass, nothing to check
                    break;
                }
                let prev = &passes[i];
                if prev.inputs.iter().any(|e| e.name.eq(&input.name)) {
                    // Already issued barrier before
                    break;
                }
                if !prev.outputs.contains(&input.name) {
                    // Continue to previous pass
                    continue;
                }
                // Image was written to before, barrier for reading
                let barrier = vk::ImageMemoryBarrier2::builder()
                    .image(input.image)
                    .src_access_mask(vk::AccessFlags2::MEMORY_WRITE)
                    .dst_access_mask(vk::AccessFlags2::MEMORY_READ)
                    .old_layout(vk::ImageLayout::ATTACHMENT_OPTIMAL)
                    .new_layout(vk::ImageLayout::READ_ONLY_OPTIMAL)
                    .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                    .dst_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
                    .subresource_range(Attachment::color_subresource_range())
                    .build();
                barriers.push(barrier);
                break;
            }
        }
        for output in outputs {
            if Attachment::DEFAULT_NAME == output.name {
                /*
                 * Handled in the rendering loop, since the swapchain
                 * changes which image this barrier refers to.
                 */
                continue;
            }
            loop {
                i = wrap_around(i, passes.len());
                if i == currenti {
                    // Looped back to current pass, nothing to check
                    break;
                }
                let prev = &passes[i];
                if prev.outputs.contains(&output.name) {
                    // Already issued barrier before
                    break;
                }
                if !prev.inputs.iter().any(|e| e.name.eq(&output.name)) {
                    // Continue to previous pass
                    continue;
                }
                // Image was read before, issue barrier for writing
                let barrier = vk::ImageMemoryBarrier2::builder()
                    .image(output.image)
                    .src_access_mask(vk::AccessFlags2::MEMORY_READ)
                    .dst_access_mask(vk::AccessFlags2::MEMORY_WRITE)
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(vk::ImageLayout::ATTACHMENT_OPTIMAL)
                    .src_stage_mask(vk::PipelineStageFlags2::NONE)
                    .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                    .subresource_range(Attachment::color_subresource_range())
                    .build();
                barriers.push(barrier);
                break;
            }
        }
        return barriers;
    }

    fn mask_layout_aspect_for(
        format: crate::format::Format,
    ) -> (vk::AccessFlags, vk::ImageLayout, vk::ImageAspectFlags) {
        if format.has_depth() {
            let (layout, aspect) = if format.has_stencil() {
                (
                    vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                    vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL,
                )
            } else {
                (
                    vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL,
                    vk::ImageAspectFlags::DEPTH,
                )
            };
            (
                vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
                layout,
                aspect,
            )
        } else {
            (
                vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                vk::ImageAspectFlags::COLOR,
            )
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
            // (
            //     vk::VertexInputBindingDescription {
            //         stride: (std::mem::size_of::<f32>() * 3) as u32,
            //         input_rate: vk::VertexInputRate::VERTEX,
            //         binding: crate::shader::ATTRIB_LOC_NORMAL,
            //     },
            //     vk::VertexInputAttributeDescription {
            //         location: crate::shader::ATTRIB_LOC_NORMAL,
            //         binding: crate::shader::ATTRIB_LOC_NORMAL,
            //         format: vk::Format::R32G32B32_SFLOAT,
            //         offset: 0,
            //     },
            // ),
            (
                vk::VertexInputBindingDescription {
                    stride: (std::mem::size_of::<u8>() * 4) as u32,
                    input_rate: vk::VertexInputRate::VERTEX,
                    binding: crate::shader::ATTRIB_LOC_COLOR,
                },
                vk::VertexInputAttributeDescription {
                    location: crate::shader::ATTRIB_LOC_COLOR,
                    binding: crate::shader::ATTRIB_LOC_COLOR,
                    format: vk::Format::R32G32B32_SFLOAT,
                    offset: 0,
                },
            ),
            // (
            //     vk::VertexInputBindingDescription {
            //         stride: (std::mem::size_of::<f32>() * 2) as u32,
            //         input_rate: vk::VertexInputRate::VERTEX,
            //         binding: crate::shader::ATTRIB_LOC_TEXCOORD,
            //     },
            //     vk::VertexInputAttributeDescription {
            //         location: crate::shader::ATTRIB_LOC_TEXCOORD,
            //         binding: crate::shader::ATTRIB_LOC_TEXCOORD,
            //         format: vk::Format::R32G32_SFLOAT,
            //         offset: 0,
            //     },
            // ),
            // (
            //     vk::VertexInputBindingDescription {
            //         stride: (std::mem::size_of::<u32>() * 1) as u32,
            //         input_rate: vk::VertexInputRate::INSTANCE,
            //         binding: crate::shader::ATTRIB_LOC_INSTANCE_ID,
            //     },
            //     vk::VertexInputAttributeDescription {
            //         location: crate::shader::ATTRIB_LOC_INSTANCE_ID,
            //         binding: crate::shader::ATTRIB_LOC_INSTANCE_ID,
            //         format: vk::Format::R32_UINT,
            //         offset: 0,
            //     },
            // ),
        ]
    }
}
