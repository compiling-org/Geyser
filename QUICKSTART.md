# Geyser Quick Start Guide

## 🚀 Get Started in 5 Minutes

This guide gets you running Geyser examples quickly.

---

## Installation

### 1. Prerequisites

**All Platforms:**
```bash
# Ensure Rust is installed
rustc --version
```

**Windows:**
- Install [Vulkan SDK](https://vulkan.lunarg.com/)

**Linux:**
```bash
# Ubuntu/Debian
sudo apt install vulkan-tools libvulkan-dev

# Fedora
sudo dnf install vulkan-tools vulkan-loader-devel
```

**macOS:**
- Xcode Command Line Tools: `xcode-select --install`

### 2. Clone and Build

```bash
git clone https://github.com/yourusername/geyser.git
cd geyser
cargo build --features vulkan,metal
```

---

## Run Your First Example

### Windows/Linux: Vulkan

```bash
cargo run --example vulkan_to_vulkan --features vulkan
```

**What you'll see:**
```
=== Geyser Vulkan to Vulkan Texture Sharing Example ===

Creating Vulkan Context 1...
✓ Context 1 created

App 1: Creating shareable texture...
✓ Texture created
  - Width: 1024
  - Height: 768
  - Format: Rgba8Unorm
...
```

### macOS: Metal

```bash
cargo run --example metal_to_metal --features metal
```

### All Platforms: Bevy Integration

```bash
# Windows/Linux
cargo run --example bevy_integration --features vulkan

# macOS
cargo run --example bevy_integration --features metal
```

**What you'll see:**
- A window with animated RGB gradient pattern
- Console output showing initialization steps

---

## Basic Usage

### Create a Shareable Texture

```rust
use geyser::{
    TextureShareManager,
    TextureDescriptor, TextureFormat, TextureUsage,
};

// Create manager (platform-specific)
#[cfg(target_os = "windows")]
use geyser::vulkan::VulkanTextureShareManager;

let manager = VulkanTextureShareManager::new(
    instance, device, physical_device, queue_family
)?;

// Define texture
let desc = TextureDescriptor {
    width: 1920,
    height: 1080,
    format: TextureFormat::Rgba8Unorm,
    usage: vec![
        TextureUsage::RenderAttachment,
        TextureUsage::TextureBinding,
    ],
    label: Some("MyTexture".to_string()),
};

// Create it
let texture = manager.create_shareable_texture(&desc)?;
```

### Export a Texture

```rust
// Export for sharing
let handle = manager.export_texture(texture.as_ref())?;

// Handle can now be:
// - Sent to another process via IPC
// - Passed to another API context
// - Serialized for storage
```

### Import a Texture

```rust
// In another context/process
let imported = manager.import_texture(handle, &desc)?;

// Use the imported texture
println!("Imported: {}x{}", imported.width(), imported.height());
```

### Clean Up

```rust
// Release resources
manager.release_texture_handle(handle)?;
drop(texture);
```

---

## Project Structure

```
geyser/
├── src/
│   ├── lib.rs              # Core traits
│   ├── common/mod.rs       # Common types
│   ├── error.rs            # Error handling
│   ├── vulkan/mod.rs       # Vulkan backend
│   ├── metal/mod.rs        # Metal backend
│   └── webgpu/mod.rs       # WebGPU (stub)
├── examples/
│   ├── vulkan_to_vulkan.rs # Vulkan example
│   ├── metal_to_metal.rs   # Metal example
│   ├── bevy_integration.rs # Bevy example
│   └── README.md           # Examples guide
└── docs/
    ├── architecture.md     # Architecture
    ├── PHASE1_SUMMARY.md   # Implementation details
    └── EXAMPLES_SUMMARY.md # Examples breakdown
```

---

## Common Tasks

### Add a New Texture Format

```rust
// In src/common/mod.rs
pub enum TextureFormat {
    Rgba8Unorm,
    Bgra8Unorm,
    R16Float,
    R32Float,
    MyNewFormat, // Add here
}

// Update backend mappings
// In src/vulkan/mod.rs
fn map_texture_format_to_vk(&self, format: TextureFormat) -> Result<vk::Format> {
    match format {
        // ...
        TextureFormat::MyNewFormat => Ok(vk::Format::MY_FORMAT),
    }
}
```

### Check Examples

```bash
# List all examples
ls examples/*.rs

# Run specific example
cargo run --example <name> --features <backend>
```

### Read Documentation

```bash
# View architecture
cat docs/architecture.md

# View Phase 1 summary
cat docs/PHASE1_SUMMARY.md

# View examples guide
cat examples/README.md
```

---

## Troubleshooting

### "Failed to load Vulkan"
```bash
# Check Vulkan installation
vulkaninfo

# Windows: Ensure Vulkan SDK is in PATH
# Linux: Install vulkan-loader
```

### "No Metal device found"
```bash
# macOS only
# Check: System Preferences > Displays
# Ensure macOS 10.13+
```

### Compilation errors
```bash
# Clean and rebuild
cargo clean
cargo build --features vulkan,metal

# Check feature flags
cargo build --features vulkan  # or metal
```

---

## Next Steps

### Learn More
1. **Architecture**: Read [`docs/architecture.md`](docs/architecture.md)
2. **Examples**: Study [`examples/README.md`](examples/README.md)
3. **Phase 1**: Review [`docs/PHASE1_SUMMARY.md`](docs/PHASE1_SUMMARY.md)

### Contribute
1. **Pick an issue**: Check GitHub Issues
2. **Read guidelines**: See [`CONTRIBUTING.md`](CONTRIBUTING.md)
3. **Submit PR**: Follow contribution workflow

### Advanced Topics
1. **External Memory**: Implement real Vulkan export/import
2. **Synchronization**: Add cross-process sync primitives
3. **Cross-API**: Bridge Vulkan ↔ Metal
4. **Performance**: Add benchmarks

---

## Quick Reference

### Feature Flags
- `vulkan`: Enable Vulkan backend
- `metal`: Enable Metal backend (macOS only)
- `webgpu`: Enable WebGPU backend (future)

### Examples
- `vulkan_to_vulkan`: Vulkan sharing
- `metal_to_metal`: Metal sharing (macOS)
- `bevy_integration`: Bevy integration
- `vulkan_to_metal`: Cross-API (Phase 2)

### Platforms
- **Windows**: Vulkan via `ash`
- **Linux**: Vulkan via `ash`
- **macOS**: Metal via `metal` crate

---

## Resources

- **Main README**: [`README.md`](README.md)
- **Examples Guide**: [`examples/README.md`](examples/README.md)
- **Architecture**: [`docs/architecture.md`](docs/architecture.md)
- **Phase 1 Summary**: [`docs/PHASE1_SUMMARY.md`](docs/PHASE1_SUMMARY.md)
- **Project Status**: [`PROJECT_STATUS.md`](PROJECT_STATUS.md)

### External Links
- [Vulkan Specification](https://www.khronos.org/vulkan/)
- [Metal Documentation](https://developer.apple.com/metal/)
- [Bevy Engine](https://bevyengine.org/)
- [Rust Book](https://doc.rust-lang.org/book/)

---

**Happy sharing!** 🎉

For issues, questions, or contributions, visit the GitHub repository.
