# Cross-Process IPC with Geyser

This document explains how to share GPU textures between separate processes using Geyser's IPC (Inter-Process Communication) capabilities.

## Overview

Geyser enables **true zero-copy texture sharing** between processes by exporting and importing platform-specific GPU memory handles. This allows:

- **Separate render and display processes** (e.g., browser compositor model)
- **Multi-process game engines** (separate worlds, physics, rendering)
- **Video processing pipelines** (producer/consumer chains)
- **Distributed rendering** (render farm nodes)

## Architecture

### Handle Export/Import Flow

```
┌─────────────────┐                    ┌─────────────────┐
│   Producer      │                    │   Consumer      │
│   Process A     │                    │   Process B     │
├─────────────────┤                    ├─────────────────┤
│                 │                    │                 │
│ 1. Create       │                    │                 │
│    Texture      │                    │                 │
│                 │                    │                 │
│ 2. Export       │                    │                 │
│    Handle       │                    │                 │
│    (Win32       │                    │                 │
│     HANDLE/FD)  │                    │                 │
│         │       │                    │                 │
│         │       │   3. IPC Channel   │                 │
│         └───────┼───────────────────>│                 │
│                 │   (File/Pipe/      │                 │
│                 │    Socket/SHM)     │                 │
│                 │                    │                 │
│                 │                    │ 4. Import       │
│                 │                    │    Handle       │
│                 │                    │                 │
│ 5. Render       │                    │ 6. Read/Display │
│    to Texture   │<───────────────────┤    Texture      │
│                 │   Sync Semaphore   │                 │
│                 │                    │                 │
└─────────────────┘                    └─────────────────┘
        │                                      │
        └──────────────────┬───────────────────┘
                           │
                    ┌──────▼──────┐
                    │  Shared GPU │
                    │   Memory    │
                    └─────────────┘
```

### Key Components

1. **Texture Handle Export**
   - `VulkanTextureShareHandle` contains platform-specific handle
   - Includes memory size, type index, format metadata
   - Must be serialized and sent via IPC

2. **IPC Communication Layer**
   - Simple file-based (examples): `ipc_utils.rs`
   - Production: Named pipes, sockets, or shared memory
   - Passes handle + metadata between processes

3. **Synchronization Primitives**
   - Binary semaphores for GPU-GPU sync
   - Fences for CPU-GPU sync
   - Prevents race conditions between producer/consumer

## Platform-Specific Details

### Windows

**External Memory:**
- Uses `VK_KHR_external_memory_win32`
- Handle type: `HANDLE` (opaque Win32 handle)
- Serialized as `u64` (cast from `isize`)

**Synchronization:**
- Uses `VK_KHR_external_semaphore_win32`
- Uses `VK_KHR_external_fence_win32`

**IPC Mechanisms:**
- Named pipes: `\\\\.\\pipe\\geyser_ipc`
- File mapping: `CreateFileMappingW`
- Sockets: TCP/UDP localhost

### Linux

**External Memory:**
- Uses `VK_KHR_external_memory_fd`
- Handle type: File descriptor (`i32`)
- Must be duplicated across fork/exec

**Synchronization:**
- Uses `VK_KHR_external_semaphore_fd`
- Uses `VK_KHR_external_fence_fd`

**IPC Mechanisms:**
- Unix domain sockets
- Shared memory: `shm_open` + `mmap`
- Pipes: `pipe()` or `mkfifo()`

## Usage Examples

### Producer Process

```rust
use geyser::{
    vulkan::VulkanTextureShareManager,
    common::{TextureDescriptor, TextureFormat, TextureUsage},
    TextureShareManager,
};

// 1. Create Vulkan context (instance, device, physical device)
let manager = VulkanTextureShareManager::new(
    instance, device, physical_device, queue_family_index
)?;

// 2. Create shareable texture
let texture_desc = TextureDescriptor {
    width: 1920,
    height: 1080,
    format: TextureFormat::Rgba8Unorm,
    usage: vec![
        TextureUsage::RenderAttachment,
        TextureUsage::TextureBinding,
    ],
    label: Some("SharedFrame".to_string()),
};

let texture = manager.create_shareable_texture(&texture_desc)?;

// 3. Export handle
let handle = manager.export_texture(texture.as_ref())?;

// 4. Create and export synchronization semaphore
let semaphore = manager.create_exportable_semaphore()?;
#[cfg(target_os = "windows")]
let sem_handle = manager.export_semaphore_win32(semaphore)?;

// 5. Send handle metadata over IPC
// (serialize handle.raw_handle, width, height, format, etc.)
send_via_ipc(&handle, &texture_desc, &sem_handle)?;

// 6. Render and signal
record_render_commands(&texture);
submit_with_semaphore_signal(semaphore);

// 7. Notify consumer
notify_frame_ready()?;
```

### Consumer Process

```rust
use geyser::{
    vulkan::{VulkanTextureShareManager, VulkanTextureShareHandle},
    common::{ApiTextureHandle, TextureDescriptor},
    TextureShareManager,
};

// 1. Create separate Vulkan context
let manager = VulkanTextureShareManager::new(
    instance, device, physical_device, queue_family_index
)?;

// 2. Receive handle metadata from IPC
let (texture_handle, descriptor, sem_handle) = receive_via_ipc()?;

// 3. Reconstruct handle
let vulkan_handle = VulkanTextureShareHandle {
    raw_handle: texture_handle,
    memory_type_index,
    size,
    handle_type: vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32,
    dedicated_allocation: true,
};

// 4. Import texture
let imported = manager.import_texture(
    ApiTextureHandle::Vulkan(vulkan_handle),
    &descriptor,
)?;

// 5. Import semaphore
#[cfg(target_os = "windows")]
let semaphore = manager.import_semaphore_win32(&sem_handle)?;

// 6. Wait for frame ready notification
wait_for_frame_notification()?;

// 7. Wait on semaphore, then use texture
wait_semaphore(semaphore);
display_or_process(&imported)?;
```

## Running the Examples

### Basic IPC Example

**Terminal 1 (Producer):**
```bash
cargo run --example ipc_producer --features vulkan
```

**Terminal 2 (Consumer):**
```bash
cargo run --example ipc_consumer --features vulkan
```

The producer will:
1. Create a 1024x768 RGBA8 texture
2. Export handle and send via file-based IPC
3. Render 10 frames with notifications
4. Signal shutdown

The consumer will:
1. Wait for handle from producer
2. Import the shared texture
3. Process frame notifications
4. Cleanup on shutdown

## Synchronization Patterns

### Producer-Consumer (One-Way)

**Use case:** Video encoder, frame capture

```
Producer                    Consumer
   │                           │
   ├─ Render Frame 0           │
   ├─ Signal Semaphore ────────>│
   │                           ├─ Wait Semaphore
   │                           ├─ Process Frame 0
   ├─ Render Frame 1           │
   ├─ Signal Semaphore ────────>│
   │                           ├─ Wait Semaphore
   │                           └─ Process Frame 1
   └─ ...                      └─ ...
```

### Ping-Pong (Bidirectional)

**Use case:** Collaborative rendering, effects pipeline

```
Process A                   Process B
   │                           │
   ├─ Render Pass 1            │
   ├─ Signal Sem A ────────────>│
   │                           ├─ Wait Sem A
   │                           ├─ Render Pass 2
   │                           ├─ Signal Sem B
   │<────────────── Signal     │
   ├─ Wait Sem B               │
   └─ Continue...              └─ Continue...
```

### Multiple Consumers

**Use case:** Mirror display, multi-monitor

```
Producer
   │
   ├─ Create Texture
   ├─ Export Handle
   ├──────────┬───────────┐
   │          │           │
Consumer 1    Consumer 2  Consumer 3
   │          │           │
   └─ Display └─ Record   └─ Stream
```

## Best Practices

### 1. Handle Lifetime Management

- **Producer:** Keep original texture alive until all consumers release
- **Consumer:** Release imported handle before producer destroys texture
- Consider reference counting for multi-consumer scenarios

### 2. Synchronization

- **Always use semaphores** for GPU-GPU synchronization
- **Use fences** when CPU needs to wait on GPU work
- Avoid busy-waiting; prefer blocking IPC operations

### 3. Error Handling

```rust
// Producer: Handle consumer disconnect
match send_via_ipc(&handle) {
    Ok(_) => continue_rendering(),
    Err(e) if e.kind() == ErrorKind::BrokenPipe => {
        // Consumer disconnected, cleanup
        release_resources();
    }
    Err(e) => return Err(e),
}

// Consumer: Handle producer disconnect
match receive_via_ipc() {
    Ok(handle) => process(handle),
    Err(e) if e.kind() == ErrorKind::TimedOut => {
        // Producer may have crashed
        initiate_cleanup();
    }
    Err(e) => return Err(e),
}
```

### 4. Memory Pressure

- Limit number of in-flight frames (typically 2-3)
- Implement texture pooling for frequent create/destroy
- Monitor GPU memory usage across processes

### 5. Security Considerations

- Validate handle metadata before import
- Consider handle access permissions (Windows security descriptors)
- Avoid exposing raw handles over network without encryption

## Troubleshooting

### "Invalid external handle"

**Cause:** Handle was closed or invalid between export and import

**Solution:**
- Ensure producer keeps texture alive until consumer imports
- Check handle serialization (u64 cast on Windows)
- Verify extensions are enabled on both contexts

### "Memory type mismatch"

**Cause:** Different memory heaps between devices

**Solution:**
- Query memory properties on both devices
- Use compatible memory types (device-local + exportable)
- Consider fallback to host-visible memory

### "Semaphore import failed"

**Cause:** Semaphore was destroyed or not yet created

**Solution:**
- Export semaphore before sending via IPC
- Keep semaphore alive during sharing
- Import in correct order (texture, then sync primitives)

### "Access violation / segfault"

**Cause:** Race condition, memory already freed

**Solution:**
- Use proper semaphore synchronization
- Implement handshake protocol (ready signals)
- Validate handle before use

## Performance Considerations

### Latency

- **File-based IPC:** ~1-5ms (simple, examples only)
- **Named pipes:** ~0.1-0.5ms (production)
- **Shared memory:** ~0.01-0.1ms (best performance)
- **Texture import:** ~0.5-2ms (Vulkan overhead)

### Bandwidth

- **Zero-copy sharing:** No data transfer overhead
- **Only metadata** sent over IPC (~100 bytes)
- **GPU memory** remains on device, accessible by both processes

### Scaling

- Multiple consumers add minimal overhead
- Consider texture pooling for high-frequency sharing
- Use timeline semaphores (Phase 2+) for better pipelining

## Future Improvements (Phase 2+)

- [ ] Timeline semaphores for advanced sync patterns
- [ ] Cross-API sharing (Vulkan ↔ Metal, Vulkan ↔ DX12)
- [ ] Automatic handle duplication (fork/exec handling)
- [ ] Built-in IPC library (no external dependency)
- [ ] Multi-GPU scenarios (explicit device selection)
- [ ] Shared memory allocator for metadata passing

## See Also

- [BENCHMARKS.md](BENCHMARKS.md) - Performance metrics
- [VULKAN.md](VULKAN.md) - Vulkan backend details
- [examples/ipc_producer.rs](../examples/ipc_producer.rs)
- [examples/ipc_consumer.rs](../examples/ipc_consumer.rs)
- [examples/ipc_utils.rs](../examples/ipc_utils.rs)

## References

- [Vulkan External Memory Spec](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VK_KHR_external_memory.html)
- [Vulkan External Semaphore Spec](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VK_KHR_external_semaphore.html)
- [Windows Handle Duplication](https://docs.microsoft.com/en-us/windows/win32/api/handleapi/nf-handleapi-duplicatehandle)
- [Linux File Descriptor Passing](https://man7.org/linux/man-pages/man7/unix.7.html)

---

**Last Updated:** Phase 2 Development  
**Version:** 0.2.0
