use ash::vk;

use super::{attachment::Attachment, file::PipelineStep};

struct Pass {
    is_blitting: bool,
    inputs: Vec<String>,
    outputs: Vec<String>,
}

pub struct BarrierGen {
    passes: Vec<Pass>,
}

impl BarrierGen {
    pub fn new(passes: &Vec<PipelineStep>) -> Self {
        let tmp = passes
            .iter()
            // Filter out disabled passes and only keep the inputs/outputs around
            .filter_map(|p| match p {
                PipelineStep::Render(p) => {
                    if p.is_disabled {
                        None
                    } else {
                        Some(Pass {
                            is_blitting: false,
                            inputs: p.inputs.iter().map(|i| i.name.clone()).collect(),
                            outputs: p.outputs.clone(),
                        })
                    }
                }
                // Blit pass has only one input/output, re-represent as single item vecs
                PipelineStep::Blit(p) => {
                    if p.is_disabled {
                        None
                    } else {
                        Some(Pass {
                            is_blitting: true,
                            inputs: [p.input.clone()].into(),
                            outputs: [p.output.clone()].into(),
                        })
                    }
                }
            })
            .collect();
        BarrierGen { passes: tmp }
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
                let prev = &self.passes[i];
                let old_layout;
                let src_access;
                if prev.inputs.iter().any(|e| e.eq(&input.name)) {
                    // Previous pass had this attachment as an input
                    if (prev.is_blitting && curr_is_blitting)
                        || (!prev.is_blitting && !curr_is_blitting)
                    {
                        // Already issued this same barrier before
                        break;
                    }
                    if prev.is_blitting {
                        // Curr is not blitting, prev was. Issue a transition from transfer src
                        old_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
                        src_access = vk::AccessFlags2::MEMORY_READ;
                    } else {
                        // Curr is blitting, prev wasn't. Issue a transition from shader read
                        old_layout = vk::ImageLayout::READ_ONLY_OPTIMAL;
                        src_access = vk::AccessFlags2::MEMORY_READ;
                    }
                } else if prev.outputs.contains(&input.name) {
                    // Previous pass had this attachment as an output
                    if prev.is_blitting {
                        // Prev was blitting. Issue a transition from transfer dst
                        old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
                        src_access = vk::AccessFlags2::MEMORY_WRITE;
                    } else {
                        // Prev wasn't blitting. Issue a transition from fragment shader write
                        old_layout = vk::ImageLayout::ATTACHMENT_OPTIMAL;
                        src_access = vk::AccessFlags2::MEMORY_WRITE;
                    }
                } else {
                    // Previous pass didn't reference this attachment, continue.
                    continue;
                }
                // Image was written to before, barrier for reading
                let new_layout = if curr_is_blitting {
                    vk::ImageLayout::TRANSFER_SRC_OPTIMAL
                } else {
                    vk::ImageLayout::READ_ONLY_OPTIMAL
                };
                let barrier = vk::ImageMemoryBarrier2::builder()
                    .image(input.image)
                    .src_access_mask(src_access)
                    .dst_access_mask(vk::AccessFlags2::MEMORY_READ)
                    .old_layout(old_layout)
                    .new_layout(new_layout)
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
