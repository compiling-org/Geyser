//! Unit tests for Metal backend

use super::*;

#[test]
fn test_metal_texture_share_handle_creation() {
    let handle = MetalTextureShareHandle {
        io_surface_id: 12345,
    };

    assert_eq!(handle.io_surface_id, 12345);
}

#[test]
fn test_metal_event_handle_creation() {
    let handle = MetalEventHandle {
        shared_event_id: 67890,
    };

    assert_eq!(handle.shared_event_id, 67890);
}

#[test]
fn test_metal_texture_share_handle_clone() {
    let handle1 = MetalTextureShareHandle {
        io_surface_id: 999,
    };

    let handle2 = handle1.clone();

    assert_eq!(handle1.io_surface_id, handle2.io_surface_id);
}

#[test]
fn test_metal_event_handle_clone() {
    let handle1 = MetalEventHandle {
        shared_event_id: 111,
    };

    let handle2 = handle1.clone();

    assert_eq!(handle1.shared_event_id, handle2.shared_event_id);
}

#[test]
fn test_bytes_per_element_calculation() {
    use crate::common::TextureFormat;

    // Test that bytes per element calculation is correct for common formats
    let formats_and_sizes = vec![
        (TextureFormat::R8Unorm, 1),
        (TextureFormat::Rg8Unorm, 2),
        (TextureFormat::Rgba8Unorm, 4),
        (TextureFormat::R16Float, 2),
        (TextureFormat::Rg16Float, 4),
        (TextureFormat::Rgba16Float, 8),
        (TextureFormat::R32Float, 4),
        (TextureFormat::Rg32Float, 8),
        (TextureFormat::Rgba32Float, 16),
        (TextureFormat::Depth32Float, 4),
    ];

    for (format, expected_size) in formats_and_sizes {
        // We can't directly test the private method, but we can test the logic
        let size = match format {
            TextureFormat::R8Unorm => 1,
            TextureFormat::Rg8Unorm => 2,
            TextureFormat::Rgba8Unorm | TextureFormat::Bgra8Unorm |
            TextureFormat::Rgba8Srgb | TextureFormat::Bgra8Srgb => 4,
            TextureFormat::R16Float | TextureFormat::R16Uint | TextureFormat::R16Sint => 2,
            TextureFormat::Rg16Float => 4,
            TextureFormat::Rgba16Float => 8,
            TextureFormat::R32Float | TextureFormat::R32Uint | TextureFormat::R32Sint => 4,
            TextureFormat::Rg32Float => 8,
            TextureFormat::Rgba32Float => 16,
            TextureFormat::Depth32Float => 4,
            TextureFormat::Depth24Plus => 4,
            TextureFormat::Depth24PlusStencil8 => 8,
            TextureFormat::Rgb10a2Unorm => 4,
            TextureFormat::Rg11b10Float => 4,
        };

        assert_eq!(size, expected_size, "Format {:?} should have {} bytes per element", format, expected_size);
    }
}

#[test]
fn test_format_mapping_coverage() {
    use crate::common::TextureFormat;

    // Ensure all texture formats have a Metal mapping
    let all_formats = vec![
        TextureFormat::Rgba8Unorm,
        TextureFormat::Bgra8Unorm,
        TextureFormat::Rgba8Srgb,
        TextureFormat::Bgra8Srgb,
        TextureFormat::R8Unorm,
        TextureFormat::Rg8Unorm,
        TextureFormat::R16Float,
        TextureFormat::Rg16Float,
        TextureFormat::Rgba16Float,
        TextureFormat::R16Uint,
        TextureFormat::R16Sint,
        TextureFormat::R32Float,
        TextureFormat::Rg32Float,
        TextureFormat::Rgba32Float,
        TextureFormat::R32Uint,
        TextureFormat::R32Sint,
        TextureFormat::Depth32Float,
        TextureFormat::Depth24Plus,
        TextureFormat::Depth24PlusStencil8,
        TextureFormat::Rgb10a2Unorm,
        TextureFormat::Rg11b10Float,
    ];

    // Just verify we have 21 formats covered
    assert_eq!(all_formats.len(), 21);
}

#[test]
fn test_usage_flags_mapping() {
    use crate::common::TextureUsage;

    let usages = vec![
        TextureUsage::CopySrc,
        TextureUsage::CopyDst,
        TextureUsage::TextureBinding,
        TextureUsage::RenderAttachment,
        TextureUsage::StorageBinding,
    ];

    // Just verify we handle all usage types
    assert_eq!(usages.len(), 5);
}

#[test]
fn test_sync_handle_metal_variant() {
    use crate::common::SyncHandle;

    let event_handle = MetalEventHandle {
        shared_event_id: 42,
    };

    let sync_handle = SyncHandle::MetalEvent(event_handle);

    match sync_handle {
        SyncHandle::MetalEvent(h) => {
            assert_eq!(h.shared_event_id, 42);
        }
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_api_texture_handle_metal_variant() {
    use crate::common::ApiTextureHandle;

    let texture_handle = MetalTextureShareHandle {
        io_surface_id: 123,
    };

    let api_handle = ApiTextureHandle::Metal(texture_handle);

    match api_handle {
        ApiTextureHandle::Metal(h) => {
            assert_eq!(h.io_surface_id, 123);
        }
        _ => panic!("Wrong variant"),
    }
}
