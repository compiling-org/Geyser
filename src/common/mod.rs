//! Common types and traits used across different graphics APIs.

use std::fmt;

/// Represents the intended usage of a texture, influencing how it's created and shared.
/// This is similar to `TextureUsage` in WebGPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureUsage {
    /// Texture can be copied from (source).
    CopySrc,
    /// Texture can be copied to (destination).
    CopyDst,
    /// Texture can be sampled by shaders.
    TextureBinding,
    /// Texture can be used as a render target.
    RenderAttachment,
    /// Texture can be written to by compute shaders (storage texture).
    StorageBinding,
    // Add more as necessary, e.g., Present, etc.
}

/// Represents a texture format, abstracting over API-specific enums.
/// Supports common color, depth, and HDR formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureFormat {
    // 8-bit formats
    Rgba8Unorm,
    Bgra8Unorm,
    Rgba8Srgb,
    Bgra8Srgb,
    R8Unorm,
    Rg8Unorm,
    
    // 16-bit formats
    R16Float,
    Rg16Float,
    Rgba16Float,
    R16Uint,
    R16Sint,
    
    // 32-bit formats
    R32Float,
    Rg32Float,
    Rgba32Float,
    R32Uint,
    R32Sint,
    
    // Depth/Stencil formats
    Depth32Float,
    Depth24Plus,
    Depth24PlusStencil8,
    
    // HDR formats
    Rgb10a2Unorm,
    Rg11b10Float,
}

impl fmt::Display for TextureFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// A descriptor for creating a new shareable texture.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextureDescriptor {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub usage: Vec<TextureUsage>, // A texture can have multiple usages
    pub label: Option<String>,
}

/// Opaque handle for sharing textures between APIs or processes.
/// This will contain API-specific details like Vulkan external memory handles, Metal IOSurfaceIDs, etc.
/// This needs to be serializable/deserializable to pass between processes.
#[derive(Debug, Clone)]
pub enum ApiTextureHandle {
    #[cfg(feature = "vulkan")]
    Vulkan(crate::vulkan::VulkanTextureShareHandle),
    #[cfg(feature = "metal")]
    Metal(crate::metal::MetalTextureShareHandle),
    // #[cfg(feature = "webgpu")]
    // WebGpu(crate::webgpu::WebGpuTextureShareHandle),
    // Add more variants for other APIs
}

/// Handle for sharing synchronization primitives between processes.
/// Used to coordinate GPU access to shared textures.
#[derive(Debug, Clone)]
pub enum SyncHandle {
    #[cfg(feature = "vulkan")]
    VulkanSemaphore(crate::vulkan::VulkanSemaphoreHandle),
    #[cfg(feature = "vulkan")]
    VulkanFence(crate::vulkan::VulkanFenceHandle),
    #[cfg(feature = "metal")]
    MetalEvent(crate::metal::MetalEventHandle),
}

/// Synchronization primitives associated with a shared texture.
/// Used for coordinating access between multiple processes or contexts.
#[derive(Debug, Clone)]
pub struct SyncPrimitives {
    /// Optional semaphore for signaling when texture is ready
    pub semaphore: Option<SyncHandle>,
    /// Optional fence for CPU-side synchronization
    pub fence: Option<SyncHandle>,
}

impl Default for SyncPrimitives {
    fn default() -> Self {
        Self {
            semaphore: None,
            fence: None,
        }
    }
}

#[cfg(test)]
mod tests;
