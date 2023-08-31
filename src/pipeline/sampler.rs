use ash::vk::{self, SamplerMipmapMode};

use crate::context::VulkanContext;

use super::file::SamplerKind;

#[derive(Clone)]
pub struct Sampler {
    pub name: String,
    pub sampler: vk::Sampler,
    pub descriptor_offset: usize,
}

impl Sampler {
    pub fn of(ctx: &VulkanContext, name: String, is_linear: bool) -> Self {
        let info = Self::info_of(is_linear);
        let sampler = unsafe { ctx.device.create_sampler(&info, None) }.unwrap();
        ctx.try_set_debug_name(&name, sampler);
        Self {
            name,
            sampler,
            descriptor_offset: 0,
        }
    }

    pub fn of_kind(ctx: &VulkanContext, kind: SamplerKind) -> Self {
        let name = kind.to_string();
        let is_linear = match kind {
            SamplerKind::Linear => true,
            SamplerKind::Nearest => false,
        };
        Self::of(ctx, name, is_linear)
    }

    fn info_of(is_linear: bool) -> vk::SamplerCreateInfo {
        let filter = if is_linear {
            vk::Filter::LINEAR
        } else {
            vk::Filter::NEAREST
        };
        vk::SamplerCreateInfo::builder()
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .anisotropy_enable(false)
            .compare_enable(false)
            .mipmap_mode(if is_linear {
                SamplerMipmapMode::LINEAR
            } else {
                SamplerMipmapMode::NEAREST
            })
            .min_filter(filter)
            .mag_filter(filter)
            .max_lod(vk::LOD_CLAMP_NONE)
            .build()
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe { device.destroy_sampler(self.sampler, None) }
    }
}
