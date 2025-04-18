use ash::vk;

use super::{
    attachment::Attachment,
    file::{DescHandler, Pipeline, PipelineStep},
};

struct Pass {
    name: String,
    inputs: Vec<String>,
    outputs: Vec<String>,
    is_blitting: bool,
}

pub struct BarrierGen {
    passes: Vec<Pass>,
}

#[derive(Default)]
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
            src_access,
            old_layout,
            new_layout,
            ..Default::default()
        }
    }

    fn already_issued() -> Self {
        Self {
            already_issued: true,
            ..Default::default()
        }
    }

    fn next_pass() -> Self {
        Self {
            keep_searching: true,
            ..Default::default()
        }
    }

    fn was_blitting(&self) -> bool {
        self.old_layout == vk::ImageLayout::TRANSFER_SRC_OPTIMAL
            || self.old_layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL
    }
}

impl BarrierGen {
    pub fn new(passes: &[PipelineStep]) -> Self {
        let tmp = passes
            .iter()
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
                        name: p.name.clone(),
                        is_blitting: false,
                        inputs,
                        outputs,
                    }
                }
                // Blit pass has only one input/output, re-represent as single item vecs
                PipelineStep::Blit(p) => Pass {
                    name: p.name.clone(),
                    is_blitting: true,
                    inputs: [p.input.clone()].into(),
                    outputs: [p.output.clone()].into(),
                },
                _ => panic!("unsupported pipeline step!"),
            })
            .collect();
        BarrierGen { passes: tmp }
    }

    fn output_barrier_for(
        prev_pass: &Pass,
        attachment: &Attachment,
        current_is_blitting: bool,
    ) -> BarrierEval {
        Self::eval_barrier_for(prev_pass, attachment, current_is_blitting, true)
    }

    fn input_barrier_for(
        prev_pass: &Pass,
        attachment: &Attachment,
        current_is_blitting: bool,
    ) -> BarrierEval {
        Self::eval_barrier_for(prev_pass, attachment, current_is_blitting, false)
    }

    fn eval_barrier_for(
        prev_pass: &Pass,
        attachment: &Attachment,
        current_is_blitting: bool,
        is_output: bool,
    ) -> BarrierEval {
        let new_layout = if current_is_blitting {
            // Blitting outputs require "transfer dst", inputs "transfer src"
            if is_output {
                vk::ImageLayout::TRANSFER_DST_OPTIMAL
            } else {
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL
            }
        } else {
            // Non blitting outputs require "attachment", inputs "read only"
            if is_output {
                vk::ImageLayout::ATTACHMENT_OPTIMAL
            } else {
                vk::ImageLayout::READ_ONLY_OPTIMAL
            }
        };
        if prev_pass.inputs.iter().any(|e| e.eq(&attachment.name)) {
            // Previous pass had this attachment as an input
            if !is_output {
                // Only check for same barriers if it's evaluating an input against inputs
                if (prev_pass.is_blitting && current_is_blitting)
                    || (!prev_pass.is_blitting && !current_is_blitting)
                {
                    // Already issued this same barrier before
                    return BarrierEval::already_issued();
                }
            }
            if prev_pass.is_blitting {
                // Prev was blitting, curr isnt. Issue a transition from transfer src
                return BarrierEval::of(
                    vk::AccessFlags2::MEMORY_READ,
                    vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    new_layout,
                );
            }
            // Curr is blitting, prev wasn't. Issue a transition from read only
            return BarrierEval::of(
                vk::AccessFlags2::MEMORY_READ,
                vk::ImageLayout::READ_ONLY_OPTIMAL,
                new_layout,
            );
        }
        if prev_pass.outputs.contains(&attachment.name) {
            // Previous pass had this attachment as an output
            if is_output {
                // Only check for same barriers if it's evaluating an output against outputs
                if (prev_pass.is_blitting && current_is_blitting)
                    || (!prev_pass.is_blitting && !current_is_blitting)
                {
                    // Already issued this same barrier before
                    return BarrierEval::already_issued();
                }
            }
            if prev_pass.is_blitting {
                // Prev was blitting, curr isnt. Issue a transition from transfer dst
                return BarrierEval::of(
                    vk::AccessFlags2::MEMORY_WRITE,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    new_layout,
                );
            }
            // Prev wasn't blitting. Issue a transition from output attachment
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
        let mut barriers: Vec<(&str, bool, vk::ImageMemoryBarrier2)> = Vec::new();
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
            // Search back starting from current passs
            let mut i = currenti;
            loop {
                i = wrap_around(i, self.passes.len());
                if i == currenti {
                    // Looped back to current pass, nothing to check
                    break;
                }
                let prev = &self.passes[i];
                let ev_barrier = Self::input_barrier_for(prev, input, curr_is_blitting);
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
                    .src_stage_mask(if ev_barrier.was_blitting() {
                        vk::PipelineStageFlags2::TRANSFER
                    } else if input.format.has_depth_or_stencil() {
                        vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS
                            | vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS
                    } else {
                        vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT
                    })
                    .dst_stage_mask(if curr_is_blitting {
                        vk::PipelineStageFlags2::TRANSFER
                    } else if input.format.has_depth_or_stencil() {
                        vk::PipelineStageFlags2::FRAGMENT_SHADER
                            | vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS
                            | vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS
                    } else {
                        vk::PipelineStageFlags2::FRAGMENT_SHADER
                    })
                    .subresource_range(Attachment::default_subresource_range(input.format.aspect()))
                    .build();
                barriers.push((&input.name, false, barrier));
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
            // Search back starting from current passs
            let mut i = currenti;
            loop {
                i = wrap_around(i, self.passes.len());
                if i == currenti {
                    // Looped back to current pass, nothing to check
                    break;
                }
                let prev = &self.passes[i];
                let ev_barrier = Self::output_barrier_for(prev, output, curr_is_blitting);
                if ev_barrier.already_issued {
                    break;
                }
                if ev_barrier.keep_searching {
                    continue;
                }
                let barrier = vk::ImageMemoryBarrier2::builder()
                    .image(output.image)
                    .src_access_mask(ev_barrier.src_access)
                    .dst_access_mask(vk::AccessFlags2::MEMORY_WRITE)
                    .old_layout(ev_barrier.old_layout)
                    .new_layout(ev_barrier.new_layout)
                    .src_stage_mask(if ev_barrier.was_blitting() {
                        vk::PipelineStageFlags2::TRANSFER
                    } else if output.format.has_depth_or_stencil() {
                        vk::PipelineStageFlags2::FRAGMENT_SHADER
                            | vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS
                            | vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS
                    } else {
                        vk::PipelineStageFlags2::FRAGMENT_SHADER
                    })
                    .dst_stage_mask(if curr_is_blitting {
                        vk::PipelineStageFlags2::TRANSFER
                    } else if output.format.has_depth_or_stencil() {
                        vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS
                            | vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS
                    } else {
                        vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT
                    })
                    .subresource_range(Attachment::default_subresource_range(
                        output.format.aspect(),
                    ))
                    .build();
                barriers.push((&output.name, true, barrier));
                break;
            }
        }
        if log::log_enabled!(log::Level::Trace) {
            // Log the generated barriers if we're in TRACE logging level
            log::debug!(
                "emitted {} barriers for pass {} at index {}",
                barriers.len(),
                self.passes[currenti].name,
                currenti,
            );
            barriers.iter().enumerate().for_each(|e| {
                let tmp = format!("{:?}", e.1 .2)
                    .replace("{", "{\n")
                    .replace(", ", ",\n ")
                    .replace("}", "\n}");
                log::debug!(
                    "pass {} at {}, barrier {}, {}: {} \n{}",
                    self.passes[currenti].name,
                    currenti,
                    e.0,
                    if e.1 .1 { "output" } else { "input" },
                    e.1 .0,
                    tmp,
                );
            });
        }
        return barriers.iter().map(|e| e.2).collect();
    }
}
