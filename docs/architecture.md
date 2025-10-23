# Geyser Architecture

## Overview

Geyser is designed as a modular, trait-based system that abstracts GPU texture sharing across multiple graphics APIs.

## Core Components

### 1. Common Layer (`src/common/`)

- **TextureFormat**: API-agnostic texture format enumeration
- **ApiTextureHandle**: Enum containing platform-specific handles for sharing

### 2. Error Handling (`src/error.rs`)

- **GeyserError**: Comprehensive error types for all operations
- Conversions from platform-specific errors (Vulkan, Metal, etc.)

### 3. Core Traits (`src/lib.rs`)

- **SharedTexture**: Represents a shareable texture with common properties
- **TextureShareManager**: Factory for importing/exporting textures

### 4. Backend Implementations

#### Vulkan (`src/vulkan/`)
- Uses Vulkan external memory extensions
- Platform-specific: `VK_KHR_external_memory_fd` (Linux), `VK_KHR_external_memory_win32` (Windows)
- Leverages `ash` for Vulkan bindings

#### Metal (`src/metal/`)
- Uses IOSurface for texture sharing on macOS/iOS
- Direct Metal API bindings via `metal` crate

#### WebGPU (`src/webgpu/`)
- Future implementation
- Will leverage wgpu's texture sharing capabilities

## Sharing Workflow

### Export Flow
1. Create texture in API context A
2. Export texture via `TextureShareManager::export_texture()`
3. Returns `ApiTextureHandle` containing platform handle
4. Handle can be serialized/passed to another process

### Import Flow
1. Receive `ApiTextureHandle` from external source
2. Import via `TextureShareManager::import_shared_texture()`
3. Returns `Box<dyn SharedTexture>` usable in API context B

## Platform Considerations

### Linux
- Vulkan: File descriptors via `VK_KHR_external_memory_fd`
- DMA-BUF for zero-copy sharing

### Windows
- Vulkan: Win32 handles via `VK_KHR_external_memory_win32`
- DirectX interop possibilities

### macOS
- Metal: IOSurface for efficient sharing
- Vulkan via MoltenVK: Can bridge to IOSurface

## Future Enhancements

- Synchronization primitives (fences, semaphores)
- Mipmap and array texture support
- Compressed texture formats
- Multi-GPU scenarios
