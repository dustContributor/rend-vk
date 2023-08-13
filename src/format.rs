use ash::vk;
use serde::Deserialize;

use crate::UsedAsIndex;

impl Format {
    pub fn is_compressed(self) -> bool {
        (self >= Self::BC1_RGBA_SRGB_BLOCK && self <= Self::BC7_UNORM_BLOCK ) ||
        // Probably useless
        (self >= Self::ASTC_10X10_SRGB_BLOCK && self <= Self::ASTC_8X8_UNORM_BLOCK ) ||
        (self >= Self::ETC2_R8G8B8A1_SRGB_BLOCK && self <= Self::ETC2_R8G8B8_UNORM_BLOCK )
    }

    pub fn has_depth(self) -> bool {
        (self >= Self::D16_UNORM && self <= Self::D32_SFLOAT_S8_UINT)
            || self == Self::X8_D24_UNORM_PACK32
    }

    pub fn has_stencil(self) -> bool {
        match self {
            Self::D16_UNORM_S8_UINT
            | Self::D24_UNORM_S8_UINT
            | Self::D32_SFLOAT_S8_UINT
            | Self::S8_UINT => true,
            _ => false,
        }
    }

    pub fn aspect(self) -> vk::ImageAspectFlags {
        let depth = if self.has_depth() {
            vk::ImageAspectFlags::DEPTH
        } else {
            vk::ImageAspectFlags::NONE
        };
        let stencil = if self.has_stencil() {
            vk::ImageAspectFlags::STENCIL
        } else {
            vk::ImageAspectFlags::NONE
        };
        let aspect = depth | stencil;
        return if aspect == vk::ImageAspectFlags::NONE {
            vk::ImageAspectFlags::COLOR
        } else {
            aspect
        };
    }

    pub fn size_for(self, width: u32, height: u32) -> u32 {
        match self {
            // size is defined per 4x4 block for BC formats
            Self::BC1_RGBA_SRGB_BLOCK
            | Self::BC1_RGBA_UNORM_BLOCK
            | Self::BC1_RGB_SRGB_BLOCK
            | Self::BC1_RGB_UNORM_BLOCK => width * height / 4 * 8,
            Self::BC2_SRGB_BLOCK
            | Self::BC2_UNORM_BLOCK
            | Self::BC3_SRGB_BLOCK
            | Self::BC3_UNORM_BLOCK => width * height / 4 * 16,
            _ => panic!("unrecognized format {}", self),
        }
    }

    pub fn size_for_extent(self, extent: vk::Extent2D) -> u32 {
        self.size_for(extent.width, extent.height)
    }

    pub const fn of_u8(v: u8) -> Self {
        if v > Self::MAX_VALUE {
            panic!()
        } else {
            unsafe { std::mem::transmute(v) }
        }
    }

    pub const fn of_u32(v: u32) -> Self {
        if v > (Self::MAX_VALUE as u32) {
            panic!()
        } else {
            unsafe { std::mem::transmute(v as u8) }
        }
    }

    pub const fn of_usize(v: usize) -> Self {
        if v > (Self::MAX_VALUE as usize) {
            panic!()
        } else {
            unsafe { std::mem::transmute(v as u8) }
        }
    }

    pub const fn to_u8(self) -> u8 {
        self as u8
    }

    pub const fn to_u32(self) -> u32 {
        self as u32
    }

    pub const fn to_usize(self) -> usize {
        self as usize
    }
}

#[derive(Deserialize, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, strum_macros::Display)]
/* Preserve these as-is since the serde screaming case renaming wouldn't work */
#[allow(non_camel_case_types)]
pub enum Format {
    UNDEFINED,
    A1R5G5B5_UNORM_PACK16,
    A2B10G10R10_SINT_PACK32,
    A2B10G10R10_SNORM_PACK32,
    A2B10G10R10_SSCALED_PACK32,
    A2B10G10R10_UINT_PACK32,
    A2B10G10R10_UNORM_PACK32,
    A2B10G10R10_USCALED_PACK32,
    A2R10G10B10_SINT_PACK32,
    A2R10G10B10_SNORM_PACK32,
    A2R10G10B10_SSCALED_PACK32,
    A2R10G10B10_UINT_PACK32,
    A2R10G10B10_UNORM_PACK32,
    A2R10G10B10_USCALED_PACK32,
    A8B8G8R8_SINT_PACK32,
    A8B8G8R8_SNORM_PACK32,
    A8B8G8R8_SRGB_PACK32,
    A8B8G8R8_SSCALED_PACK32,
    A8B8G8R8_UINT_PACK32,
    A8B8G8R8_UNORM_PACK32,
    A8B8G8R8_USCALED_PACK32,
    ASTC_10X10_SRGB_BLOCK,
    ASTC_10X10_UNORM_BLOCK,
    ASTC_10X5_SRGB_BLOCK,
    ASTC_10X5_UNORM_BLOCK,
    ASTC_10X6_SRGB_BLOCK,
    ASTC_10X6_UNORM_BLOCK,
    ASTC_10X8_SRGB_BLOCK,
    ASTC_10X8_UNORM_BLOCK,
    ASTC_12X10_SRGB_BLOCK,
    ASTC_12X10_UNORM_BLOCK,
    ASTC_12X12_SRGB_BLOCK,
    ASTC_12X12_UNORM_BLOCK,
    ASTC_4X4_SRGB_BLOCK,
    ASTC_4X4_UNORM_BLOCK,
    ASTC_5X4_SRGB_BLOCK,
    ASTC_5X4_UNORM_BLOCK,
    ASTC_5X5_SRGB_BLOCK,
    ASTC_5X5_UNORM_BLOCK,
    ASTC_6X5_SRGB_BLOCK,
    ASTC_6X5_UNORM_BLOCK,
    ASTC_6X6_SRGB_BLOCK,
    ASTC_6X6_UNORM_BLOCK,
    ASTC_8X5_SRGB_BLOCK,
    ASTC_8X5_UNORM_BLOCK,
    ASTC_8X6_SRGB_BLOCK,
    ASTC_8X6_UNORM_BLOCK,
    ASTC_8X8_SRGB_BLOCK,
    ASTC_8X8_UNORM_BLOCK,
    B10G11R11_UFLOAT_PACK32,
    B4G4R4A4_UNORM_PACK16,
    B5G5R5A1_UNORM_PACK16,
    B5G6R5_UNORM_PACK16,
    B8G8R8A8_SINT,
    B8G8R8A8_SNORM,
    B8G8R8A8_SRGB,
    B8G8R8A8_SSCALED,
    B8G8R8A8_UINT,
    B8G8R8A8_UNORM,
    B8G8R8A8_USCALED,
    B8G8R8_SINT,
    B8G8R8_SNORM,
    B8G8R8_SRGB,
    B8G8R8_SSCALED,
    B8G8R8_UINT,
    B8G8R8_UNORM,
    B8G8R8_USCALED,
    BC1_RGBA_SRGB_BLOCK,
    BC1_RGBA_UNORM_BLOCK,
    BC1_RGB_SRGB_BLOCK,
    BC1_RGB_UNORM_BLOCK,
    BC2_SRGB_BLOCK,
    BC2_UNORM_BLOCK,
    BC3_SRGB_BLOCK,
    BC3_UNORM_BLOCK,
    BC4_SNORM_BLOCK,
    BC4_UNORM_BLOCK,
    BC5_SNORM_BLOCK,
    BC5_UNORM_BLOCK,
    BC6H_SFLOAT_BLOCK,
    BC6H_UFLOAT_BLOCK,
    BC7_SRGB_BLOCK,
    BC7_UNORM_BLOCK,
    D16_UNORM,
    D16_UNORM_S8_UINT,
    D24_UNORM_S8_UINT,
    D32_SFLOAT,
    D32_SFLOAT_S8_UINT,
    E5B9G9R9_UFLOAT_PACK32,
    EAC_R11G11_SNORM_BLOCK,
    EAC_R11G11_UNORM_BLOCK,
    EAC_R11_SNORM_BLOCK,
    EAC_R11_UNORM_BLOCK,
    ETC2_R8G8B8A1_SRGB_BLOCK,
    ETC2_R8G8B8A1_UNORM_BLOCK,
    ETC2_R8G8B8A8_SRGB_BLOCK,
    ETC2_R8G8B8A8_UNORM_BLOCK,
    ETC2_R8G8B8_SRGB_BLOCK,
    ETC2_R8G8B8_UNORM_BLOCK,
    R16G16B16A16_SFLOAT,
    R16G16B16A16_SINT,
    R16G16B16A16_SNORM,
    R16G16B16A16_SSCALED,
    R16G16B16A16_UINT,
    R16G16B16A16_UNORM,
    R16G16B16A16_USCALED,
    R16G16B16_SFLOAT,
    R16G16B16_SINT,
    R16G16B16_SNORM,
    R16G16B16_SSCALED,
    R16G16B16_UINT,
    R16G16B16_UNORM,
    R16G16B16_USCALED,
    R16G16_SFLOAT,
    R16G16_SINT,
    R16G16_SNORM,
    R16G16_SSCALED,
    R16G16_UINT,
    R16G16_UNORM,
    R16G16_USCALED,
    R16_SFLOAT,
    R16_SINT,
    R16_SNORM,
    R16_SSCALED,
    R16_UINT,
    R16_UNORM,
    R16_USCALED,
    R32G32B32A32_SFLOAT,
    R32G32B32A32_SINT,
    R32G32B32A32_UINT,
    R32G32B32_SFLOAT,
    R32G32B32_SINT,
    R32G32B32_UINT,
    R32G32_SFLOAT,
    R32G32_SINT,
    R32G32_UINT,
    R32_SFLOAT,
    R32_SINT,
    R32_UINT,
    R4G4B4A4_UNORM_PACK16,
    R4G4_UNORM_PACK8,
    R5G5B5A1_UNORM_PACK16,
    R5G6B5_UNORM_PACK16,
    R64G64B64A64_SFLOAT,
    R64G64B64A64_SINT,
    R64G64B64A64_UINT,
    R64G64B64_SFLOAT,
    R64G64B64_SINT,
    R64G64B64_UINT,
    R64G64_SFLOAT,
    R64G64_SINT,
    R64G64_UINT,
    R64_SFLOAT,
    R64_SINT,
    R64_UINT,
    R8G8B8A8_SINT,
    R8G8B8A8_SNORM,
    R8G8B8A8_SRGB,
    R8G8B8A8_SSCALED,
    R8G8B8A8_UINT,
    R8G8B8A8_UNORM,
    R8G8B8A8_USCALED,
    R8G8B8_SINT,
    R8G8B8_SNORM,
    R8G8B8_SRGB,
    R8G8B8_SSCALED,
    R8G8B8_UINT,
    R8G8B8_UNORM,
    R8G8B8_USCALED,
    R8G8_SINT,
    R8G8_SNORM,
    R8G8_SRGB,
    R8G8_SSCALED,
    R8G8_UINT,
    R8G8_UNORM,
    R8G8_USCALED,
    R8_SINT,
    R8_SNORM,
    R8_SRGB,
    R8_SSCALED,
    R8_UINT,
    R8_UNORM,
    R8_USCALED,
    S8_UINT,
    X8_D24_UNORM_PACK32,
}

const MAX_FORMAT: u8 = Format::X8_D24_UNORM_PACK32.to_u8();
impl UsedAsIndex<MAX_FORMAT> for Format {}

impl Format {
    pub fn to_vk(self) -> vk::Format {
        match self {
            Self::UNDEFINED => vk::Format::UNDEFINED,
            Self::A1R5G5B5_UNORM_PACK16 => vk::Format::A1R5G5B5_UNORM_PACK16,
            Self::A2B10G10R10_SINT_PACK32 => vk::Format::A2B10G10R10_SINT_PACK32,
            Self::A2B10G10R10_SNORM_PACK32 => vk::Format::A2B10G10R10_SNORM_PACK32,
            Self::A2B10G10R10_SSCALED_PACK32 => vk::Format::A2B10G10R10_SSCALED_PACK32,
            Self::A2B10G10R10_UINT_PACK32 => vk::Format::A2B10G10R10_UINT_PACK32,
            Self::A2B10G10R10_UNORM_PACK32 => vk::Format::A2B10G10R10_UNORM_PACK32,
            Self::A2B10G10R10_USCALED_PACK32 => vk::Format::A2B10G10R10_USCALED_PACK32,
            Self::A2R10G10B10_SINT_PACK32 => vk::Format::A2R10G10B10_SINT_PACK32,
            Self::A2R10G10B10_SNORM_PACK32 => vk::Format::A2R10G10B10_SNORM_PACK32,
            Self::A2R10G10B10_SSCALED_PACK32 => vk::Format::A2R10G10B10_SSCALED_PACK32,
            Self::A2R10G10B10_UINT_PACK32 => vk::Format::A2R10G10B10_UINT_PACK32,
            Self::A2R10G10B10_UNORM_PACK32 => vk::Format::A2R10G10B10_UNORM_PACK32,
            Self::A2R10G10B10_USCALED_PACK32 => vk::Format::A2R10G10B10_USCALED_PACK32,
            Self::A8B8G8R8_SINT_PACK32 => vk::Format::A8B8G8R8_SINT_PACK32,
            Self::A8B8G8R8_SNORM_PACK32 => vk::Format::A8B8G8R8_SNORM_PACK32,
            Self::A8B8G8R8_SRGB_PACK32 => vk::Format::A8B8G8R8_SRGB_PACK32,
            Self::A8B8G8R8_SSCALED_PACK32 => vk::Format::A8B8G8R8_SSCALED_PACK32,
            Self::A8B8G8R8_UINT_PACK32 => vk::Format::A8B8G8R8_UINT_PACK32,
            Self::A8B8G8R8_UNORM_PACK32 => vk::Format::A8B8G8R8_UNORM_PACK32,
            Self::A8B8G8R8_USCALED_PACK32 => vk::Format::A8B8G8R8_USCALED_PACK32,
            Self::ASTC_10X10_SRGB_BLOCK => vk::Format::ASTC_10X10_SRGB_BLOCK,
            Self::ASTC_10X10_UNORM_BLOCK => vk::Format::ASTC_10X10_UNORM_BLOCK,
            Self::ASTC_10X5_SRGB_BLOCK => vk::Format::ASTC_10X5_SRGB_BLOCK,
            Self::ASTC_10X5_UNORM_BLOCK => vk::Format::ASTC_10X5_UNORM_BLOCK,
            Self::ASTC_10X6_SRGB_BLOCK => vk::Format::ASTC_10X6_SRGB_BLOCK,
            Self::ASTC_10X6_UNORM_BLOCK => vk::Format::ASTC_10X6_UNORM_BLOCK,
            Self::ASTC_10X8_SRGB_BLOCK => vk::Format::ASTC_10X8_SRGB_BLOCK,
            Self::ASTC_10X8_UNORM_BLOCK => vk::Format::ASTC_10X8_UNORM_BLOCK,
            Self::ASTC_12X10_SRGB_BLOCK => vk::Format::ASTC_12X10_SRGB_BLOCK,
            Self::ASTC_12X10_UNORM_BLOCK => vk::Format::ASTC_12X10_UNORM_BLOCK,
            Self::ASTC_12X12_SRGB_BLOCK => vk::Format::ASTC_12X12_SRGB_BLOCK,
            Self::ASTC_12X12_UNORM_BLOCK => vk::Format::ASTC_12X12_UNORM_BLOCK,
            Self::ASTC_4X4_SRGB_BLOCK => vk::Format::ASTC_4X4_SRGB_BLOCK,
            Self::ASTC_4X4_UNORM_BLOCK => vk::Format::ASTC_4X4_UNORM_BLOCK,
            Self::ASTC_5X4_SRGB_BLOCK => vk::Format::ASTC_5X4_SRGB_BLOCK,
            Self::ASTC_5X4_UNORM_BLOCK => vk::Format::ASTC_5X4_UNORM_BLOCK,
            Self::ASTC_5X5_SRGB_BLOCK => vk::Format::ASTC_5X5_SRGB_BLOCK,
            Self::ASTC_5X5_UNORM_BLOCK => vk::Format::ASTC_5X5_UNORM_BLOCK,
            Self::ASTC_6X5_SRGB_BLOCK => vk::Format::ASTC_6X5_SRGB_BLOCK,
            Self::ASTC_6X5_UNORM_BLOCK => vk::Format::ASTC_6X5_UNORM_BLOCK,
            Self::ASTC_6X6_SRGB_BLOCK => vk::Format::ASTC_6X6_SRGB_BLOCK,
            Self::ASTC_6X6_UNORM_BLOCK => vk::Format::ASTC_6X6_UNORM_BLOCK,
            Self::ASTC_8X5_SRGB_BLOCK => vk::Format::ASTC_8X5_SRGB_BLOCK,
            Self::ASTC_8X5_UNORM_BLOCK => vk::Format::ASTC_8X5_UNORM_BLOCK,
            Self::ASTC_8X6_SRGB_BLOCK => vk::Format::ASTC_8X6_SRGB_BLOCK,
            Self::ASTC_8X6_UNORM_BLOCK => vk::Format::ASTC_8X6_UNORM_BLOCK,
            Self::ASTC_8X8_SRGB_BLOCK => vk::Format::ASTC_8X8_SRGB_BLOCK,
            Self::ASTC_8X8_UNORM_BLOCK => vk::Format::ASTC_8X8_UNORM_BLOCK,
            Self::B10G11R11_UFLOAT_PACK32 => vk::Format::B10G11R11_UFLOAT_PACK32,
            Self::B4G4R4A4_UNORM_PACK16 => vk::Format::B4G4R4A4_UNORM_PACK16,
            Self::B5G5R5A1_UNORM_PACK16 => vk::Format::B5G5R5A1_UNORM_PACK16,
            Self::B5G6R5_UNORM_PACK16 => vk::Format::B5G6R5_UNORM_PACK16,
            Self::B8G8R8A8_SINT => vk::Format::B8G8R8A8_SINT,
            Self::B8G8R8A8_SNORM => vk::Format::B8G8R8A8_SNORM,
            Self::B8G8R8A8_SRGB => vk::Format::B8G8R8A8_SRGB,
            Self::B8G8R8A8_SSCALED => vk::Format::B8G8R8A8_SSCALED,
            Self::B8G8R8A8_UINT => vk::Format::B8G8R8A8_UINT,
            Self::B8G8R8A8_UNORM => vk::Format::B8G8R8A8_UNORM,
            Self::B8G8R8A8_USCALED => vk::Format::B8G8R8A8_USCALED,
            Self::B8G8R8_SINT => vk::Format::B8G8R8_SINT,
            Self::B8G8R8_SNORM => vk::Format::B8G8R8_SNORM,
            Self::B8G8R8_SRGB => vk::Format::B8G8R8_SRGB,
            Self::B8G8R8_SSCALED => vk::Format::B8G8R8_SSCALED,
            Self::B8G8R8_UINT => vk::Format::B8G8R8_UINT,
            Self::B8G8R8_UNORM => vk::Format::B8G8R8_UNORM,
            Self::B8G8R8_USCALED => vk::Format::B8G8R8_USCALED,
            Self::BC1_RGBA_SRGB_BLOCK => vk::Format::BC1_RGBA_SRGB_BLOCK,
            Self::BC1_RGBA_UNORM_BLOCK => vk::Format::BC1_RGBA_UNORM_BLOCK,
            Self::BC1_RGB_SRGB_BLOCK => vk::Format::BC1_RGB_SRGB_BLOCK,
            Self::BC1_RGB_UNORM_BLOCK => vk::Format::BC1_RGB_UNORM_BLOCK,
            Self::BC2_SRGB_BLOCK => vk::Format::BC2_SRGB_BLOCK,
            Self::BC2_UNORM_BLOCK => vk::Format::BC2_UNORM_BLOCK,
            Self::BC3_SRGB_BLOCK => vk::Format::BC3_SRGB_BLOCK,
            Self::BC3_UNORM_BLOCK => vk::Format::BC3_UNORM_BLOCK,
            Self::BC4_SNORM_BLOCK => vk::Format::BC4_SNORM_BLOCK,
            Self::BC4_UNORM_BLOCK => vk::Format::BC4_UNORM_BLOCK,
            Self::BC5_SNORM_BLOCK => vk::Format::BC5_SNORM_BLOCK,
            Self::BC5_UNORM_BLOCK => vk::Format::BC5_UNORM_BLOCK,
            Self::BC6H_SFLOAT_BLOCK => vk::Format::BC6H_SFLOAT_BLOCK,
            Self::BC6H_UFLOAT_BLOCK => vk::Format::BC6H_UFLOAT_BLOCK,
            Self::BC7_SRGB_BLOCK => vk::Format::BC7_SRGB_BLOCK,
            Self::BC7_UNORM_BLOCK => vk::Format::BC7_UNORM_BLOCK,
            Self::D16_UNORM => vk::Format::D16_UNORM,
            Self::D16_UNORM_S8_UINT => vk::Format::D16_UNORM_S8_UINT,
            Self::D24_UNORM_S8_UINT => vk::Format::D24_UNORM_S8_UINT,
            Self::D32_SFLOAT => vk::Format::D32_SFLOAT,
            Self::D32_SFLOAT_S8_UINT => vk::Format::D32_SFLOAT_S8_UINT,
            Self::E5B9G9R9_UFLOAT_PACK32 => vk::Format::E5B9G9R9_UFLOAT_PACK32,
            Self::EAC_R11G11_SNORM_BLOCK => vk::Format::EAC_R11G11_SNORM_BLOCK,
            Self::EAC_R11G11_UNORM_BLOCK => vk::Format::EAC_R11G11_UNORM_BLOCK,
            Self::EAC_R11_SNORM_BLOCK => vk::Format::EAC_R11_SNORM_BLOCK,
            Self::EAC_R11_UNORM_BLOCK => vk::Format::EAC_R11_UNORM_BLOCK,
            Self::ETC2_R8G8B8A1_SRGB_BLOCK => vk::Format::ETC2_R8G8B8A1_SRGB_BLOCK,
            Self::ETC2_R8G8B8A1_UNORM_BLOCK => vk::Format::ETC2_R8G8B8A1_UNORM_BLOCK,
            Self::ETC2_R8G8B8A8_SRGB_BLOCK => vk::Format::ETC2_R8G8B8A8_SRGB_BLOCK,
            Self::ETC2_R8G8B8A8_UNORM_BLOCK => vk::Format::ETC2_R8G8B8A8_UNORM_BLOCK,
            Self::ETC2_R8G8B8_SRGB_BLOCK => vk::Format::ETC2_R8G8B8_SRGB_BLOCK,
            Self::ETC2_R8G8B8_UNORM_BLOCK => vk::Format::ETC2_R8G8B8_UNORM_BLOCK,
            Self::R16G16B16A16_SFLOAT => vk::Format::R16G16B16A16_SFLOAT,
            Self::R16G16B16A16_SINT => vk::Format::R16G16B16A16_SINT,
            Self::R16G16B16A16_SNORM => vk::Format::R16G16B16A16_SNORM,
            Self::R16G16B16A16_SSCALED => vk::Format::R16G16B16A16_SSCALED,
            Self::R16G16B16A16_UINT => vk::Format::R16G16B16A16_UINT,
            Self::R16G16B16A16_UNORM => vk::Format::R16G16B16A16_UNORM,
            Self::R16G16B16A16_USCALED => vk::Format::R16G16B16A16_USCALED,
            Self::R16G16B16_SFLOAT => vk::Format::R16G16B16_SFLOAT,
            Self::R16G16B16_SINT => vk::Format::R16G16B16_SINT,
            Self::R16G16B16_SNORM => vk::Format::R16G16B16_SNORM,
            Self::R16G16B16_SSCALED => vk::Format::R16G16B16_SSCALED,
            Self::R16G16B16_UINT => vk::Format::R16G16B16_UINT,
            Self::R16G16B16_UNORM => vk::Format::R16G16B16_UNORM,
            Self::R16G16B16_USCALED => vk::Format::R16G16B16_USCALED,
            Self::R16G16_SFLOAT => vk::Format::R16G16_SFLOAT,
            Self::R16G16_SINT => vk::Format::R16G16_SINT,
            Self::R16G16_SNORM => vk::Format::R16G16_SNORM,
            Self::R16G16_SSCALED => vk::Format::R16G16_SSCALED,
            Self::R16G16_UINT => vk::Format::R16G16_UINT,
            Self::R16G16_UNORM => vk::Format::R16G16_UNORM,
            Self::R16G16_USCALED => vk::Format::R16G16_USCALED,
            Self::R16_SFLOAT => vk::Format::R16_SFLOAT,
            Self::R16_SINT => vk::Format::R16_SINT,
            Self::R16_SNORM => vk::Format::R16_SNORM,
            Self::R16_SSCALED => vk::Format::R16_SSCALED,
            Self::R16_UINT => vk::Format::R16_UINT,
            Self::R16_UNORM => vk::Format::R16_UNORM,
            Self::R16_USCALED => vk::Format::R16_USCALED,
            Self::R32G32B32A32_SFLOAT => vk::Format::R32G32B32A32_SFLOAT,
            Self::R32G32B32A32_SINT => vk::Format::R32G32B32A32_SINT,
            Self::R32G32B32A32_UINT => vk::Format::R32G32B32A32_UINT,
            Self::R32G32B32_SFLOAT => vk::Format::R32G32B32_SFLOAT,
            Self::R32G32B32_SINT => vk::Format::R32G32B32_SINT,
            Self::R32G32B32_UINT => vk::Format::R32G32B32_UINT,
            Self::R32G32_SFLOAT => vk::Format::R32G32_SFLOAT,
            Self::R32G32_SINT => vk::Format::R32G32_SINT,
            Self::R32G32_UINT => vk::Format::R32G32_UINT,
            Self::R32_SFLOAT => vk::Format::R32_SFLOAT,
            Self::R32_SINT => vk::Format::R32_SINT,
            Self::R32_UINT => vk::Format::R32_UINT,
            Self::R4G4B4A4_UNORM_PACK16 => vk::Format::R4G4B4A4_UNORM_PACK16,
            Self::R4G4_UNORM_PACK8 => vk::Format::R4G4_UNORM_PACK8,
            Self::R5G5B5A1_UNORM_PACK16 => vk::Format::R5G5B5A1_UNORM_PACK16,
            Self::R5G6B5_UNORM_PACK16 => vk::Format::R5G6B5_UNORM_PACK16,
            Self::R64G64B64A64_SFLOAT => vk::Format::R64G64B64A64_SFLOAT,
            Self::R64G64B64A64_SINT => vk::Format::R64G64B64A64_SINT,
            Self::R64G64B64A64_UINT => vk::Format::R64G64B64A64_UINT,
            Self::R64G64B64_SFLOAT => vk::Format::R64G64B64_SFLOAT,
            Self::R64G64B64_SINT => vk::Format::R64G64B64_SINT,
            Self::R64G64B64_UINT => vk::Format::R64G64B64_UINT,
            Self::R64G64_SFLOAT => vk::Format::R64G64_SFLOAT,
            Self::R64G64_SINT => vk::Format::R64G64_SINT,
            Self::R64G64_UINT => vk::Format::R64G64_UINT,
            Self::R64_SFLOAT => vk::Format::R64_SFLOAT,
            Self::R64_SINT => vk::Format::R64_SINT,
            Self::R64_UINT => vk::Format::R64_UINT,
            Self::R8G8B8A8_SINT => vk::Format::R8G8B8A8_SINT,
            Self::R8G8B8A8_SNORM => vk::Format::R8G8B8A8_SNORM,
            Self::R8G8B8A8_SRGB => vk::Format::R8G8B8A8_SRGB,
            Self::R8G8B8A8_SSCALED => vk::Format::R8G8B8A8_SSCALED,
            Self::R8G8B8A8_UINT => vk::Format::R8G8B8A8_UINT,
            Self::R8G8B8A8_UNORM => vk::Format::R8G8B8A8_UNORM,
            Self::R8G8B8A8_USCALED => vk::Format::R8G8B8A8_USCALED,
            Self::R8G8B8_SINT => vk::Format::R8G8B8_SINT,
            Self::R8G8B8_SNORM => vk::Format::R8G8B8_SNORM,
            Self::R8G8B8_SRGB => vk::Format::R8G8B8_SRGB,
            Self::R8G8B8_SSCALED => vk::Format::R8G8B8_SSCALED,
            Self::R8G8B8_UINT => vk::Format::R8G8B8_UINT,
            Self::R8G8B8_UNORM => vk::Format::R8G8B8_UNORM,
            Self::R8G8B8_USCALED => vk::Format::R8G8B8_USCALED,
            Self::R8G8_SINT => vk::Format::R8G8_SINT,
            Self::R8G8_SNORM => vk::Format::R8G8_SNORM,
            Self::R8G8_SRGB => vk::Format::R8G8_SRGB,
            Self::R8G8_SSCALED => vk::Format::R8G8_SSCALED,
            Self::R8G8_UINT => vk::Format::R8G8_UINT,
            Self::R8G8_UNORM => vk::Format::R8G8_UNORM,
            Self::R8G8_USCALED => vk::Format::R8G8_USCALED,
            Self::R8_SINT => vk::Format::R8_SINT,
            Self::R8_SNORM => vk::Format::R8_SNORM,
            Self::R8_SRGB => vk::Format::R8_SRGB,
            Self::R8_SSCALED => vk::Format::R8_SSCALED,
            Self::R8_UINT => vk::Format::R8_UINT,
            Self::R8_UNORM => vk::Format::R8_UNORM,
            Self::R8_USCALED => vk::Format::R8_USCALED,
            Self::S8_UINT => vk::Format::S8_UINT,
            Self::X8_D24_UNORM_PACK32 => vk::Format::X8_D24_UNORM_PACK32,
        }
    }
}
