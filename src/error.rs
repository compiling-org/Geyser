//! Custom error types for the Geyser library.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, GeyserError>;

#[derive(Error, Debug)]
pub enum GeyserError {
    #[error("Failed to initialize Vulkan: {0}")]
    VulkanInitializationError(String),
    #[error("Vulkan API error: {0}")]
    VulkanApiError(String),
    #[error("Failed to initialize Metal: {0}")]
    MetalInitializationError(String),
    #[error("Metal API error: {0}")]
    MetalApiError(String),
    #[error("Unsupported texture format: {0}")]
    UnsupportedTextureFormat(String),
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    #[error("Not implemented: {0}")]
    NotImplemented(String),
    #[error("Invalid handle provided for import/export")]
    InvalidTextureHandle,
    #[error("Resource already in use or cannot be shared")]
    ResourceInUse,
    #[error("Operation not supported on current platform or API")]
    OperationNotSupported,
    #[error("Other error: {0}")]
    Other(String),
}

#[cfg(feature = "vulkan")]
impl From<ash::vk::Result> for GeyserError {
    fn from(err: ash::vk::Result) -> Self {
        GeyserError::VulkanApiError(format!("Vulkan Error: {:?}", err))
    }
}

#[cfg(feature = "vulkan")]
impl From<gpu_allocator::AllocationError> for GeyserError {
    fn from(err: gpu_allocator::AllocationError) -> Self {
        GeyserError::VulkanApiError(format!("GPU Allocator Error: {:?}", err))
    }
}

// Add more `From` implementations for Metal, WebGPU, etc.
