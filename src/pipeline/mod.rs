use ash::vk;
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    process::Command,
};

use crate::{buffer, format, render, shader};

pub mod file;
mod load;
mod state;

struct Attachment {
    name: String,
    memory: vk::DeviceMemory,
    image: vk::Image,
}

struct PipelineStage {
    name: String,
    outputs: Vec<Attachment>,
    inputs: Vec<Attachment>,
}
