# Geyser: High-Performance GPU Texture Sharing

## 🌊 Overview

Geyser is a high-performance Rust library designed for zero-copy GPU texture sharing across various graphics APIs, including Vulkan, Metal, and eventually WebGPU. It aims to provide a unified, safe, and efficient interface for applications and processes to seamlessly share GPU memory resources.

This capability is critical for:
*   Building efficient visual pipelines.
*   Enabling real-time collaboration between different graphics tools.
*   Creating modular visual systems where components can share GPU resources.

## ✨ Features

### Core Capabilities
*   ✅ **Zero-Copy Sharing:** Direct GPU memory sharing without CPU transfers
*   ✅ **Unified Rust API:** Clean, safe interface across all graphics APIs
*   ✅ **Production-Ready:** Full implementation for Vulkan and Metal backends
*   ✅ **Cross-Process Support:** Share textures between separate processes
*   ✅ **Synchronization Primitives:** Semaphores, fences, and events for GPU coordination

### Platform Support
*   ✅ **Windows:** Vulkan with HANDLE-based external memory
*   ✅ **Linux:** Vulkan with FD-based external memory
*   ✅ **macOS/iOS:** Metal with IOSurface-backed textures

### Texture Features
*   ✅ **21 Texture Formats:** RGBA8, RGBA16, RGBA32, Depth, HDR, and more
*   ✅ **All Usage Types:** Render targets, samplers, storage, copy operations
*   ✅ **Resource Management:** Automatic lifetime tracking and cleanup
*   ✅ **Thread-Safe:** Safe concurrent access from multiple threads

## 🚀 Getting Started

### Prerequisites

*   Rust `stable` or `beta`
*   Vulkan SDK (for Vulkan support)
*   macOS (for Metal support)

### Installation

Add `geyser` to your `Cargo.toml`:

```toml
[dependencies]
geyser = { version = "0.1", features = ["vulkan", "metal"] } # Choose features based on needs
```

### Usage Example

```rust
use geyser::{
    vulkan::VulkanTextureShareManager,
    common::{TextureDescriptor, TextureFormat, TextureUsage},
    TextureShareManager,
};

// Create manager
let manager = VulkanTextureShareManager::new(
    instance, device, physical_device, queue_family
)?;

// Create shareable texture
let desc = TextureDescriptor {
    width: 1920,
    height: 1080,
    format: TextureFormat::Rgba8Unorm,
    usage: vec![TextureUsage::RenderAttachment, TextureUsage::TextureBinding],
    label: Some("MyTexture".to_string()),
};
let texture = manager.create_shareable_texture(&desc)?;

// Export for sharing
let handle = manager.export_texture(texture.as_ref())?;

// In another process/context:
let imported = manager.import_texture(handle, &desc)?;

// Clean up
manager.release_texture_handle(handle)?;
```

### Examples

See the [`examples/`](examples/) directory for comprehensive demonstrations:

**Phase 1 - Basic Sharing:**
- **`vulkan_to_vulkan.rs`**: Vulkan texture sharing between contexts
- **`metal_to_metal.rs`**: Metal IOSurface-based sharing (macOS)
- **`bevy_integration.rs`**: Integration with Bevy game engine

**Phase 2 - Cross-Process IPC:**
- **`ipc_producer.rs`** & **`ipc_consumer.rs`**: Multi-process texture sharing with binary semaphores
- **`timeline_semaphore_pipeline.rs`**: Frame pipelining with timeline semaphores
- **`timeline_ipc_producer.rs`** & **`timeline_ipc_consumer.rs`**: Multi-process with timeline semaphores

Run examples with:
```bash
# Single process
cargo run --example vulkan_to_vulkan --features vulkan
cargo run --example timeline_semaphore_pipeline --features vulkan

# Multi-process (run in separate terminals)
cargo run --example ipc_producer --features vulkan
cargo run --example ipc_consumer --features vulkan
```

See [`examples/README.md`](examples/README.md) for detailed usage instructions.

## 🛠️ Architecture

Geyser's architecture is built around a common set of traits and enums (`SharedTexture`, `TextureShareManager`, `TextureFormat`, `ApiTextureHandle`) that abstract the specifics of each graphics API. Individual modules (`src/vulkan`, `src/metal`, `src/webgpu`) provide the concrete implementations for each backend, utilizing platform-specific interop mechanisms (e.g., Vulkan external memory, Metal IOSurfaces).

## 💡 Motivation

The modern graphics landscape often involves multiple applications or components needing to interact with the GPU. Existing solutions for sharing GPU data are often API-specific, involve costly CPU round-trips, or lack the safety guarantees Rust provides. Geyser aims to fill this gap, offering a robust and performant solution for the Rust graphics ecosystem.

## 🗺️ Roadmap

### ✅ Phase 1: Foundational Backends (COMPLETE)
*   ✅ Vulkan → Vulkan sharing on Linux/Windows
*   ✅ Metal → Metal sharing on macOS/iOS
*   ✅ Complete API for texture creation, import, and export
*   ✅ Windows external memory (HANDLE-based)
*   ✅ Linux external memory (FD-based)
*   ✅ Synchronization primitives (semaphores, fences, events)
*   ✅ Comprehensive test coverage
*   ✅ Documentation and examples

### ✅ Phase 2: Cross-API Sharing (80% Complete)
*   ✅ **Real cross-process IPC examples** - Producer/consumer with binary & timeline semaphores
*   ✅ **Timeline semaphores** - Counter-based synchronization for advanced pipelines
*   ✅ **Performance benchmarks** - Comprehensive criterion-based benchmark suite
*   🔄 **Bevy engine integration** - Plugin foundation complete (wgpu-hal bridge remains for Phase 3)
*   ⚪ **Vulkan ↔ Metal sharing** - Requires macOS development environment

### 🔵 Phase 3: WebGPU Integration & Bevy Completion (15% Complete)
*   🔄 **wgpu-hal bridge** - Foundation complete (format/usage conversion, handle wrappers), external memory import pending
*   ⚪ **Bevy multi-window example** - Shared textures across windows
*   ⚪ **Bevy multi-process game** - Physics/render process separation
*   ⚪ **WebGPU backend** - Import/export for wgpu
*   ⚪ **Cross-API sharing** - Vulkan ↔ WebGPU, Metal ↔ WebGPU
*   ⚪ **Web platform support** - Browser-based texture sharing

### ⚪ Phase 4: Advanced Features (Future)
*   ⚪ **Compressed texture formats** - BC, ASTC, ETC2 support
*   ⚪ **Texture arrays** - 2D array and cube map sharing
*   ⚪ **3D textures** - Volume texture support
*   ⚪ **Multi-GPU scenarios** - Explicit device selection and transfer
*   ⚪ **Additional integrations** - wgpu, three-d, rend3, etc.

## 👋 Contributing

We welcome contributions! Please see our `CONTRIBUTING.md` for guidelines.

## 📄 License

Geyser is licensed under the [MIT License](LICENSE-MIT) or the [Apache License, Version 2.0](LICENSE-APACHE), at your option.
