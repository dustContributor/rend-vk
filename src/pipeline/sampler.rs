use ash::vk;

use super::file::SamplerKind;

#[derive(Clone)]
pub struct Sampler {
    name: String,
    sampler: vk::Sampler,
}

impl Sampler {
    pub fn of(device: &ash::Device, name: String, is_linear: bool) -> Self {
        let info = Self::info_of(is_linear);
        let sampler = unsafe { device.create_sampler(&info, None) }.unwrap();
        Self { name, sampler }
    }

    pub fn of_kind(device: &ash::Device, kind: SamplerKind) -> Self {
        let name = kind.to_string();
        let is_linear = match kind {
            SamplerKind::LINEAR => true,
            SamplerKind::NEAREST => false,
            _ => panic!(),
        };
        Self::of(device, name, is_linear)
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
            .min_filter(filter)
            .mag_filter(filter)
            .max_lod(vk::LOD_CLAMP_NONE)
            .build()
    }
}