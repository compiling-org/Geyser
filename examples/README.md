# Geyser Examples

This directory contains examples demonstrating Geyser's GPU texture sharing capabilities.

## Overview

The examples are organized by functionality and complexity:

### Phase 1 Examples (Foundational)
- **`vulkan_to_vulkan.rs`**: Demonstrates Vulkan texture sharing between two contexts
- **`metal_to_metal.rs`**: Demonstrates Metal texture sharing using IOSurface (macOS only)
- **`bevy_integration.rs`**: Shows conceptual integration with Bevy game engine

### Phase 2 Examples (Cross-API, Coming Soon)
- **`vulkan_to_metal.rs`**: Cross-API sharing between Vulkan and Metal

## Prerequisites

### All Platforms
- Rust stable or beta toolchain
- GPU with appropriate driver support

### Windows
- Vulkan SDK installed
- Compatible GPU with Vulkan support

### Linux
- Vulkan development packages:
  ```bash
  # Ubuntu/Debian
  sudo apt install vulkan-tools libvulkan-dev
  
  # Fedora
  sudo dnf install vulkan-tools vulkan-loader-devel
  ```

### macOS
- Xcode Command Line Tools
- Metal is included with macOS

## Running Examples

### Vulkan to Vulkan (Windows/Linux)

```bash
# On Windows or Linux
cargo run --example vulkan_to_vulkan --features vulkan
```

This example:
1. Creates two Vulkan contexts
2. Creates a shareable texture in Context 1
3. Exports the texture handle
4. Imports the handle in Context 2
5. Demonstrates proper cleanup

**Expected Output:**
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

### Metal to Metal (macOS only)

```bash
# On macOS
cargo run --example metal_to_metal --features metal
```

This example:
1. Creates two Metal contexts
2. Creates a shareable texture with IOSurface backing
3. Exports the IOSurface ID
4. Imports the texture in another context via IOSurface lookup
5. Demonstrates proper resource management

**Expected Output:**
```
=== Geyser Metal to Metal Texture Sharing Example ===

Creating Metal Context 1...
✓ Context 1 created

App 1: Creating shareable texture...
✓ Texture created
...
```

### Bevy Integration

```bash
# Windows/Linux with Vulkan
cargo run --example bevy_integration --features vulkan

# macOS with Metal
cargo run --example bevy_integration --features metal
```

This example:
1. Initializes a Geyser texture manager
2. Creates a shareable texture
3. Sets up a Bevy window and renderer
4. Displays an animated pattern updated from the Geyser texture
5. Demonstrates the integration pattern (currently using CPU copies)

**Expected Output:**
- A window displaying an animated RGB gradient pattern
- Console output showing initialization steps

**Note:** This Phase 1 example uses CPU-side copies to transfer data. True zero-copy integration requires deeper WGPU/Bevy integration (Phase 2/3 goal).

## Platform-Specific Notes

### Windows
- Examples use `VK_KHR_external_memory_win32` for Vulkan external memory
- Requires Vulkan SDK to be properly installed
- May need to enable Vulkan validation layers for debugging

### Linux
- Examples use `VK_KHR_external_memory_fd` for Vulkan external memory
- File descriptor-based sharing
- Ensure your GPU drivers support external memory extensions

### macOS
- Metal examples use IOSurface for cross-process sharing
- IOSurface IDs can be passed between processes
- Vulkan support via MoltenVK (not yet implemented for cross-API sharing)

## Implementation Status

### ✅ Completed Features

#### Vulkan Backend
- ✅ Windows external memory (VK_KHR_external_memory_win32)
- ✅ Linux external memory (VK_KHR_external_memory_fd)
- ✅ Texture export/import with proper extension loading
- ✅ Semaphore synchronization (Windows HANDLE / Linux FD)
- ✅ Fence synchronization (Windows HANDLE / Linux FD)
- ✅ Resource lifetime management

#### Metal Backend
- ✅ IOSurface-backed texture sharing (macOS/iOS)
- ✅ Texture export/import via IOSurface IDs
- ✅ MTLSharedEvent synchronization
- ✅ All 21 common texture formats
- ✅ Proper storage mode configuration

#### Testing
- ✅ 9 unit tests for common types
- ✅ Unit tests for Vulkan synchronization primitives
- ✅ Unit tests for Metal handles and formats
- ✅ Integration test framework

### 🚧 Current Limitations

1. **Bevy Integration**: Currently uses CPU-side copies, not true zero-copy
2. **Cross-Process Examples**: Examples simulate cross-process by using multiple contexts
3. **WebGPU Backend**: Not yet implemented

### 🎯 Future Enhancements

#### Phase 2 (Planned)
- Real cross-process IPC examples
- Cross-API sharing (Vulkan ↔ Metal on macOS)
- Timeline semaphores for advanced sync
- Performance benchmarks

#### Phase 3 (Future)
- WebGPU backend integration
- Zero-copy Bevy integration
- Multi-GPU scenarios
- Compressed texture format support

## Troubleshooting

### "Failed to load Vulkan"
- **Windows**: Ensure Vulkan SDK is installed and in PATH
- **Linux**: Install vulkan-loader package
- Check that your GPU supports Vulkan

### "No Metal device found" (macOS)
- Ensure you're running on macOS 10.13 or later
- Check that Metal is supported on your Mac

### Bevy window doesn't open
- Check that you have proper windowing support (X11/Wayland on Linux)
- On Windows, ensure you have up-to-date graphics drivers

### Compilation errors with Bevy
- Bevy has many features; ensure you're not running into dependency conflicts
- Try `cargo clean` and rebuild

## Example Code Structure

Each example follows a similar pattern:

```rust
// 1. Create graphics API context (Vulkan/Metal)
let manager = create_manager()?;

// 2. Create a shareable texture
let texture_desc = TextureDescriptor { /* ... */ };
let texture = manager.create_shareable_texture(&texture_desc)?;

// 3. Export the texture handle
let handle = manager.export_texture(texture.as_ref())?;

// 4. In another context/process, import the texture
let imported = manager.import_texture(handle, &texture_desc)?;

// 5. Use the imported texture
// ...

// 6. Clean up
manager.release_texture_handle(handle)?;
```

## Contributing

When adding new examples:
1. Follow the existing naming convention
2. Include comprehensive comments
3. Add platform-specific `#[cfg]` guards
4. Update this README with usage instructions
5. Test on all supported platforms if possible

## Next Steps

To extend these examples:
1. Implement real external memory export/import in Vulkan backend
2. Create multi-process examples using IPC for handle passing
3. Add synchronization primitives
4. Implement cross-API sharing examples
5. Develop zero-copy Bevy integration

## Resources

- [Vulkan External Memory](https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/VK_KHR_external_memory.html)
- [Metal IOSurface](https://developer.apple.com/documentation/iosurface)
- [Bevy Rendering](https://bevyengine.org/learn/book/getting-started/plugins/)
- [WGPU Documentation](https://docs.rs/wgpu/latest/wgpu/)
