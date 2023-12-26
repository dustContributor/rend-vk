use ash::vk::{self};

use crate::context::VulkanContext;

use super::file::{Filtering, WrapMode};

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct SamplerKey {
    pub filter: Filtering,
    pub wrap_mode: WrapMode,
    pub anisotropy: u8,
}

#[derive(Clone)]
pub struct Sampler {
    pub name: String,
    pub sampler: vk::Sampler,
    pub descriptor_offset: usize,
    pub position: u32,
}

impl Sampler {
    pub fn of_key(ctx: &VulkanContext, name: String, key: SamplerKey) -> Self {
        Self::of(ctx, name, key.filter, key.wrap_mode, key.anisotropy)
    }

    pub fn of(
        ctx: &VulkanContext,
        name: String,
        filter: Filtering,
        wrap_mode: WrapMode,
        anisotropy: u8,
    ) -> Self {
        let info = Self::info_of(filter, wrap_mode, anisotropy);
        let sampler = unsafe { ctx.device.create_sampler(&info, None) }.unwrap();
        ctx.try_set_debug_name(&name, sampler);
        Self {
            name,
            sampler,
            descriptor_offset: 0,
            position: 0,
        }
    }

    fn info_of(filter: Filtering, wrap_mode: WrapMode, anisotropy: u8) -> vk::SamplerCreateInfo {
        vk::SamplerCreateInfo::builder()
            .address_mode_u(wrap_mode.to_vk())
            .address_mode_v(wrap_mode.to_vk())
            .address_mode_w(wrap_mode.to_vk())
            .anisotropy_enable(if anisotropy > 1 { true } else { false })
            .compare_enable(false)
            .mipmap_mode(filter.to_vk_mip_map())
            .min_filter(filter.to_vk())
            .mag_filter(filter.to_vk())
            .max_anisotropy(anisotropy as f32)
            .max_lod(vk::LOD_CLAMP_NONE)
            .build()
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe { device.destroy_sampler(self.sampler, None) }
    }
}
