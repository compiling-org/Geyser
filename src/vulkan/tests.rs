//! Unit tests for Vulkan backend synchronization primitives

use super::*;

#[test]
fn test_vulkan_semaphore_handle_creation() {
    let handle = VulkanSemaphoreHandle {
        raw_handle: 12345,
        handle_type: vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_WIN32,
    };

    assert_eq!(handle.raw_handle, 12345);
    assert!(handle.handle_type.contains(vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_WIN32));
}

#[test]
fn test_vulkan_fence_handle_creation() {
    let handle = VulkanFenceHandle {
        raw_handle: 67890,
        handle_type: vk::ExternalFenceHandleTypeFlags::OPAQUE_WIN32,
    };

    assert_eq!(handle.raw_handle, 67890);
    assert!(handle.handle_type.contains(vk::ExternalFenceHandleTypeFlags::OPAQUE_WIN32));
}

#[test]
fn test_vulkan_semaphore_handle_clone() {
    let handle1 = VulkanSemaphoreHandle {
        raw_handle: 111,
        handle_type: vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_FD,
    };

    let handle2 = handle1.clone();

    assert_eq!(handle1.raw_handle, handle2.raw_handle);
}

#[test]
fn test_vulkan_fence_handle_clone() {
    let handle1 = VulkanFenceHandle {
        raw_handle: 222,
        handle_type: vk::ExternalFenceHandleTypeFlags::OPAQUE_FD,
    };

    let handle2 = handle1.clone();

    assert_eq!(handle1.raw_handle, handle2.raw_handle);
}

#[test]
fn test_vulkan_texture_share_handle() {
    let handle = VulkanTextureShareHandle {
        raw_handle: 999,
        memory_type_index: 0,
        size: 1024 * 1024,
        handle_type: vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32,
        dedicated_allocation: true,
    };

    assert_eq!(handle.raw_handle, 999);
    assert_eq!(handle.memory_type_index, 0);
    assert_eq!(handle.size, 1024 * 1024);
    assert!(handle.dedicated_allocation);
}

#[test]
fn test_sync_handle_variants() {
    use crate::common::SyncHandle;

    let sem_handle = VulkanSemaphoreHandle {
        raw_handle: 111,
        handle_type: vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_WIN32,
    };

    let fence_handle = VulkanFenceHandle {
        raw_handle: 222,
        handle_type: vk::ExternalFenceHandleTypeFlags::OPAQUE_WIN32,
    };

    let sync_sem = SyncHandle::VulkanSemaphore(sem_handle);
    let sync_fence = SyncHandle::VulkanFence(fence_handle);

    // Just verify they can be created
    match sync_sem {
        SyncHandle::VulkanSemaphore(_) => (),
        _ => panic!("Wrong variant"),
    }

    match sync_fence {
        SyncHandle::VulkanFence(_) => (),
        _ => panic!("Wrong variant"),
    }
}

#[test]
#[cfg(target_os = "windows")]
fn test_windows_handle_types() {
    assert!(vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_WIN32.as_raw() != 0);
    assert!(vk::ExternalFenceHandleTypeFlags::OPAQUE_WIN32.as_raw() != 0);
    assert!(vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32.as_raw() != 0);
}

#[test]
#[cfg(target_os = "linux")]
fn test_linux_handle_types() {
    assert!(vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_FD.as_raw() != 0);
    assert!(vk::ExternalFenceHandleTypeFlags::OPAQUE_FD.as_raw() != 0);
    assert!(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD.as_raw() != 0);
}
