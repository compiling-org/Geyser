//! wgpu-hal Bridge for Bevy Zero-Copy Texture Sharing
//!
//! This module provides utilities to bridge between Geyser's texture handles
//! and wgpu-hal, enabling zero-copy texture sharing with Bevy's render backend.
//!
//! # Architecture
//! - Format and usage conversion between Geyser and wgpu-hal types
//! - Safe wrapper around raw wgpu-hal texture handles
//! - Import/export of textures from Vulkan/Metal external handles
//!
//! # Platform Support
//! - Vulkan (via VK_KHR_external_memory)
//! - Metal (via IOSurface) - TODO
//! - D3D12 (via NT handles) - TODO

use crate::common::{TextureFormat, TextureUsage, TextureDescriptor};
use crate::error::GeyserError;

#[cfg(feature = "vulkan")]
use crate::vulkan::VulkanTextureShareHandle;

/// Convert Geyser TextureFormat to wgpu-types TextureFormat
pub fn to_wgpu_format(format: TextureFormat) -> wgpu_types::TextureFormat {
    match format {
        // 8-bit formats
        TextureFormat::Rgba8Unorm => wgpu_types::TextureFormat::Rgba8Unorm,
        TextureFormat::Bgra8Unorm => wgpu_types::TextureFormat::Bgra8Unorm,
        TextureFormat::Rgba8Srgb => wgpu_types::TextureFormat::Rgba8UnormSrgb,
        TextureFormat::Bgra8Srgb => wgpu_types::TextureFormat::Bgra8UnormSrgb,
        TextureFormat::R8Unorm => wgpu_types::TextureFormat::R8Unorm,
        TextureFormat::Rg8Unorm => wgpu_types::TextureFormat::Rg8Unorm,
        
        // 16-bit formats
        TextureFormat::R16Float => wgpu_types::TextureFormat::R16Float,
        TextureFormat::Rg16Float => wgpu_types::TextureFormat::Rg16Float,
        TextureFormat::Rgba16Float => wgpu_types::TextureFormat::Rgba16Float,
        TextureFormat::R16Uint => wgpu_types::TextureFormat::R16Uint,
        TextureFormat::R16Sint => wgpu_types::TextureFormat::R16Sint,
        
        // 32-bit formats
        TextureFormat::R32Float => wgpu_types::TextureFormat::R32Float,
        TextureFormat::Rg32Float => wgpu_types::TextureFormat::Rg32Float,
        TextureFormat::Rgba32Float => wgpu_types::TextureFormat::Rgba32Float,
        TextureFormat::R32Uint => wgpu_types::TextureFormat::R32Uint,
        TextureFormat::R32Sint => wgpu_types::TextureFormat::R32Sint,
        
        // Depth/Stencil formats
        TextureFormat::Depth32Float => wgpu_types::TextureFormat::Depth32Float,
        TextureFormat::Depth24Plus => wgpu_types::TextureFormat::Depth24Plus,
        TextureFormat::Depth24PlusStencil8 => wgpu_types::TextureFormat::Depth24PlusStencil8,
        
        // HDR formats
        TextureFormat::Rgb10a2Unorm => wgpu_types::TextureFormat::Rgb10a2Unorm,
        TextureFormat::Rg11b10Float => wgpu_types::TextureFormat::Rg11b10Ufloat,
    }
}

/// Convert Geyser TextureUsage to wgpu-types TextureUsages bitflags
pub fn to_wgpu_usage(usage: &[TextureUsage]) -> wgpu_types::TextureUsages {
    let mut wgpu_usage = wgpu_types::TextureUsages::empty();
    
    for u in usage {
        wgpu_usage |= match u {
            TextureUsage::CopySrc => wgpu_types::TextureUsages::COPY_SRC,
            TextureUsage::CopyDst => wgpu_types::TextureUsages::COPY_DST,
            TextureUsage::TextureBinding => wgpu_types::TextureUsages::TEXTURE_BINDING,
            TextureUsage::RenderAttachment => wgpu_types::TextureUsages::RENDER_ATTACHMENT,
            TextureUsage::StorageBinding => wgpu_types::TextureUsages::STORAGE_BINDING,
        };
    }
    
    wgpu_usage
}

/// Convert wgpu-types TextureFormat back to Geyser format
pub fn from_wgpu_format(format: wgpu_types::TextureFormat) -> Result<TextureFormat, GeyserError> {
    match format {
        wgpu_types::TextureFormat::Rgba8Unorm => Ok(TextureFormat::Rgba8Unorm),
        wgpu_types::TextureFormat::Bgra8Unorm => Ok(TextureFormat::Bgra8Unorm),
        wgpu_types::TextureFormat::Rgba8UnormSrgb => Ok(TextureFormat::Rgba8Srgb),
        wgpu_types::TextureFormat::Bgra8UnormSrgb => Ok(TextureFormat::Bgra8Srgb),
        wgpu_types::TextureFormat::R8Unorm => Ok(TextureFormat::R8Unorm),
        wgpu_types::TextureFormat::Rg8Unorm => Ok(TextureFormat::Rg8Unorm),
        wgpu_types::TextureFormat::R16Float => Ok(TextureFormat::R16Float),
        wgpu_types::TextureFormat::Rg16Float => Ok(TextureFormat::Rg16Float),
        wgpu_types::TextureFormat::Rgba16Float => Ok(TextureFormat::Rgba16Float),
        wgpu_types::TextureFormat::R16Uint => Ok(TextureFormat::R16Uint),
        wgpu_types::TextureFormat::R16Sint => Ok(TextureFormat::R16Sint),
        wgpu_types::TextureFormat::R32Float => Ok(TextureFormat::R32Float),
        wgpu_types::TextureFormat::Rg32Float => Ok(TextureFormat::Rg32Float),
        wgpu_types::TextureFormat::Rgba32Float => Ok(TextureFormat::Rgba32Float),
        wgpu_types::TextureFormat::R32Uint => Ok(TextureFormat::R32Uint),
        wgpu_types::TextureFormat::R32Sint => Ok(TextureFormat::R32Sint),
        wgpu_types::TextureFormat::Depth32Float => Ok(TextureFormat::Depth32Float),
        wgpu_types::TextureFormat::Depth24Plus => Ok(TextureFormat::Depth24Plus),
        wgpu_types::TextureFormat::Depth24PlusStencil8 => Ok(TextureFormat::Depth24PlusStencil8),
        wgpu_types::TextureFormat::Rgb10a2Unorm => Ok(TextureFormat::Rgb10a2Unorm),
        wgpu_types::TextureFormat::Rg11b10Ufloat => Ok(TextureFormat::Rg11b10Float),
        _ => Err(GeyserError::UnsupportedFormat(format!("Unsupported wgpu format: {:?}", format))),
    }
}

/// Safe wrapper around a wgpu-hal texture handle imported from external memory
pub struct WgpuTextureHandle {
    /// The raw wgpu-hal texture
    pub(crate) texture: Box<dyn std::any::Any + Send + Sync>,
    /// Texture descriptor
    pub descriptor: TextureDescriptor,
    /// Backend type (for safe downcasting)
    pub backend_type: WgpuBackendType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WgpuBackendType {
    Vulkan,
    Metal,
    Dx12,
    Gl,
}

impl WgpuTextureHandle {
    /// Create a new WgpuTextureHandle (internal use)
    pub(crate) fn new<T: 'static + Send + Sync>(
        texture: T,
        descriptor: TextureDescriptor,
        backend_type: WgpuBackendType,
    ) -> Self {
        Self {
            texture: Box::new(texture),
            descriptor,
            backend_type,
        }
    }
    
    /// Try to get the underlying Vulkan texture
    #[cfg(feature = "vulkan")]
    pub fn as_vulkan_raw(&self) -> Option<*const std::ffi::c_void> {
        if self.backend_type == WgpuBackendType::Vulkan {
            // Placeholder: actual raw handle exposure TBD
            None
        } else {
            None
        }
    }
}

/// Import a Geyser texture handle into wgpu-hal
/// This creates a wgpu-hal texture from an external memory handle
#[cfg(feature = "vulkan")]
pub fn import_vulkan_texture(
    _handle: &VulkanTextureShareHandle,
    _descriptor: &TextureDescriptor,
) -> Result<WgpuTextureHandle, GeyserError> {
    // The stable path is to import via raw Vulkan first and then wrap into wgpu.
    // Direct import into wgpu-hal is not exposed on stable APIs.
    Err(GeyserError::NotImplemented(
        "Direct import into wgpu-hal is not exposed; use Vulkan import then wrap".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_conversion_roundtrip() {
        let formats = vec![
            TextureFormat::Rgba8Unorm,
            TextureFormat::Bgra8Unorm,
            TextureFormat::R32Float,
            TextureFormat::Depth32Float,
        ];
        
        for format in formats {
            let wgpu_format = to_wgpu_format(format);
            let back = from_wgpu_format(wgpu_format).unwrap();
            assert_eq!(format, back);
        }
    }
    
    #[test]
    fn test_usage_conversion() {
        let usage = vec![
            TextureUsage::CopySrc,
            TextureUsage::TextureBinding,
            TextureUsage::RenderAttachment,
        ];
        
        let wgpu_usage = to_wgpu_usage(&usage);
        
        assert!(wgpu_usage.contains(wgpu_types::TextureUsages::COPY_SRC));
        assert!(wgpu_usage.contains(wgpu_types::TextureUsages::TEXTURE_BINDING));
        assert!(wgpu_usage.contains(wgpu_types::TextureUsages::RENDER_ATTACHMENT));
    }
}
