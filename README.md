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

- **`vulkan_to_vulkan.rs`**: Vulkan texture sharing between contexts
- **`metal_to_metal.rs`**: Metal IOSurface-based sharing (macOS)
- **`bevy_integration.rs`**: Integration with Bevy game engine
- **`vulkan_to_metal.rs`**: Cross-API sharing (Phase 2)

Run examples with:
```bash
cargo run --example vulkan_to_vulkan --features vulkan
cargo run --example metal_to_metal --features metal
cargo run --example bevy_integration --features vulkan  # or metal on macOS
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

### 🚧 Phase 2: Cross-API Sharing (Planned)
*   ⚪ Vulkan ↔ Metal sharing on macOS via MoltenVK
*   ⚪ True zero-copy Bevy engine integration
*   ⚪ Real cross-process IPC examples
*   ⚪ Timeline semaphores for advanced synchronization
*   ⚪ Performance benchmarks and optimization

### 🔵 Phase 3: WebGPU Integration (Future)
*   ⚪ WebGPU backend for import/export
*   ⚪ Cross-API sharing involving WebGPU
*   ⚪ Web platform support

### ⚪ Phase 4: Advanced Features (Future)
*   ⚪ Compressed texture format support
*   ⚪ Texture arrays and 3D textures
*   ⚪ Multi-GPU scenarios
*   ⚪ Additional graphics library integrations

## 👋 Contributing

We welcome contributions! Please see our `CONTRIBUTING.md` for guidelines.

## 📄 License

Geyser is licensed under the [MIT License](LICENSE-MIT) or the [Apache License, Version 2.0](LICENSE-APACHE), at your option.
