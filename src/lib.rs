#[macro_use]
extern crate lazy_static;

pub mod buffer;
pub mod context;
pub mod debug;
pub mod format;
pub mod java_api;
pub mod pipeline;
pub mod render_task;
pub mod renderer;
pub mod shader;
pub mod shader_resource;
pub mod swapchain;
pub mod texture;
pub mod updater;
pub mod window;

pub trait UsedAsIndex<const T: u8> {
    const MAX_VALUE: u8 = T;
    const MAX_SIZE: usize = Self::MAX_VALUE as usize;
    const MAX_LEN: usize = Self::MAX_SIZE + 1;
}

pub fn pos_mul(mul: usize, val: usize) -> usize {
    val.div_ceil(mul) * mul
}
