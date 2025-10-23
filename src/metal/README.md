# Metal Backend for Geyser

## Overview

The Metal backend provides high-performance texture sharing on macOS and iOS using **IOSurface**. IOSurface is Apple's framework for sharing GPU textures across processes and between different graphics APIs (Metal, OpenGL, Vulkan via MoltenVK).

## Features

### Texture Sharing
- ✅ Create shareable textures backed by IOSurface
- ✅ Export textures to IOSurface IDs for cross-process sharing
- ✅ Import textures from IOSurface IDs
- ✅ Automatic resource lifetime management
- ✅ Support for all common texture formats (21 formats)
- ✅ Configurable texture usage flags

### Synchronization
- ✅ MTLSharedEvent for cross-process GPU synchronization
- ✅ Event creation, export, import, and signaling
- ✅ CPU-side event waiting
- ✅ GPU-side synchronization via command buffer encoding

### Storage Modes
- Uses `MTLStorageMode::Shared` for IOSurface-backed textures
- Allows both CPU and GPU access
- Optimized for cross-process and cross-API sharing

## Platform Requirements

- **macOS**: 10.13 (High Sierra) or later
- **iOS**: 11.0 or later
- **Hardware**: Any Mac with Metal-capable GPU
- **Xcode**: Required for building on macOS

## Building

The Metal backend can only be compiled on macOS:

```bash
# On macOS
cargo build --features metal
cargo test --features metal
```

On other platforms (Windows, Linux), the Metal feature will fail to compile as it depends on macOS-specific frameworks (`metal`, `core-graphics`).

## API Usage

### Creating a Shared Texture

```rust
use geyser::{
    metal::MetalTextureShareManager,
    common::{TextureDescriptor, TextureFormat, TextureUsage},
};
use metal::Device;
use std::sync::Arc;

// Get Metal device
let device = Arc::new(Device::system_default().expect("No Metal device"));

// Create manager
let manager = MetalTextureShareManager::new(device)?;

// Create texture descriptor
let descriptor = TextureDescriptor {
    width: 1920,
    height: 1080,
    format: TextureFormat::Rgba8Unorm,
    usage: vec![TextureUsage::RenderAttachment, TextureUsage::TextureBinding],
    label: Some("SharedTexture".to_string()),
};

// Create shareable texture
let texture = manager.create_shareable_texture(&descriptor)?;
```

### Exporting a Texture

```rust
// Export texture to get IOSurface ID
let handle = manager.export_texture(texture.as_ref())?;

// Share the handle between processes (e.g., via IPC)
// The handle contains the IOSurface ID which can be sent as a u32
```

### Importing a Texture

```rust
// In another process with the IOSurface ID
let imported_texture = manager.import_texture(handle, &descriptor)?;

// Use the imported texture
```

### Synchronization

```rust
// Create a shared event
let event = manager.create_shared_event()?;

// Export for sharing
let event_handle = manager.export_shared_event(&event)?;

// Signal the event
manager.signal_event(&event, 1);

// Wait for event (in another process after importing)
let imported_event = manager.import_shared_event(&event_handle)?;
manager.wait_for_event(&imported_event, 1)?;
```

## Implementation Details

### IOSurface
- Each shareable texture is backed by an IOSurface
- IOSurface IDs are used for cross-process identification
- Surfaces are kept alive via reference counting in `exported_surfaces`

### Shared Events
- MTLSharedEvent provides timeline-based synchronization
- Events can be signaled from CPU or GPU
- Shared event handles enable cross-process synchronization

### Format Support
All 21 common texture formats are supported:
- 8-bit: RGBA8, BGRA8, R8, RG8 (unorm and sRGB variants)
- 16-bit: R16, RG16, RGBA16 (float, uint, sint)
- 32-bit: R32, RG32, RGBA32 (float, uint, sint)
- Depth/Stencil: Depth32Float, Depth24Plus, Depth24PlusStencil8
- HDR: RGB10A2, RG11B10

### Thread Safety
- All managers use `Mutex` for internal state
- Safe for concurrent access from multiple threads
- IOSurface and MTLSharedEvent are inherently thread-safe

## Cross-API Interoperability

The Metal backend can share textures with:

1. **Vulkan (via MoltenVK)**:
   - Import Metal IOSurface into Vulkan
   - Requires VK_EXT_metal_surface extension
   
2. **OpenGL**:
   - Use IOSurface with OpenGL on macOS
   - Via CGLTexImageIOSurface2D

3. **Core Graphics / Core Image**:
   - Direct IOSurface support in Apple frameworks

## Known Limitations

1. **Platform-specific**: Only works on Apple platforms
2. **Storage Mode**: IOSurface textures must use Shared storage mode
3. **Pixel Format**: Some formats may have limited IOSurface support
4. **Performance**: Shared storage mode may be slower than Private for some workloads

## Testing

Unit tests are included but require macOS to run:

```bash
# On macOS
cargo test --features metal --lib
cargo test --features metal --test integration_tests
```

Integration tests require actual Metal hardware and driver support.

## Future Enhancements

- [ ] Support for texture arrays and 3D textures
- [ ] Additional pixel format conversions
- [ ] Async event waiting with MTLSharedEventListener
- [ ] Memory usage optimization
- [ ] Support for compressed texture formats
