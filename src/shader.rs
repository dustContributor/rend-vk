use ash::{vk, Device};

use crate::context::VulkanContext;

pub const ATTRIB_LOC_POSITION: u32 = 0;
pub const ATTRIB_LOC_NORMAL: u32 = 1;
pub const ATTRIB_LOC_COLOR: u32 = 2;
pub const ATTRIB_LOC_TEXCOORD: u32 = 3;
pub const ATTRIB_LOC_JOINT_WEIGHT: u32 = 4;
pub const ATTRIB_LOC_INSTANCE_ID: u32 = 5;

pub struct ShaderProgram {
    pub name: String,
    pub shaders: Vec<Shader>,
}
pub struct Shader {
    pub name: String,
    pub info: vk::PipelineShaderStageCreateInfo,
}
impl Shader {
    pub fn type_id(&self) -> vk::ShaderStageFlags {
        self.info.stage
    }
}
impl ShaderProgram {
    pub fn destroy(&self, device: &Device) {
        self.shaders
            .iter()
            .for_each(|e| unsafe { device.destroy_shader_module(e.info.module, None) });
    }
    pub fn new(
        ctx: &VulkanContext,
        name: String,
        vertex: Option<(String, Vec<u32>)>,
        fragment: Option<(String, Vec<u32>)>,
        geometry: Option<(String, Vec<u32>)>,
    ) -> Self {
        let shader_entry_name = c"main";
        let stage_infos: Vec<Shader> = vec![vertex, fragment, geometry]
            .into_iter()
            .enumerate()
            .filter_map(|(i, c)| match c {
                Some(shader_bin) => {
                    let (sh_type, sh_type_name) = match i {
                        0 => (vk::ShaderStageFlags::VERTEX, "vert"),
                        1 => (vk::ShaderStageFlags::FRAGMENT, "frag"),
                        2 => (vk::ShaderStageFlags::GEOMETRY, "geom"),
                        _ => panic!("unrecognized shader type {}", i),
                    };
                    let info = vk::ShaderModuleCreateInfo::default().code(&shader_bin.1);
                    let module = unsafe { ctx.device.create_shader_module(&info, None) }
                        .unwrap_or_else(|_| panic!("shader module error, type: {}", i));
                    ctx.try_set_debug_name(
                        &format!("{}_shader_module_{}", name, sh_type_name),
                        module,
                    );
                    Some(Shader {
                        name: shader_bin.0,
                        info: vk::PipelineShaderStageCreateInfo {
                            module,
                            p_name: shader_entry_name.as_ptr(),
                            stage: sh_type,
                            ..Default::default()
                        },
                    })
                }
                None => None,
            })
            .collect();

        ShaderProgram {
            name,
            shaders: stage_infos,
        }
    }
}
