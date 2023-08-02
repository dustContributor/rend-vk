#[macro_use]
extern crate lazy_static;

pub mod batch;
pub mod buffer;
pub mod context;
pub mod debug;
pub mod format;
pub mod java_api;
pub mod pipeline;
pub mod render;
pub mod render_task;
pub mod renderer;
pub mod shader;
pub mod swapchain;
pub mod updater;
pub mod window;
pub mod texture;

pub const DEBUG_ENABLED: bool = true;
pub const VALIDATION_LAYER_ENABLED: bool = true;

pub trait UsedAsIndex<const T: u8>
{
    const MAX_VALUE: u8 = T;
    const MAX_SIZE: usize = Self::MAX_VALUE as usize;
    const MAX_LEN: usize = Self::MAX_SIZE + 1;
}