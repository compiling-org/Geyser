# Geyser Phase 2 Implementation Roadmap

## 🎯 Overview

This document outlines the concrete steps to complete Phase 2 of the Geyser project, focusing on making the texture sharing functionality fully operational with real external memory implementation and cross-process capabilities.

---

## ✅ Phase 1 Recap (Completed)

- ✅ Core architecture and trait system
- ✅ Vulkan backend structure (placeholder external memory)
- ✅ Metal backend with IOSurface
- ✅ 26 texture formats supported
- ✅ Synchronization primitive types defined
- ✅ Comprehensive examples (3 functional)
- ✅ Integration test suite
- ✅ 1,500+ lines of documentation

---

## 🚀 Phase 2 Goals

1. **Real External Memory** - Implement actual Vulkan external memory export/import
2. **Synchronization** - Add working sync primitives
3. **Cross-Process** - Real IPC-based texture sharing
4. **Testing** - Validate on actual hardware
5. **Performance** - Benchmarks and optimization

---

## 📋 Implementation Tasks

### Sprint 1: Core Functionality (2-3 weeks)

#### Task 1.1: Windows Vulkan External Memory ⚡ CRITICAL
**File:** `src/vulkan/mod.rs`  
**Priority:** 🔴 HIGHEST  
**Estimated Time:** 3-5 days

**What to Implement:**
```rust
// In VulkanTextureShareManager

#[cfg(target_os = "windows")]
fn export_external_memory_win32(&self, memory: vk::DeviceMemory) -> Result<u64> {
    use ash::extensions::khr::ExternalMemoryWin32;
    
    let ext = ExternalMemoryWin32::new(&self.instance, &self.device);
    
    let handle_info = vk::MemoryGetWin32HandleInfoKHR::builder()
        .memory(memory)
        .handle_type(vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32);
    
    let handle = unsafe {
        ext.get_memory_win32_handle_khr(&handle_info)?
    };
    
    Ok(handle as u64)
}

#[cfg(target_os = "windows")]
fn import_external_memory_win32(&self, handle: u64, size: u64) -> Result<vk::DeviceMemory> {
    let import_info = vk::ImportMemoryWin32HandleInfoKHR::builder()
        .handle_type(vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32)
        .handle(handle as *mut std::ffi::c_void);
    
    let alloc_info = vk::MemoryAllocateInfo::builder()
        .allocation_size(size)
        .memory_type_index(/* determine from requirements */)
        .push_next(&mut import_info);
    
    unsafe { self.device.allocate_memory(&alloc_info, None) }
}
```

**Steps:**
1. Add `VK_KHR_external_memory_win32` extension loading
2. Modify `create_shareable_texture` to use `ExportMemoryAllocateInfo`
3. Implement `export_external_memory_win32`
4. Implement `import_external_memory_win32`
5. Update `export_texture` to call new function
6. Update `import_texture` to call new function
7. Test on Windows with Vulkan SDK

**Validation:**
- Create texture in process A
- Export handle
- Import handle in process A (different manager)
- Verify texture properties match

---

#### Task 1.2: Linux Vulkan External Memory
**File:** `src/vulkan/mod.rs`  
**Priority:** 🟡 HIGH  
**Estimated Time:** 3-5 days

Similar to Windows, but using:
- `VK_KHR_external_memory_fd`
- File descriptors instead of Win32 handles
- `get_memory_fd_khr` and `ImportMemoryFdInfoKHR`

---

#### Task 1.3: Vulkan External Memory Tests
**File:** `tests/external_memory_tests.rs`  
**Priority:** 🟡 HIGH  
**Estimated Time:** 2 days

Create tests that verify:
- Real handle export/import
- Memory dedications
- Handle validity
- Cross-manager sharing

---

### Sprint 2: Synchronization (2-3 weeks)

#### Task 2.1: Vulkan Semaphore Export/Import
**File:** `src/vulkan/mod.rs`  
**Priority:** 🟡 MEDIUM-HIGH  
**Estimated Time:** 3-4 days

**API Addition:**
```rust
impl VulkanTextureShareManager {
    pub fn create_shared_semaphore(&self) -> Result<vk::Semaphore>;
    pub fn export_semaphore(&self, semaphore: vk::Semaphore) -> Result<VulkanSemaphoreHandle>;
    pub fn import_semaphore(&self, handle: VulkanSemaphoreHandle) -> Result<vk::Semaphore>;
}
```

**Extensions:**
- `VK_KHR_external_semaphore`
- `VK_KHR_external_semaphore_win32` / `_fd`

---

#### Task 2.2: Metal Shared Events
**File:** `src/metal/mod.rs`  
**Priority:** 🟡 MEDIUM  
**Estimated Time:** 2-3 days

```rust
impl MetalTextureShareManager {
    pub fn create_shared_event(&self) -> Result<metal::SharedEvent>;
    pub fn export_event(&self, event: &metal::SharedEvent) -> Result<MetalEventHandle>;
    pub fn import_event(&self, handle: MetalEventHandle) -> Result<metal::SharedEvent>;
}
```

---

#### Task 2.3: Synchronization API
**File:** `src/lib.rs`  
**Priority:** 🟡 MEDIUM  
**Estimated Time:** 2 days

Extend `TextureShareManager` trait:
```rust
pub trait TextureShareManager {
    // ... existing methods
    
    fn wait_for_texture(&self, texture: &dyn SharedTexture, timeout_ns: u64) -> Result<()>;
    fn signal_texture_ready(&self, texture: &dyn SharedTexture) -> Result<()>;
}
```

---

### Sprint 3: Cross-Process Examples (2-3 weeks)

#### Task 3.1: IPC Utilities
**File:** `examples/shared/ipc.rs`  
**Priority:** 🟡 MEDIUM  
**Estimated Time:** 3-4 days

Implement:
- Named pipe communication (Windows)
- Unix domain sockets (Linux/macOS)
- Handle serialization/deserialization
- Simple protocol for handle passing

```rust
pub struct TextureHandleMessage {
    pub handle_data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
}

pub fn send_handle(handle: &ApiTextureHandle) -> Result<()>;
pub fn receive_handle() -> Result<(ApiTextureHandle, TextureDescriptor)>;
```

---

#### Task 3.2: Producer Binary
**File:** `examples/producer.rs`  
**Priority:** 🟡 MEDIUM  
**Estimated Time:** 2 days

Creates texture, exports, sends handle via IPC:
```rust
fn main() -> Result<()> {
    let manager = create_manager()?;
    let texture = manager.create_shareable_texture(&desc)?;
    let handle = manager.export_texture(texture.as_ref())?;
    
    ipc::send_handle(&handle)?;
    
    // Keep texture alive
    std::thread::park();
}
```

---

#### Task 3.3: Consumer Binary
**File:** `examples/consumer.rs`  
**Priority:** 🟡 MEDIUM  
**Estimated Time:** 2 days

Receives handle, imports texture, uses it:
```rust
fn main() -> Result<()> {
    let (handle, desc) = ipc::receive_handle()?;
    
    let manager = create_manager()?;
    let texture = manager.import_texture(handle, &desc)?;
    
    // Use texture for rendering
    render_with_texture(texture)?;
}
```

---

### Sprint 4: Performance & Polish (1-2 weeks)

#### Task 4.1: Benchmark Suite
**File:** `benches/texture_ops.rs`  
**Priority:** 🟢 MEDIUM  
**Estimated Time:** 2-3 days

Measure:
- Texture creation time
- Export latency
- Import latency
- Cross-process roundtrip
- Memory overhead

---

#### Task 4.2: Documentation Updates
**Files:** Various  
**Priority:** 🟢 MEDIUM  
**Estimated Time:** 2-3 days

Update:
- `PHASE2_SUMMARY.md` with results
- `README.md` with real capabilities
- Examples README with new binaries
- Architecture docs with sync primitives

---

#### Task 4.3: CI/CD Improvements
**File:** `.github/workflows/ci.yml`  
**Priority:** 🟢 LOW  
**Estimated Time:** 1 day

Add:
- Cross-process test job
- Benchmark tracking
- Multiple OS testing

---

## 🔧 Technical Considerations

### Windows-Specific
- **Handle Lifetime**: Win32 HANDLEs need explicit closing via `CloseHandle`
- **Security**: Handles may need security descriptors for cross-process
- **Duplication**: May need `DuplicateHandle` for handle passing

### Linux-Specific
- **File Descriptors**: Need proper `dup`/`close` handling
- **DMA-BUF**: Consider using DMA-BUF instead of opaque FDs
- **Permissions**: File descriptor passing requires proper socket permissions

### macOS-Specific
- **IOSurface**: Already working, just needs sync
- **MoltenVK**: Cross-API requires MoltenVK awareness

---

## 📊 Success Criteria

### Phase 2 Complete When:
1. ✅ Real external memory working on Windows
2. ✅ Real external memory working on Linux  
3. ✅ Synchronization primitives functional
4. ✅ Producer/consumer example works cross-process
5. ✅ Tests passing on Windows + Linux
6. ✅ Benchmarks show acceptable performance
7. ✅ Documentation updated

---

## 🚨 Known Risks & Mitigation

### Risk 1: GPU Allocator Incompatibility
**Impact:** HIGH  
**Mitigation:** May need manual `vk::DeviceMemory` allocation for exported textures

### Risk 2: Platform-Specific Bugs
**Impact:** MEDIUM  
**Mitigation:** Test on actual hardware early, maintain platform-specific code paths

### Risk 3: Performance Issues
**Impact:** MEDIUM  
**Mitigation:** Early benchmarking, profiling, optimization iteration

### Risk 4: Security Concerns
**Impact:** LOW-MEDIUM  
**Mitigation:** Handle validation, timeout mechanisms, proper cleanup

---

## 📅 Timeline Estimate

- **Sprint 1 (Core):** 2-3 weeks
- **Sprint 2 (Sync):** 2-3 weeks  
- **Sprint 3 (Cross-Process):** 2-3 weeks
- **Sprint 4 (Polish):** 1-2 weeks

**Total:** 7-11 weeks (part-time development)

---

## 🎯 Next Immediate Actions

### This Week:
1. Start Task 1.1: Windows external memory
2. Set up Vulkan SDK with external memory extensions
3. Create simple test case for export/import

### Next Week:
1. Complete Windows implementation
2. Test on actual Windows hardware
3. Start Linux implementation

### Month 1 Goal:
Real external memory working on Windows + Linux with tests passing

---

## 📚 Resources

- [Vulkan External Memory Spec](https://www.khronos.org/registry/vulkan/specs/1.3-extensions/html/vkspec.html#VK_KHR_external_memory)
- [Windows Handle Management](https://docs.microsoft.com/en-us/windows/win32/api/handleapi/)
- [Linux DMA-BUF](https://www.kernel.org/doc/html/latest/driver-api/dma-buf.html)
- [Metal Shared Events](https://developer.apple.com/documentation/metal/mtlsharedevent)

---

**Document Version:** 1.0  
**Last Updated:** 2025-10-23  
**Status:** 🟢 Active Development
