use std::{collections::HashMap, ops::Range};

use ash::vk;

use crate::texture::MipMap;

use super::{
    attachment::Attachment,
    file::{AttachmentFile, BaseState, DescHandler, Pipeline, PipelineStep, State, Target},
};

struct Image {
    name: String,
    level: u8,
}

impl Image {
    pub fn of_attachment(f: &impl super::file::AttachmentFile) -> Self {
        Self {
            level: f.level().get(),
            name: f.name().to_string(),
        }
    }
}

struct Pass {
    name: String,
    inputs: Vec<Image>,
    outputs: Vec<Image>,
    is_blitting: bool,
}

pub struct BarrierGen {
    passes: Vec<Pass>,
    levels_by_owner: HashMap<String, u8>,
}

#[derive(Default)]
struct BarrierEval {
    src_access: vk::AccessFlags2,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
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
    pub fn new(
        targets: &[Target],
        passes: &[PipelineStep],
        resolve_state: &dyn Fn(&BaseState) -> State,
    ) -> Self {
        let levels_by_owner = targets
            .iter()
            .map(|t| (t.name.clone(), t.level))
            .collect::<HashMap<_, _>>();
        let tmp = passes
            .iter()
            .map(|p| match p {
                PipelineStep::Render(p) => {
                    let mut outputs: Vec<_> = p
                        .outputs
                        .iter()
                        .map(|e| e.get())
                        .map(|i| Image::of_attachment(&i))
                        .collect();
                    let mut inputs = Vec::with_capacity(p.inputs.len());
                    for input in p.inputs.iter().map(|i| i.get()) {
                        for lvl in
                            Self::level_range_for(&input.name, input.level.get(), &levels_by_owner)
                        {
                            inputs.push(Image {
                                level: lvl,
                                name: input.name().to_string(),
                            });
                        }
                    }
                    // Depth stencil attachment requires some special checks
                    if let Some(d) = &p.depth_stencil {
                        let state = resolve_state(&p.state);
                        let writing = Pipeline::handle_option(state.writing);
                        let depth_img = Image {
                            name: d.clone(),
                            level: 0,
                        };
                        if writing.depth {
                            // Writes depth, interpret it as an output from the pass
                            outputs.push(depth_img);
                        } else {
                            /*
                             * Assume it's only depth testing, interpret it as an input,
                             * checking if it isn't already being sampled in the same pass.
                             */
                            if !inputs.iter().any(|e| d == &e.name) {
                                inputs.push(depth_img);
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
                    inputs: [Image::of_attachment(&p.input.get())].into(),
                    outputs: [Image::of_attachment(&p.output.get())].into(),
                },
                _ => panic!("unsupported pipeline step!"),
            })
            .collect();
        BarrierGen {
            passes: tmp,
            levels_by_owner,
        }
    }

    fn level_range_for(
        name: &str,
        level_usage: u8,
        levels_by_owner: &HashMap<String, u8>,
    ) -> Range<u8> {
        let owner_levels = match levels_by_owner.get(name) {
            Some(r) => *r,
            None => panic!("levels for attachment '{}' not found!", name),
        };
        if MipMap::is_all_levels_value(level_usage) {
            0..owner_levels
        } else {
            level_usage..level_usage + 1
        }
    }

    fn eval_barrier_for(
        prev_pass: &Pass,
        name: &str,
        level: u8,
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
        if prev_pass
            .inputs
            .iter()
            .any(|e| e.name.eq(name) && e.level == level)
        {
            // Previous pass had this attachment as an input
            if !is_output {
                // Only check for same barriers if it's evaluating an input against inputs
                if prev_pass.is_blitting == current_is_blitting {
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
        if prev_pass
            .outputs
            .iter()
            .any(|e| e.name.eq(name) && e.level == level)
        {
            // Previous pass had this attachment as an output
            if is_output {
                // Only check for same barriers if it's evaluating an output against outputs
                if prev_pass.is_blitting == current_is_blitting {
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
            if input.is_default() {
                panic!("Can't read from the default attachment!")
            }
            let level_range =
                Self::level_range_for(&input.name, input.level_usage, &self.levels_by_owner);
            // We may need to emit one barrier per mip map level if the input reads several of them
            for level_usage in level_range {
                // Search back starting from current passs
                let mut i = currenti;
                loop {
                    i = wrap_around(i, self.passes.len());
                    if i == currenti {
                        // Looped back to current pass, nothing to check
                        break;
                    }
                    let prev = &self.passes[i];
                    let ev_barrier = Self::eval_barrier_for(
                        prev,
                        &input.name,
                        level_usage,
                        curr_is_blitting,
                        false,
                    );
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
                        .subresource_range(Attachment::subresource_range_wlevels(
                            input.format.aspect(),
                            level_usage as u32,
                            1,
                        ))
                        .build();
                    barriers.push((&input.name, false, barrier));
                    break;
                }
            }
        }
        for output in outputs {
            if output.is_default() {
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
                let ev_barrier = Self::eval_barrier_for(
                    prev,
                    &output.name,
                    output.level_usage, // always a specific mip
                    curr_is_blitting,
                    true,
                );
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
                    .subresource_range(Attachment::subresource_range_wlevels(
                        output.format.aspect(),
                        output.level_usage as u32,
                        1,
                    ))
                    .build();
                barriers.push((&output.name, true, barrier));
                break;
            }
        }
        if log::log_enabled!(log::Level::Trace) {
            // Log the generated barriers if we're in TRACE logging level
            log::trace!(
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
                log::trace!(
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
        barriers.iter().map(|e| e.2).collect()
    }
}
