use ash::vk::{self, DescriptorType, ShaderStageFlags};

use std::{
    collections::{HashMap, HashSet},
    process::Command,
};

use super::{
    barrier_gen::BarrierGen,
    descriptor::DescriptorGroup,
    file::*,
    sampler::{Sampler, SamplerKey},
    stage::Stage,
};
use crate::shader;
use crate::texture::MipMap;
use crate::{context::VulkanContext, texture};
use crate::{pipeline::attachment::Attachment, renderer::Renderer};

impl Pipeline {
    pub fn read(name: Option<&str>) -> Self {
        let name = name.unwrap_or("pipeline.json");
        let file = std::fs::File::open(name)
            .expect(format!("failed opening the pipeline at {}", name).as_str());
        return serde_json::from_reader(file)
            .expect(format!("couldn't parse the pipeline at {}", name).as_str());
    }

    fn spirv_path_of(shader: &str) -> String {
        format!("shader/vk/{}.spv", shader)
    }

    fn source_path_of(shader: &str) -> String {
        format!("shader/{}", shader)
    }

    fn compile_shader_programs(
        ctx: &VulkanContext,
        pip: &Pipeline,
    ) -> HashMap<String, shader::ShaderProgram> {
        // Create dest folder for all of the SPIR-V binaries
        let base_path = Self::spirv_path_of("tmp");
        let base_path = std::path::Path::new(&base_path).parent().unwrap();
        std::fs::DirBuilder::new()
            .recursive(true)
            .create(&base_path)
            .expect(&format!(
                "failed creating the SPIR-V folder at {}!",
                base_path.to_str().unwrap()
            ));
        // Same shader could be used in multiple programs, flatten and de-duplicate
        let shaders = pip
            .programs
            .iter()
            .map(|p| vec![&p.fragment, &p.vertex, &p.geometry])
            .flatten()
            .filter(|f| !f.is_empty())
            .collect::<HashSet<_>>();
        // Invoke glslang and compile each shader into SPIR-V
        for shader in shaders {
            let spirv_path = Self::spirv_path_of(shader);
            let source_path = Self::source_path_of(shader);
            // Some flags so the various macros work
            let args = [
                &source_path,
                "-V",
                "-DIS_VULKAN=1",
                "-DIS_EXTERNAL_COMPILER=1",
                "--glsl-version",
                "460",
                "-o",
                &spirv_path,
            ];
            // TODO: Could launch all of these these concurrently and wait for them all.
            log::info!("compiling shader {} with args {:?}...", shader, args);
            let output = Command::new("glslangValidator")
                .args(args)
                .output()
                .expect(format!("failed to start compiler for {}!", &shader).as_str());

            if !output.status.success() {
                let msg = String::from_utf8_lossy(if output.stdout.len() > 0 {
                    &output.stdout
                } else {
                    &output.stderr
                });
                panic!(
                    "error compiling! shader: {}, status: {}, error: {}",
                    shader, output.status, msg
                )
            }
            log::info!("shader {} compiled!", shader);
        }

        let load_spirv = |name: &str| match name.is_empty() {
            true => None,
            false => {
                let path = Self::spirv_path_of(name);
                let mut file =
                    std::fs::File::open(&path).expect(&format!("spirv {path} failed to open!"));
                let bin = ash::util::read_spv(&mut file)
                    .expect(&format!("spirv {} failed to load!", path));
                Some((name.to_string(), bin))
            }
        };

        let programs_by_name: HashMap<_, _> = pip
            .programs
            .iter()
            .map(|p| {
                (
                    p.name.clone(),
                    shader::ShaderProgram::new(
                        ctx,
                        p.name.clone(),
                        load_spirv(&p.vertex),
                        load_spirv(&p.fragment),
                        load_spirv(&p.geometry),
                    ),
                )
            })
            .collect();

        return programs_by_name;
    }

    pub fn load(
        ctx: &VulkanContext,
        default_attachment: Attachment,
        is_validation_layer_enabled: bool,
        name: Option<&str>,
    ) -> crate::pipeline::Pipeline {
        let pip = Self::read(name);
        let barrier_gen = BarrierGen::new(&pip.passes);
        let shader_programs_by_name = Self::compile_shader_programs(ctx, &pip);

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
                ctx.try_set_debug_name(&format!("{}_att_image", f.name), texture.image);
                ctx.try_set_debug_name(&format!("{}_att_image_memory", f.name), texture.memory);
                ctx.try_set_debug_name(&format!("{}_att_image_view", f.name), texture.view);
                return (
                    f.name.clone(),
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
        attachments_by_name.insert(default_attachment_name, default_attachment);
        // If there are no inputs whatsoever, just use a dummy one sized buffer.
        let enabled_passes: Vec<_> = pip.passes.iter().filter(|e| !e.is_disabled()).collect();
        // Descriptor pool to use across all descriptor sets
        let descriptor_pool = super::descriptor::make_pool(ctx);
        ctx.try_set_debug_name("main_descriptor_pool", descriptor_pool);
        let image_descriptors = Self::image_desc_buffer(ctx, descriptor_pool);
        let mut sampler_descriptors =
            Self::sampler_desc_buffer(ctx, descriptor_pool, Renderer::MAX_SAMPLERS);

        let mut samplers_by_key: HashMap<SamplerKey, Sampler> = HashMap::new();

        let mut stages = Vec::<Box<dyn Stage>>::with_capacity(enabled_passes.len());
        for (pass_index, pass) in enabled_passes.iter().enumerate() {
            let render_pass = match pass {
                PipelineStep::Blit(blit) => {
                    let blit_stage = Self::build_blit_stage(
                        blit,
                        &barrier_gen,
                        pass_index,
                        is_validation_layer_enabled,
                        window_width,
                        window_height,
                        &attachments_by_name,
                    );
                    stages.push(Box::new(blit_stage));
                    // Nothing else to do for blit stages
                    continue;
                }
                // Render pass requires the longer setup below
                PipelineStep::Render(render) => render,
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
            let attachment_outputs =
                Self::find_attachments(&render_pass.outputs, &attachments_by_name);
            let attachment_inputs = Self::find_attachments(
                &render_pass
                    .inputs
                    .iter()
                    .map(|e| e.name.clone())
                    .collect::<Vec<_>>(),
                &attachments_by_name,
            );
            let attachment_samplers: Vec<_> = render_pass
                .inputs
                .iter()
                .map(|i| {
                    let sampler = Self::handle_option(i.sampler.clone());
                    let key = SamplerKey {
                        filter: sampler.filter,
                        wrap_mode: sampler.wrap_mode,
                        compare_func: sampler.compare_func,
                        anisotropy: sampler.anisotropy,
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
                    descriptor_pool,
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
                let descriptor_index = attachment_descriptors
                    .as_mut()
                    .unwrap()
                    .place_image_sampler(
                        ctx,
                        e.0.view,
                        vk::ImageLayout::READ_ONLY_OPTIMAL,
                        e.1.sampler,
                    );
                Attachment {
                    descriptor_offset: 0,
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
                /*
                 * When using a COLOR attachment, we're storing unconditionally.
                 * Same case when using a DEPTH_STENCIL attachment and depth-stencil writing is enabled.
                 */
                store_op: if writing.depth_or_stencil() || !e.format.has_depth_or_stencil() {
                    vk::AttachmentStoreOp::STORE
                } else {
                    // Don't store if using a DEPTH_STENCIL attachment with writes disabled
                    vk::AttachmentStoreOp::NONE
                },
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
                barrier_gen.gen_image_barriers_for(pass_index, &inputs, &outputs_for_barriers);

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

            ctx.try_set_debug_name(&format!("{}_pipeline", render_pass.name), graphics_pipeline);
            ctx.try_set_debug_name(
                &format!("{}_pipeline_layout", render_pass.name),
                pipeline_layout,
            );
            // TODO: Deferred descriptor writes
            // if let Some(d) = &mut attachment_descriptors {
            //     // If there are any input descriptors, write them into device memory
            //     d.into_device()
            // }
            stages.push(Box::new(crate::pipeline::render_stage::RenderStage {
                name: render_pass.name.clone(),
                is_validation_layer_enabled,
                rendering: super::render_stage::Rendering {
                    attachments: attachment_rendering,
                    depth_stencil: depth_stencil_rendering,
                    default_attachment_index,
                },
                batch_parent_id: render_pass.batch_parent_id,
                render_area: scissors[0],
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
                index: pass_index as u32,
                is_final: default_attachment_index.is_some(),
                image_barriers,
                attachment_descriptors,
                reserved_buffers: Vec::new(),
            }));
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
            sampler_descriptors.place_sampler_at(ctx, sampler.position as u32, sampler.sampler);
        }
        // TODO: Deferred descriptor writes
        // sampler_descriptors.into_device();
        // image_descriptors.into_device();
        return crate::pipeline::Pipeline {
            stages,
            attachments: attachments_by_name.into_values().collect(),
            descriptor_pool,
            image_descriptors,
            sampler_descriptors,
            samplers_by_key,
        };
    }

    fn find_attachments(
        names: &[String],
        attachments_by_name: &HashMap<String, Attachment>,
    ) -> Vec<Attachment> {
        names
            .iter()
            .map(|e| {
                attachments_by_name
                    .get(e)
                    .expect(&format!("attachment {e} missing!"))
                    .clone()
            })
            .collect()
    }

    fn build_blit_stage(
        blit: &BlitPass,
        barrier_gen: &BarrierGen,
        index: usize,
        is_validation_layer_enabled: bool,
        window_width: u32,
        window_height: u32,
        attachments_by_name: &HashMap<String, Attachment>,
    ) -> crate::pipeline::blit_stage::BlitStage {
        let mut outputs = Self::find_attachments(&[blit.output.clone()], &attachments_by_name);
        let mut inputs = Self::find_attachments(&[blit.input.clone()], &attachments_by_name);
        let image_barriers = barrier_gen.gen_image_barriers_for(index, &inputs, &outputs);
        crate::pipeline::blit_stage::BlitStage {
            name: blit.name.clone(),
            index: index.try_into().unwrap(),
            is_validation_layer_enabled,
            image_barriers,
            filter: blit.filter.to_vk(),
            region: blit.to_vk(window_width as f32, window_height as f32),
            input: inputs.remove(0),
            output: outputs.remove(0),
        }
    }

    pub fn image_desc_buffer(ctx: &VulkanContext, pool: vk::DescriptorPool) -> DescriptorGroup {
        let desc_buffer = DescriptorGroup::of(
            ctx,
            pool,
            "images".to_string(),
            DescriptorType::SAMPLED_IMAGE,
            1024,
            true,
        );
        desc_buffer
    }

    pub fn attachment_image_desc_buffer(
        ctx: &VulkanContext,
        pool: vk::DescriptorPool,
        prefix: &str,
        size: u32,
    ) -> DescriptorGroup {
        let name = format!("{}_attachments", prefix);
        let desc_buffer = DescriptorGroup::of(
            ctx,
            pool,
            name,
            DescriptorType::COMBINED_IMAGE_SAMPLER,
            size,
            false,
        );
        desc_buffer
    }

    pub fn sampler_desc_buffer(
        ctx: &VulkanContext,
        pool: vk::DescriptorPool,
        size: u32,
    ) -> DescriptorGroup {
        let desc_buffer = DescriptorGroup::of(
            ctx,
            pool,
            "samplers".to_string(),
            DescriptorType::SAMPLER,
            size,
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
