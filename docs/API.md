# Geyser API Documentation

## Table of Contents

1. [Core Traits](#core-traits)
2. [Common Types](#common-types)
3. [Vulkan Backend](#vulkan-backend)
4. [Metal Backend](#metal-backend)
5. [Error Handling](#error-handling)
6. [Best Practices](#best-practices)

---

## Core Traits

### `TextureShareManager`

The main trait for managing shareable textures across graphics APIs.

```rust
pub trait TextureShareManager {
    fn create_shareable_texture(&self, descriptor: &TextureDescriptor) 
        -> Result<Box<dyn SharedTexture>>;
    
    fn export_texture(&self, texture: &dyn SharedTexture) 
        -> Result<ApiTextureHandle>;
    
    fn import_texture(&self, handle: ApiTextureHandle, descriptor: &TextureDescriptor) 
        -> Result<Box<dyn SharedTexture>>;
    
    fn release_texture_handle(&self, handle: ApiTextureHandle) 
        -> Result<()>;
}
```

**Methods:**

- `create_shareable_texture`: Creates a new texture that can be shared across processes
- `export_texture`: Exports a texture to a handle for cross-process sharing
- `import_texture`: Imports a texture from a handle received from another process
- `release_texture_handle`: Releases resources associated with an exported handle

### `SharedTexture`

Trait representing a shared texture.

```rust
pub trait SharedTexture: Any {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn format(&self) -> TextureFormat;
    fn usage(&self) -> &[TextureUsage];
    fn as_any(&self) -> &dyn Any;
}
```

---

## Common Types

### `TextureDescriptor`

Describes the properties of a texture to be created.

```rust
pub struct TextureDescriptor {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub usage: Vec<TextureUsage>,
    pub label: Option<String>,
}
```

**Example:**
```rust
let descriptor = TextureDescriptor {
    width: 1920,
    height: 1080,
    format: TextureFormat::Rgba8Unorm,
    usage: vec![
        TextureUsage::RenderAttachment,
        TextureUsage::TextureBinding,
    ],
    label: Some("MySharedTexture".to_string()),
};
```

### `TextureFormat`

Supported texture pixel formats (21 formats total):

**8-bit Formats:**
- `Rgba8Unorm` - 8-bit RGBA unsigned normalized
- `Bgra8Unorm` - 8-bit BGRA unsigned normalized
- `Rgba8Srgb` - 8-bit RGBA sRGB
- `Bgra8Srgb` - 8-bit BGRA sRGB
- `R8Unorm` - 8-bit single channel
- `Rg8Unorm` - 8-bit dual channel

**16-bit Formats:**
- `R16Float`, `Rg16Float`, `Rgba16Float` - Half-precision float
- `R16Uint`, `R16Sint` - 16-bit integer

**32-bit Formats:**
- `R32Float`, `Rg32Float`, `Rgba32Float` - Single-precision float
- `R32Uint`, `R32Sint` - 32-bit integer

**Depth/Stencil:**
- `Depth32Float` - 32-bit depth
- `Depth24Plus` - 24-bit depth
- `Depth24PlusStencil8` - 24-bit depth + 8-bit stencil

**HDR Formats:**
- `Rgb10a2Unorm` - 10-bit RGB + 2-bit alpha
- `Rg11b10Float` - 11-bit RG + 10-bit B float

### `TextureUsage`

Specifies how a texture will be used:

```rust
pub enum TextureUsage {
    CopySrc,           // Can be copied from
    CopyDst,           // Can be copied to
    TextureBinding,    // Can be sampled by shaders
    RenderAttachment,  // Can be rendered to
    StorageBinding,    // Can be written by compute shaders
}
```

### `ApiTextureHandle`

Platform-specific handle for sharing textures:

```rust
pub enum ApiTextureHandle {
    #[cfg(feature = "vulkan")]
    Vulkan(VulkanTextureShareHandle),
    
    #[cfg(feature = "metal")]
    Metal(MetalTextureShareHandle),
}
```

### `SyncPrimitives`

Synchronization primitives for coordinating GPU access:

```rust
pub struct SyncPrimitives {
    pub semaphore: Option<SyncHandle>,
    pub fence: Option<SyncHandle>,
}
```

---

## Vulkan Backend

### `VulkanTextureShareManager`

Manager for Vulkan texture sharing.

**Creation:**
```rust
use geyser::vulkan::VulkanTextureShareManager;
use std::sync::Arc;

let manager = VulkanTextureShareManager::new(
    Arc::new(instance),
    Arc::new(device),
    physical_device,
    queue_family_index,
)?;
```

### Platform-Specific Features

#### Windows (HANDLE-based)

Uses `VK_KHR_external_memory_win32` extension.

**Synchronization:**
```rust
// Create exportable semaphore
let semaphore = manager.create_exportable_semaphore()?;

// Export for sharing
let sem_handle = manager.export_semaphore_win32(semaphore)?;

// Import in another process
let imported_sem = manager.import_semaphore_win32(&sem_handle)?;
```

#### Linux (FD-based)

Uses `VK_KHR_external_memory_fd` extension.

**Synchronization:**
```rust
// Create exportable fence
let fence = manager.create_exportable_fence()?;

// Export for sharing
let fence_handle = manager.export_fence_fd(fence)?;

// Import in another process
let imported_fence = manager.import_fence_fd(&fence_handle)?;
```

### Vulkan Handle Types

```rust
pub struct VulkanTextureShareHandle {
    pub raw_handle: u64,
    pub memory_type_index: u32,
    pub size: u64,
    pub handle_type: vk::ExternalMemoryHandleTypeFlags,
    pub dedicated_allocation: bool,
}

pub struct VulkanSemaphoreHandle {
    pub raw_handle: u64,
    pub handle_type: vk::ExternalSemaphoreHandleTypeFlags,
}

pub struct VulkanFenceHandle {
    pub raw_handle: u64,
    pub handle_type: vk::ExternalFenceHandleTypeFlags,
}
```

---

## Metal Backend

### `MetalTextureShareManager`

Manager for Metal texture sharing using IOSurface.

**Creation:**
```rust
use geyser::metal::MetalTextureShareManager;
use metal::Device;
use std::sync::Arc;

let device = Arc::new(Device::system_default().expect("No Metal device"));
let manager = MetalTextureShareManager::new(device)?;
```

### IOSurface-based Sharing

Metal uses IOSurface for cross-process texture sharing:

```rust
// Create shareable texture (backed by IOSurface)
let texture = manager.create_shareable_texture(&descriptor)?;

// Export IOSurface ID
let handle = manager.export_texture(texture.as_ref())?;

// In another process, import using IOSurface ID
let imported = manager.import_texture(handle, &descriptor)?;
```

### Metal Synchronization

Using `MTLSharedEvent` for cross-process synchronization:

```rust
// Create shared event
let event = manager.create_shared_event()?;

// Export event handle
let event_handle = manager.export_shared_event(&event)?;

// Signal event (CPU-side)
manager.signal_event(&event, 1);

// Import in another process
let imported_event = manager.import_shared_event(&event_handle)?;

// Wait for event
manager.wait_for_event(&imported_event, 1)?;
```

### Metal Handle Types

```rust
pub struct MetalTextureShareHandle {
    pub io_surface_id: u32,
}

pub struct MetalEventHandle {
    pub shared_event_id: u64,
}
```

---

## Error Handling

### `GeyserError`

All operations return `Result<T, GeyserError>`.

```rust
pub enum GeyserError {
    InvalidTextureHandle,
    OperationNotSupported,
    VulkanApiError(String),
    VulkanInitializationError(String),
    MetalApiError(String),
    MetalInitializationError(String),
    Other(String),
}
```

**Error Handling Example:**
```rust
match manager.create_shareable_texture(&descriptor) {
    Ok(texture) => {
        // Use texture
    }
    Err(GeyserError::VulkanApiError(msg)) => {
        eprintln!("Vulkan error: {}", msg);
    }
    Err(e) => {
        eprintln!("Other error: {:?}", e);
    }
}
```

---

## Best Practices

### 1. Resource Lifetime Management

Always release handles when done:

```rust
// Export texture
let handle = manager.export_texture(texture.as_ref())?;

// ... use in other process ...

// Clean up
manager.release_texture_handle(handle)?;
```

### 2. Synchronization

Use appropriate sync primitives for cross-process coordination:

```rust
// Writer process
let semaphore = manager.create_exportable_semaphore()?;
let sem_handle = manager.export_semaphore_win32(semaphore)?;

// ... render to texture ...
// ... signal semaphore ...

// Reader process (import sem_handle)
let imported_sem = manager.import_semaphore_win32(&sem_handle)?;
// ... wait for semaphore before reading texture ...
```

### 3. Format Compatibility

Ensure format compatibility across APIs:

```rust
// Some formats may have different representations
// Always verify format support before sharing
let descriptor = TextureDescriptor {
    format: TextureFormat::Rgba8Unorm, // Widely supported
    // ...
};
```

### 4. Thread Safety

All managers are thread-safe:

```rust
let manager = Arc::new(manager);
let manager_clone = manager.clone();

std::thread::spawn(move || {
    manager_clone.create_shareable_texture(&descriptor)
});
```

### 5. Platform-Specific Code

Use conditional compilation for platform-specific features:

```rust
#[cfg(target_os = "windows")]
{
    let handle = manager.export_semaphore_win32(semaphore)?;
}

#[cfg(target_os = "linux")]
{
    let handle = manager.export_semaphore_fd(semaphore)?;
}
```

### 6. Error Recovery

Always handle errors gracefully:

```rust
fn create_and_export_texture(
    manager: &VulkanTextureShareManager,
    descriptor: &TextureDescriptor,
) -> Result<ApiTextureHandle> {
    let texture = manager.create_shareable_texture(descriptor)?;
    let handle = manager.export_texture(texture.as_ref())?;
    Ok(handle)
}

// Usage
match create_and_export_texture(&manager, &descriptor) {
    Ok(handle) => {
        // Success
    }
    Err(e) => {
        eprintln!("Failed to create/export texture: {:?}", e);
        // Fallback logic
    }
}
```

---

## Performance Considerations

### Memory Allocation

- Shareable textures use dedicated allocations for external memory compatibility
- This may have higher memory overhead than regular textures
- Consider pooling textures for frequently created/destroyed resources

### Storage Modes

**Metal:**
- IOSurface-backed textures use `MTLStorageMode::Shared`
- Shared mode allows CPU access but may be slower than Private
- Use Private mode for non-shared textures

**Vulkan:**
- External memory requires dedicated allocations
- Memory is GPU-only by default for best performance

### Synchronization Overhead

- Synchronization primitives add overhead
- Use them only when necessary for correctness
- Consider grouping operations to reduce sync frequency

---

## See Also

- [Examples Directory](../examples/README.md)
- [Vulkan Backend Details](../src/vulkan/README.md)
- [Metal Backend Details](../src/metal/README.md)
- [Testing Guide](../tests/README.md)
