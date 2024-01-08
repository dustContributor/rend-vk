use ash::vk;

use super::{
    attachment::Attachment,
    file::{DescHandler, Pipeline, PipelineStep},
};

struct Pass {
    inputs: Vec<String>,
    outputs: Vec<String>,
    is_blitting: bool,
}

pub struct BarrierGen {
    passes: Vec<Pass>,
}
struct BarrierEval {
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
    src_access: vk::AccessFlags2,
    already_issued: bool,
    keep_searching: bool,
}

impl BarrierEval {
    fn of(
        src_access: vk::AccessFlags2,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) -> Self {
        Self {
            already_issued: false,
            keep_searching: false,
            src_access,
            old_layout,
            new_layout,
        }
    }

    fn already_issued() -> Self {
        Self {
            already_issued: true,
            keep_searching: false,
            src_access: vk::AccessFlags2::NONE,
            old_layout: vk::ImageLayout::UNDEFINED,
            new_layout: vk::ImageLayout::UNDEFINED,
        }
    }

    fn next_pass() -> Self {
        Self {
            already_issued: false,
            keep_searching: true,
            src_access: vk::AccessFlags2::NONE,
            old_layout: vk::ImageLayout::UNDEFINED,
            new_layout: vk::ImageLayout::UNDEFINED,
        }
    }
}

impl BarrierGen {
    pub fn new(passes: &Vec<PipelineStep>) -> Self {
        let tmp = passes
            .iter()
            // Filter out disabled passes and only keep the inputs/outputs around
            .filter(|p| !p.is_disabled())
            .map(|p| match p {
                PipelineStep::Render(p) => {
                    let mut outputs = p.outputs.clone();
                    let mut inputs: Vec<_> = p.inputs.iter().map(|i| i.name.clone()).collect();
                    // Depth stencil attachment requires some special checks
                    if let Some(d) = &p.depth_stencil {
                        let writing = Pipeline::handle_option(p.state.writing.clone());
                        if writing.depth {
                            // Writes depth, interpret it as an output from the pass
                            outputs.push(d.clone());
                        } else {
                            /*
                             * Assume it's only depth testing, interpret it as an input,
                             * checking if it isn't already being sampled in the same pass.
                             */
                            if !inputs.contains(d) {
                                inputs.push(d.clone());
                            }
                        }
                    }
                    Pass {
                        is_blitting: false,
                        // depth_stencil: p.depth_stencil.clone(),
                        inputs,
                        outputs,
                    }
                }
                // Blit pass has only one input/output, re-represent as single item vecs
                PipelineStep::Blit(p) => Pass {
                    is_blitting: true,
                    // depth_stencil: None,
                    inputs: [p.input.clone()].into(),
                    outputs: [p.output.clone()].into(),
                },
            })
            .collect();
        BarrierGen { passes: tmp }
    }

    fn eval_barrier_for(prev: &Pass, input: &Attachment, curr_is_blitting: bool) -> BarrierEval {
        let new_layout = if curr_is_blitting {
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL
        } else {
            vk::ImageLayout::READ_ONLY_OPTIMAL
        };
        if prev.inputs.iter().any(|e| e.eq(&input.name)) {
            // Previous pass had this attachment as an input
            if (prev.is_blitting && curr_is_blitting) || (!prev.is_blitting && !curr_is_blitting) {
                // Already issued this same barrier before
                return BarrierEval::already_issued();
            }
            if prev.is_blitting {
                // Curr is not blitting, prev was. Issue a transition from transfer src
                return BarrierEval::of(
                    vk::AccessFlags2::MEMORY_READ,
                    vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    new_layout,
                );
            }
            // Curr is blitting, prev wasn't. Issue a transition from shader read
            return BarrierEval::of(
                vk::AccessFlags2::MEMORY_READ,
                vk::ImageLayout::READ_ONLY_OPTIMAL,
                new_layout,
            );
        }
        if prev.outputs.contains(&input.name) {
            // Previous pass had this attachment as an output
            if prev.is_blitting {
                // Prev was blitting. Issue a transition from transfer dst
                return BarrierEval::of(
                    vk::AccessFlags2::MEMORY_WRITE,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    new_layout,
                );
            }
            // Prev wasn't blitting. Issue a transition from fragment shader write
            return BarrierEval::of(
                vk::AccessFlags2::MEMORY_WRITE,
                vk::ImageLayout::ATTACHMENT_OPTIMAL,
                new_layout,
            );
        }
        // Previous pass didn't reference this attachment, continue.
        BarrierEval::next_pass()
    }

    pub fn gen_image_barriers_for(
        &self,
        currenti: usize,
        inputs: &Vec<Attachment>,
        outputs: &Vec<Attachment>,
    ) -> Vec<vk::ImageMemoryBarrier2> {
        let mut i = currenti;
        let mut barriers: Vec<vk::ImageMemoryBarrier2> = Vec::new();
        let curr_is_blitting = self.passes[currenti].is_blitting;
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
                i = wrap_around(i, self.passes.len());
                if i == currenti {
                    // Looped back to current pass, nothing to check
                    break;
                }
                // DEBUG - ERROR:VALIDATION
                // [UNASSIGNED-CoreValidation-DrawState-InvalidImageLayout (1303270965)]:
                // Validation Error: [ UNASSIGNED-CoreValidation-DrawState-InvalidImageLayout ]
                // Object 0: handle = 0x7ffff1ec1280, type = VK_OBJECT_TYPE_COMMAND_BUFFER; |
                //  MessageID = 0x4dae5635 | vkQueueSubmit(): pSubmits[0].pCommandBuffers[0]
                //  command buffer VkCommandBuffer 0x7ffff1ec1280[] expects VkImage
                //  0x310000000031[depth_image] (subresource: aspectMask 0x2 array layer 0, mip level 0)
                //  to be in layout VK_IMAGE_LAYOUT_DEPTH_READ_ONLY_OPTIMAL--instead,
                //  current layout is VK_IMAGE_LAYOUT_DEPTH_ATTACHMENT_OPTIMAL.
                let prev = &self.passes[i];
                let ev_barrier = Self::eval_barrier_for(prev, input, curr_is_blitting);
                if ev_barrier.already_issued {
                    break;
                }
                if ev_barrier.keep_searching {
                    continue;
                }
                // Image was written to before, barrier for reading
                let barrier = vk::ImageMemoryBarrier2::builder()
                    .image(input.image)
                    .src_access_mask(ev_barrier.src_access)
                    .dst_access_mask(vk::AccessFlags2::MEMORY_READ)
                    .old_layout(ev_barrier.old_layout)
                    .new_layout(ev_barrier.new_layout)
                    .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                    .dst_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
                    .subresource_range(Attachment::default_subresource_range(input.format.aspect()))
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
                i = wrap_around(i, self.passes.len());
                if i == currenti {
                    // Looped back to current pass, nothing to check
                    break;
                }
                let prev = &self.passes[i];
                let old_layout;
                let src_access;
                if prev.outputs.contains(&output.name) {
                    // Previous pass had this attachment as an output
                    if (prev.is_blitting && curr_is_blitting)
                        || (!prev.is_blitting && !curr_is_blitting)
                    {
                        // Already issued this same barrier before
                        break;
                    }
                    if prev.is_blitting {
                        // Curr is not blitting, prev was. Issue a transition from transfer dst
                        old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
                        src_access = vk::AccessFlags2::MEMORY_WRITE;
                    } else {
                        // Curr is blitting, prev wasn't. Issue a transition from shader write
                        old_layout = vk::ImageLayout::ATTACHMENT_OPTIMAL;
                        src_access = vk::AccessFlags2::MEMORY_WRITE;
                    }
                } else if prev.inputs.iter().any(|e| e.eq(&output.name)) {
                    // Previous pass had this attachment as an input
                    if prev.is_blitting {
                        // Prev was blitting. Issue a transition from transfer dst
                        old_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
                        src_access = vk::AccessFlags2::MEMORY_READ;
                    } else {
                        // Prev wasn't blitting. Issue a transition from fragment shader read
                        old_layout = vk::ImageLayout::READ_ONLY_OPTIMAL;
                        src_access = vk::AccessFlags2::MEMORY_READ;
                    }
                } else {
                    // Continue to previous pass
                    continue;
                }
                // Image was read before, issue barrier for writing
                let new_layout = if curr_is_blitting {
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL
                } else {
                    vk::ImageLayout::ATTACHMENT_OPTIMAL
                };
                let barrier = vk::ImageMemoryBarrier2::builder()
                    .image(output.image)
                    .src_access_mask(src_access)
                    .dst_access_mask(vk::AccessFlags2::MEMORY_WRITE)
                    .old_layout(old_layout)
                    .new_layout(new_layout)
                    .src_stage_mask(vk::PipelineStageFlags2::NONE)
                    .dst_stage_mask(if output.format.has_depth_or_stencil() {
                        vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS
                    } else {
                        vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT
                    })
                    .subresource_range(Attachment::default_subresource_range(
                        output.format.aspect(),
                    ))
                    .build();
                barriers.push(barrier);
                break;
            }
        }
        return barriers;
    }
}
