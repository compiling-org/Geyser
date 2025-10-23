//! Geyser: High-performance GPU texture sharing library.

pub mod common;
pub mod error;

#[cfg(feature = "vulkan")]
pub mod vulkan;

#[cfg(feature = "metal")]
pub mod metal;

#[cfg(feature = "webgpu")]
pub mod webgpu; // Placeholder for future WebGPU implementation

// Bevy integration (optional)
#[cfg(all(feature = "vulkan", feature = "bevy"))]
pub mod bevy_plugin;

pub use error::{GeyserError, Result};
pub use common::{ApiTextureHandle, TextureDescriptor, TextureFormat, TextureUsage};

use std::any::Any;

/// A trait representing a texture that can be shared or has been imported.
/// This will be an API-specific wrapper around a raw texture handle.
pub trait SharedTexture {
    /// Returns the width of the texture.
    fn width(&self) -> u32;
    /// Returns the height of the texture.
    fn height(&self) -> u32;
    /// Returns the format of the texture.
    fn format(&self) -> TextureFormat;
    /// Returns the usage flags of the texture.
    fn usage(&self) -> &[TextureUsage];

    /// Helper for downcasting to concrete types.
    fn as_any(&self) -> &dyn Any;
    
    // Potentially add methods to get native handles for API-specific use,
    // but keep it as minimal as possible to maintain abstraction.
    // E.g., `fn as_vulkan_image(&self) -> Option<&VulkanImage>`
}

/// A manager for creating, exporting, and importing shared textures within a specific graphics API context.
/// Each backend (Vulkan, Metal) will have its own concrete implementation of this trait.
pub trait TextureShareManager {
    /// Creates a new texture that can be exported for sharing.
    /// This texture is owned by the current API context.
    fn create_shareable_texture(&self, descriptor: &TextureDescriptor) -> Result<Box<dyn SharedTexture>>;

    /// Exports an existing shareable texture, returning an `ApiTextureHandle` that can be
    /// serialized and sent to another process or API context.
    /// The manager is responsible for keeping the underlying resource alive until explicitly released.
    fn export_texture(&self, texture: &dyn SharedTexture) -> Result<ApiTextureHandle>;

    /// Imports a texture using a provided `ApiTextureHandle`, making it usable within
    /// the current API context.
    /// This returns a `SharedTexture` wrapper that can be used like any other texture
    /// in this context. The manager takes ownership of the imported resource for its lifetime.
    fn import_texture(&self, handle: ApiTextureHandle, descriptor: &TextureDescriptor) -> Result<Box<dyn SharedTexture>>;

    /// Releases any resources associated with a previously exported or imported texture handle.
    /// This should be called when the shared texture is no longer needed in this context.
    fn release_texture_handle(&self, handle: ApiTextureHandle) -> Result<()>;
}
