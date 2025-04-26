use ash::vk::{self};

use crate::context::VulkanContext;

use super::file::{CompareFunc, Filtering, WrapMode};

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct SamplerKey {
    pub filter: Filtering,
    pub wrap_mode: WrapMode,
    pub compare_func: CompareFunc,
    pub anisotropy: u8,
}

impl SamplerKey {
    pub fn to_u32(self) -> u32 {
        // 8 bits each
        self.filter as u32
            | (self.wrap_mode as u32) << 8
            | (self.compare_func as u32) << 16
            | (self.anisotropy as u32) << 24
    }
}

#[derive(Clone)]
pub struct Sampler {
    pub name: String,
    pub sampler: vk::Sampler,
    pub descriptor_offset: usize,
    pub position: u8,
}

impl Sampler {
    pub fn of_key(ctx: &VulkanContext, key: SamplerKey, position: u8) -> Self {
        Self::of(
            ctx,
            format!("sampler_{:#x}", key.to_u32()),
            key.filter,
            key.wrap_mode,
            key.compare_func,
            Self::validate_anisotropy(key.anisotropy),
            position,
        )
    }

    fn validate_anisotropy(v: u8) -> u8 {
        match v {
            1 => 1,
            4 => 4,
            8 => 8,
            16 => 16,
            _ => panic!("unsupported anisotropy level {}", v),
        }
    }

    fn of(
        ctx: &VulkanContext,
        name: String,
        filter: Filtering,
        wrap_mode: WrapMode,
        compare_func: CompareFunc,
        anisotropy: u8,
        position: u8,
    ) -> Self {
        if position as u32 >= crate::renderer::Renderer::MAX_SAMPLERS {
            panic!(
                "can't allocate more samplers than {}!",
                crate::renderer::Renderer::MAX_SAMPLERS
            );
        }
        let info = Self::info_of(filter, wrap_mode, compare_func, anisotropy);
        let sampler = unsafe { ctx.device.create_sampler(&info, None) }.unwrap();
        ctx.try_set_debug_name(&name, sampler);
        Self {
            name,
            sampler,
            descriptor_offset: 0,
            position,
        }
    }

    fn info_of(
        filter: Filtering,
        wrap_mode: WrapMode,
        compare_func: CompareFunc,
        anisotropy: u8,
    ) -> vk::SamplerCreateInfo {
        vk::SamplerCreateInfo::builder()
            .address_mode_u(wrap_mode.to_vk())
            .address_mode_v(wrap_mode.to_vk())
            .address_mode_w(wrap_mode.to_vk())
            .anisotropy_enable(anisotropy > 1)
            .compare_enable(compare_func != CompareFunc::None)
            .compare_op(match compare_func {
                CompareFunc::None => vk::CompareOp::ALWAYS,
                _ => compare_func.to_vk(),
            })
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
