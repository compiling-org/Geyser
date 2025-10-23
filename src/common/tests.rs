//! Unit tests for common types

use super::*;

#[test]
fn test_texture_descriptor_creation() {
    let desc = TextureDescriptor {
        width: 1920,
        height: 1080,
        format: TextureFormat::Rgba8Unorm,
        usage: vec![TextureUsage::RenderAttachment, TextureUsage::TextureBinding],
        label: Some("TestTexture".to_string()),
    };

    assert_eq!(desc.width, 1920);
    assert_eq!(desc.height, 1080);
    assert_eq!(desc.format, TextureFormat::Rgba8Unorm);
    assert_eq!(desc.usage.len(), 2);
    assert!(desc.usage.contains(&TextureUsage::RenderAttachment));
    assert!(desc.usage.contains(&TextureUsage::TextureBinding));
}

#[test]
fn test_texture_format_equality() {
    assert_eq!(TextureFormat::Rgba8Unorm, TextureFormat::Rgba8Unorm);
    assert_ne!(TextureFormat::Rgba8Unorm, TextureFormat::Bgra8Unorm);
}

#[test]
fn test_texture_format_display() {
    let format = TextureFormat::Rgba8Unorm;
    let display_str = format!("{}", format);
    assert!(!display_str.is_empty());
    assert!(display_str.contains("Rgba8Unorm"));
}

#[test]
fn test_texture_usage_flags() {
    let usages = vec![
        TextureUsage::CopySrc,
        TextureUsage::CopyDst,
        TextureUsage::TextureBinding,
        TextureUsage::RenderAttachment,
        TextureUsage::StorageBinding,
    ];

    assert_eq!(usages.len(), 5);
    assert!(usages.contains(&TextureUsage::TextureBinding));
}

#[test]
fn test_sync_primitives_default() {
    let sync = SyncPrimitives::default();
    assert!(sync.semaphore.is_none());
    assert!(sync.fence.is_none());
}

#[test]
fn test_sync_primitives_with_values() {
    #[cfg(feature = "vulkan")]
    {
        use crate::vulkan::{VulkanSemaphoreHandle, VulkanFenceHandle};
        use ash::vk;

        let semaphore = SyncHandle::VulkanSemaphore(VulkanSemaphoreHandle {
            raw_handle: 12345,
            handle_type: vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_WIN32,
        });

        let fence = SyncHandle::VulkanFence(VulkanFenceHandle {
            raw_handle: 67890,
            handle_type: vk::ExternalFenceHandleTypeFlags::OPAQUE_WIN32,
        });

        let sync = SyncPrimitives {
            semaphore: Some(semaphore),
            fence: Some(fence),
        };

        assert!(sync.semaphore.is_some());
        assert!(sync.fence.is_some());
    }
}

#[test]
fn test_texture_descriptor_clone() {
    let desc1 = TextureDescriptor {
        width: 512,
        height: 512,
        format: TextureFormat::R16Float,
        usage: vec![TextureUsage::StorageBinding],
        label: Some("Clone Test".to_string()),
    };

    let desc2 = desc1.clone();

    assert_eq!(desc1.width, desc2.width);
    assert_eq!(desc1.height, desc2.height);
    assert_eq!(desc1.format, desc2.format);
    assert_eq!(desc1.usage, desc2.usage);
    assert_eq!(desc1.label, desc2.label);
}

#[test]
fn test_all_texture_formats_exist() {
    // Ensure all documented formats can be instantiated
    let formats = vec![
        // 8-bit
        TextureFormat::Rgba8Unorm,
        TextureFormat::Bgra8Unorm,
        TextureFormat::Rgba8Srgb,
        TextureFormat::Bgra8Srgb,
        TextureFormat::R8Unorm,
        TextureFormat::Rg8Unorm,
        // 16-bit
        TextureFormat::R16Float,
        TextureFormat::Rg16Float,
        TextureFormat::Rgba16Float,
        TextureFormat::R16Uint,
        TextureFormat::R16Sint,
        // 32-bit
        TextureFormat::R32Float,
        TextureFormat::Rg32Float,
        TextureFormat::Rgba32Float,
        TextureFormat::R32Uint,
        TextureFormat::R32Sint,
        // Depth/Stencil
        TextureFormat::Depth32Float,
        TextureFormat::Depth24Plus,
        TextureFormat::Depth24PlusStencil8,
        // HDR
        TextureFormat::Rgb10a2Unorm,
        TextureFormat::Rg11b10Float,
    ];

    assert_eq!(formats.len(), 21);
}

#[test]
fn test_texture_descriptor_hash() {
    use std::collections::HashSet;

    let desc1 = TextureDescriptor {
        width: 256,
        height: 256,
        format: TextureFormat::Rgba8Unorm,
        usage: vec![TextureUsage::TextureBinding],
        label: Some("A".to_string()),
    };

    let desc2 = TextureDescriptor {
        width: 256,
        height: 256,
        format: TextureFormat::Rgba8Unorm,
        usage: vec![TextureUsage::TextureBinding],
        label: Some("B".to_string()),
    };

    let mut set = HashSet::new();
    set.insert(desc1);
    set.insert(desc2);

    // Even though labels differ, they should hash differently
    assert_eq!(set.len(), 2);
}
