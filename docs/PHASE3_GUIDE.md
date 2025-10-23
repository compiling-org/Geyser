# Phase 3 Implementation Guide

## Overview

Phase 3 focuses on completing the Bevy integration with true zero-copy texture import and adding WebGPU backend support. This document provides implementation guidance based on Phase 2 learnings.

## Status: Ready to Begin

**Prerequisites Completed:**
- ✅ Phase 1: Foundational backends (Vulkan, Metal)
- ✅ Phase 2: Cross-process IPC, Timeline semaphores, Benchmarks
- ✅ Bevy plugin foundation (events, ECS, resources)

**Phase 3 Goals:**
1. wgpu-hal bridge for zero-copy Bevy integration
2. Bevy multi-window shared texture example
3. Bevy multi-process game example
4. WebGPU/wgpu backend implementation
5. Cross-API sharing with WebGPU
6. Web platform support considerations

---

## 1. wgpu-hal Bridge Implementation

### Challenge

Bevy 0.14 uses wgpu 0.20, which has a `wgpu-hal` layer for low-level GPU access. The goal is to import Geyser's Vulkan textures directly into wgpu without CPU copies.

### Approach

**Option A: Direct hal Import (Ideal)**
```rust
use wgpu::hal;

// In Bevy's render world
let hal_device: &hal::vulkan::Device = ...; // Extract from wgpu device
let hal_texture = unsafe {
    hal_device.texture_from_raw(
        vulkan_image,           // vk::Image
        &hal::TextureDescriptor {
            // Convert Geyser descriptor to hal
        },
        None,
    )
};

// Wrap in wgpu::Texture
let wgpu_texture = unsafe {
    device.create_texture_from_hal(
        hal_texture,
        &wgpu::TextureDescriptor { ... }
    )
};
```

**Option B: Render Resource Bridge (Pragmatic)**
```rust
// Use Bevy's render resource system
// Import texture during ExtractCommands phase
// Requires careful lifetime management
```

### Key Files to Modify

1. `src/bevy_plugin/mod.rs`
   - Add wgpu-hal conversion utilities
   - Implement `import_vulkan_texture_to_wgpu()`
   - Handle format mapping (Geyser → wgpu)

2. `src/bevy_plugin/render.rs` (new)
   - Render world integration
   - Texture import during extract phase
   - Synchronization with Bevy's render graph

### Format Mapping

```rust
fn geyser_to_wgpu_format(format: geyser::TextureFormat) -> wgpu::TextureFormat {
    match format {
        TextureFormat::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
        TextureFormat::Bgra8Unorm => wgpu::TextureFormat::Bgra8Unorm,
        // ... complete mapping
    }
}
```

### Usage Mapping

```rust
fn geyser_to_wgpu_usage(usage: &[geyser::TextureUsage]) -> wgpu::TextureUsages {
    let mut wgpu_usage = wgpu::TextureUsages::empty();
    for u in usage {
        match u {
            TextureUsage::RenderAttachment => wgpu_usage |= wgpu::TextureUsages::RENDER_ATTACHMENT,
            TextureUsage::TextureBinding => wgpu_usage |= wgpu::TextureUsages::TEXTURE_BINDING,
            // ... complete mapping
        }
    }
    wgpu_usage
}
```

---

## 2. Bevy Multi-Window Example

### Concept

Two Bevy windows sharing the same texture - Window A renders a rotating cube to the texture, Window B displays it as a sprite.

### Implementation Outline

```rust
// examples/bevy_multi_window.rs

use bevy::prelude::*;
use bevy::window::WindowMode;
use geyser::bevy_plugin::{GeyserPlugin, ImportGeyserTexture};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(GeyserPlugin)
        .add_systems(Startup, setup_windows)
        .add_systems(Update, (
            render_to_shared_texture,
            display_shared_texture,
        ))
        .run();
}

fn setup_windows(
    mut commands: Commands,
    mut import_events: EventWriter<ImportGeyserTexture>,
) {
    // Create Geyser texture outside Bevy
    // Import into Bevy
    // Spawn two cameras for two windows
}
```

### Key Challenges

1. **Window Management**
   - Bevy 0.14 multi-window support
   - Per-window camera setup
   - Render target assignment

2. **Synchronization**
   - Ensure Window A finishes rendering before Window B reads
   - Use Bevy's render graph ordering

3. **Lifecycle**
   - Handle window close gracefully
   - Cleanup shared resources

---

## 3. Bevy Multi-Process Game Example

### Concept

**Process A (Physics/Logic):**
- Runs game simulation
- Renders debug visualization to shared texture
- Exports texture handle via IPC

**Process B (Display):**
- Imports shared texture
- Displays in Bevy window
- Minimal game logic

### Architecture

```
┌─────────────────┐         IPC Channel        ┌─────────────────┐
│  Process A      │◄──────────────────────────►│  Process B      │
│  (Game Logic)   │   Texture Handle + Sync    │  (Display)      │
├─────────────────┤                            ├─────────────────┤
│ Bevy ECS        │                            │ Bevy ECS        │
│ Physics Sim     │                            │ Minimal Logic   │
│ Render to       │                            │ Display Texture │
│ Geyser Texture  │                            │ from Process A  │
└─────────────────┘                            └─────────────────┘
        │                                              │
        └──────────────────┬───────────────────────────┘
                           │
                    ┌──────▼──────┐
                    │  Shared GPU │
                    │   Texture   │
                    └─────────────┘
```

### Implementation Files

```
examples/
  bevy_game_producer.rs  - Physics/logic process
  bevy_game_consumer.rs  - Display process
  bevy_game_shared.rs    - Shared types and IPC
```

### Synchronization Strategy

Use timeline semaphores from Phase 2:
```rust
// Producer
timeline_sem.signal(frame_number);
ipc.send(FrameReady { frame_number });

// Consumer
ipc.receive(); // Wait for notification
timeline_sem.wait(frame_number); // Wait for GPU completion
render_in_bevy();
```

---

## 4. WebGPU Backend Implementation

### File Structure

```
src/wgpu/
  mod.rs           - Main module
  manager.rs       - WgpuTextureShareManager
  texture.rs       - WgpuSharedTexture
  conversions.rs   - Format/usage conversions
  tests.rs         - Unit tests
```

### WgpuTextureShareManager

```rust
pub struct WgpuTextureShareManager {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    // Track exported/imported textures
    textures: Mutex<HashMap<u64, wgpu::Texture>>,
}

impl TextureShareManager for WgpuTextureShareManager {
    fn create_shareable_texture(&self, desc: &TextureDescriptor) 
        -> Result<Box<dyn SharedTexture>> 
    {
        // Create wgpu texture with COPY_SRC | COPY_DST for sharing
        // Wrap in WgpuSharedTexture
    }
    
    fn export_texture(&self, texture: &dyn SharedTexture) 
        -> Result<ApiTextureHandle> 
    {
        // On native: Extract underlying Vulkan/Metal/DX12 handle
        // On web: Use SharedArrayBuffer or canvas transfer
    }
    
    fn import_texture(&self, handle: ApiTextureHandle, desc: &TextureDescriptor) 
        -> Result<Box<dyn SharedTexture>> 
    {
        // On native: Import from underlying API
        // On web: Receive from SharedArrayBuffer
    }
}
```

### Platform Considerations

**Native (wgpu-native):**
- Access hal layer: `device.as_hal<Vulkan, _>(|hal_device| ...)`
- Import external handles
- Similar to Bevy bridge

**Web (wgpu-web):**
- No direct handle sharing
- Options:
  - OffscreenCanvas transfer
  - SharedArrayBuffer (requires COOP/COEP headers)
  - ImageBitmap
  - Canvas→Canvas copy (not zero-copy)

---

## 5. Cross-API Sharing with WebGPU

### Vulkan ↔ wgpu

On systems where wgpu uses Vulkan backend:
```rust
// Export from Geyser Vulkan
let vulkan_handle = vulkan_manager.export_texture(texture)?;

// Import into wgpu via hal
let wgpu_texture = wgpu_manager.import_texture(
    ApiTextureHandle::Vulkan(vulkan_handle),
    &desc
)?;
```

### Metal ↔ wgpu (macOS)

```rust
// Both can use IOSurface
let metal_handle = metal_manager.export_texture(texture)?;
let wgpu_texture = wgpu_manager.import_texture(
    ApiTextureHandle::Metal(metal_handle),
    &desc
)?;
```

---

## 6. Web Platform Support

### Constraints

1. **No Direct GPU Memory Sharing**
   - WebGPU doesn't expose native handles
   - No cross-context texture sharing

2. **Workarounds:**
   - **OffscreenCanvas.transferToImageBitmap()** - Transfers ownership
   - **SharedArrayBuffer** - CPU-side copy with Worker threads
   - **Canvas Capture** - `captureStream()` for video-like sharing

### Web Implementation Strategy

```rust
#[cfg(target_arch = "wasm32")]
mod web {
    // Use web-sys for DOM integration
    pub struct WebTextureShareHandle {
        canvas_id: String,
        // Or ImageBitmap handle
    }
    
    // Implement via canvas transfer
}
```

### Example: Offscreen Canvas Sharing

```javascript
// In web context
const offscreen = new OffscreenCanvas(width, height);
const ctx = offscreen.getContext('webgpu');

// Render to offscreen
// Transfer to main thread
const bitmap = offscreen.transferToImageBitmap();
postMessage({ type: 'frame', bitmap }, [bitmap]);
```

---

## Implementation Priority

### Phase 3.1: Bevy Zero-Copy (2-3 weeks)
1. ✅ Research complete
2. ⚪ Implement wgpu-hal bridge utilities
3. ⚪ Update Bevy plugin with real import
4. ⚪ Test with simple example

### Phase 3.2: Bevy Examples (1-2 weeks)
5. ⚪ Multi-window shared texture
6. ⚪ Multi-process game (physics/display)
7. ⚪ Documentation and troubleshooting

### Phase 3.3: WebGPU Backend (3-4 weeks)
8. ⚪ Native wgpu backend (Vulkan/Metal/DX12)
9. ⚪ Cross-API with wgpu
10. ⚪ Web platform prototype

### Phase 3.4: Polish (1 week)
11. ⚪ Comprehensive docs
12. ⚪ Performance validation
13. ⚪ Update examples README

---

## Technical Challenges & Solutions

### Challenge 1: wgpu Version Compatibility

**Problem:** wgpu breaking changes between versions  
**Solution:** 
- Pin to wgpu 0.20 (matches Bevy 0.14)
- Document upgrade path for Bevy 0.15+
- Abstract behind trait for flexibility

### Challenge 2: Bevy Render World Access

**Problem:** Limited access to wgpu internals  
**Solution:**
- Use `RenderDevice` resource
- Extract during `ExtractSchedule`
- Work with Bevy's `PrepareAssets` system

### Challenge 3: Web Platform Limitations

**Problem:** No true zero-copy on web  
**Solution:**
- Document limitations clearly
- Provide "best effort" implementation
- Consider OffscreenCanvas for Workers
- Future: Wait for WebGPU spec evolution

### Challenge 4: Synchronization

**Problem:** Bevy's render graph vs manual sync  
**Solution:**
- Let Bevy handle synchronization where possible
- Use external semaphores only for cross-process
- Timeline semaphores for precise control

---

## Resources

### Documentation
- [wgpu Hal Documentation](https://docs.rs/wgpu-hal)
- [Bevy Render Architecture](https://bevyengine.org/learn/book/gpu-rendering/)
- [WebGPU Specification](https://www.w3.org/TR/webgpu/)

### Reference Implementations
- Bevy's texture asset loading
- wgpu examples: hal-texture-import
- Firefox WebGPU implementation

### Community
- Bevy Discord #rendering-dev
- wgpu Matrix channel
- Rust Graphics Working Group

---

## Next Steps

1. **Start with wgpu-hal utilities**
   - Create `src/bevy_plugin/wgpu_bridge.rs`
   - Implement format/usage conversions
   - Add texture import function

2. **Test with minimal example**
   - Single window
   - Import Geyser texture
   - Display as sprite

3. **Expand to multi-window**
   - Two windows
   - Shared texture
   - Synchronized rendering

4. **Multi-process game**
   - Separate executables
   - IPC with timeline semaphores
   - Full game loop

5. **WebGPU backend**
   - Native first (via hal)
   - Web prototype
   - Document limitations

---

## Success Criteria

Phase 3 is complete when:
- ✅ Bevy can import Geyser textures with zero CPU copies
- ✅ Multi-window example works smoothly
- ✅ Multi-process game demonstrates real use case
- ✅ WebGPU backend exists (native at minimum)
- ✅ Documentation is comprehensive
- ✅ Performance matches or exceeds Phase 2 baselines

---

**Phase 3 Status:** Ready to Begin  
**Estimated Completion:** 8-12 weeks (part-time)  
**Primary Blocker:** None (all prerequisites complete)

**Let's build it!** 🚀
