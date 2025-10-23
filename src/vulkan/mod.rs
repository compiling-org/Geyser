//! Vulkan backend for texture sharing.

use ash::{
    vk,
    Device,
    Instance,
};
use gpu_allocator::{
    vulkan::{Allocator, AllocatorCreateDesc, Allocation, AllocationCreateDesc, AllocationScheme},
    MemoryLocation,
};
use std::{
    any::Any,
    sync::{Arc, Mutex},
    collections::HashMap,
};
use crate::{
    common::{ApiTextureHandle, TextureDescriptor, TextureFormat, TextureUsage},
    error::{GeyserError, Result},
    SharedTexture, TextureShareManager,
};

// --- API-Specific Handle for Vulkan ---
// This struct will contain the necessary information to re-create/import a Vulkan image
// from an external memory handle (e.g., a file descriptor on Linux, or a Windows handle).
// It needs to be serializable for inter-process communication.
#[derive(Debug, Clone)]
pub struct VulkanTextureShareHandle {
    // Platform-specific handle type. For Linux, this would be an integer file descriptor.
    // For Windows, a `HANDLE` (which is a raw pointer on 64-bit, but often represented as u64).
    pub raw_handle: u64, // External memory handle (FD on Linux, HANDLE on Windows)
    pub memory_type_index: u32,
    pub size: u64, // Size of the external memory allocation
    pub handle_type: vk::ExternalMemoryHandleTypeFlags,
    pub dedicated_allocation: bool,
}

/// Vulkan semaphore handle for synchronization.
#[derive(Debug, Clone)]
pub struct VulkanSemaphoreHandle {
    pub raw_handle: u64,
    pub handle_type: vk::ExternalSemaphoreHandleTypeFlags,
}

/// Vulkan fence handle for synchronization.
#[derive(Debug, Clone)]
pub struct VulkanFenceHandle {
    pub raw_handle: u64,
    pub handle_type: vk::ExternalFenceHandleTypeFlags,
}

// --- Vulkan Specific SharedTexture Implementation ---
pub struct VulkanSharedTexture {
    device: Arc<Device>,
    allocation: Option<Allocation>, // Owned allocation if created here
    image: vk::Image,
    image_view: Option<vk::ImageView>, // Optional, depending on usage
    descriptor: TextureDescriptor,
    // Potentially store the native handle if exported
    pub(crate) exported_handle: Option<VulkanTextureShareHandle>,
}

impl SharedTexture for VulkanSharedTexture {
    fn width(&self) -> u32 { self.descriptor.width }
    fn height(&self) -> u32 { self.descriptor.height }
    fn format(&self) -> TextureFormat { self.descriptor.format }
    fn usage(&self) -> &[TextureUsage] { &self.descriptor.usage }
    fn as_any(&self) -> &dyn Any { self }
}

impl Drop for VulkanSharedTexture {
    fn drop(&mut self) {
        unsafe {
            if let Some(view) = self.image_view.take() {
                self.device.destroy_image_view(view, None);
            }
            self.device.destroy_image(self.image, None);
            // Don't free allocation here if it was imported or exported
            // Allocation should be handled by the allocator or `TextureShareManager`'s release.
        }
    }
}

// --- Vulkan Specific TextureShareManager Implementation ---

/// Represents the Vulkan context needed for sharing operations.
pub struct VulkanTextureShareManager {
    instance: Arc<Instance>,
    device: Arc<Device>,
    physical_device: vk::PhysicalDevice,
    queue_family_index: u32,
    allocator: Mutex<Allocator>,
    // Store exported resources to manage their lifetime
    // (e.g., `vk::DeviceMemory` and associated external handles)
    exported_resources: Mutex<HashMap<u64, vk::DeviceMemory>>,
    // Store exported sync primitives
    exported_semaphores: Mutex<HashMap<u64, vk::Semaphore>>,
    exported_fences: Mutex<HashMap<u64, vk::Fence>>,
    #[cfg(target_os = "windows")]
    external_memory_win32: ash::khr::external_memory_win32::Device,
    #[cfg(target_os = "linux")]
    external_memory_fd: ash::khr::external_memory_fd::Device,
    #[cfg(target_os = "windows")]
    external_semaphore_win32: ash::khr::external_semaphore_win32::Device,
    #[cfg(target_os = "linux")]
    external_semaphore_fd: ash::khr::external_semaphore_fd::Device,
    #[cfg(target_os = "windows")]
    external_fence_win32: ash::khr::external_fence_win32::Device,
    #[cfg(target_os = "linux")]
    external_fence_fd: ash::khr::external_fence_fd::Device,
}

impl VulkanTextureShareManager {
    pub fn new(
        instance: Arc<Instance>,
        device: Arc<Device>,
        physical_device: vk::PhysicalDevice,
        queue_family_index: u32,
    ) -> Result<Self> {
        let allocator = Allocator::new(&AllocatorCreateDesc {
            instance: (*instance).clone(),
            device: (*device).clone(),
            physical_device,
            debug_settings: Default::default(),
            buffer_device_address: false, // Change if using buffer device address
            allocation_sizes: Default::default(),
        }).map_err(|e| GeyserError::VulkanInitializationError(format!("Failed to create GPU allocator: {}", e)))?;

        #[cfg(target_os = "windows")]
        let external_memory_win32 = ash::khr::external_memory_win32::Device::new(&*instance, &*device);

        #[cfg(target_os = "linux")]
        let external_memory_fd = ash::khr::external_memory_fd::Device::new(&*instance, &*device);

        #[cfg(target_os = "windows")]
        let external_semaphore_win32 = ash::khr::external_semaphore_win32::Device::new(&*instance, &*device);

        #[cfg(target_os = "linux")]
        let external_semaphore_fd = ash::khr::external_semaphore_fd::Device::new(&*instance, &*device);

        #[cfg(target_os = "windows")]
        let external_fence_win32 = ash::khr::external_fence_win32::Device::new(&*instance, &*device);

        #[cfg(target_os = "linux")]
        let external_fence_fd = ash::khr::external_fence_fd::Device::new(&*instance, &*device);

        Ok(Self {
            instance,
            device,
            physical_device,
            queue_family_index,
            allocator: Mutex::new(allocator),
            exported_resources: Mutex::new(HashMap::new()),
            exported_semaphores: Mutex::new(HashMap::new()),
            exported_fences: Mutex::new(HashMap::new()),
            #[cfg(target_os = "windows")]
            external_memory_win32,
            #[cfg(target_os = "linux")]
            external_memory_fd,
            #[cfg(target_os = "windows")]
            external_semaphore_win32,
            #[cfg(target_os = "linux")]
            external_semaphore_fd,
            #[cfg(target_os = "windows")]
            external_fence_win32,
            #[cfg(target_os = "linux")]
            external_fence_fd,
        })
    }

    // Helper to convert `TextureFormat` to `vk::Format`
    fn map_texture_format_to_vk(&self, format: TextureFormat) -> Result<vk::Format> {
        match format {
            // 8-bit formats
            TextureFormat::Rgba8Unorm => Ok(vk::Format::R8G8B8A8_UNORM),
            TextureFormat::Bgra8Unorm => Ok(vk::Format::B8G8R8A8_UNORM),
            TextureFormat::Rgba8Srgb => Ok(vk::Format::R8G8B8A8_SRGB),
            TextureFormat::Bgra8Srgb => Ok(vk::Format::B8G8R8A8_SRGB),
            TextureFormat::R8Unorm => Ok(vk::Format::R8_UNORM),
            TextureFormat::Rg8Unorm => Ok(vk::Format::R8G8_UNORM),
            
            // 16-bit formats
            TextureFormat::R16Float => Ok(vk::Format::R16_SFLOAT),
            TextureFormat::Rg16Float => Ok(vk::Format::R16G16_SFLOAT),
            TextureFormat::Rgba16Float => Ok(vk::Format::R16G16B16A16_SFLOAT),
            TextureFormat::R16Uint => Ok(vk::Format::R16_UINT),
            TextureFormat::R16Sint => Ok(vk::Format::R16_SINT),
            
            // 32-bit formats
            TextureFormat::R32Float => Ok(vk::Format::R32_SFLOAT),
            TextureFormat::Rg32Float => Ok(vk::Format::R32G32_SFLOAT),
            TextureFormat::Rgba32Float => Ok(vk::Format::R32G32B32A32_SFLOAT),
            TextureFormat::R32Uint => Ok(vk::Format::R32_UINT),
            TextureFormat::R32Sint => Ok(vk::Format::R32_SINT),
            
            // Depth/Stencil formats
            TextureFormat::Depth32Float => Ok(vk::Format::D32_SFLOAT),
            TextureFormat::Depth24Plus => Ok(vk::Format::D24_UNORM_S8_UINT),
            TextureFormat::Depth24PlusStencil8 => Ok(vk::Format::D24_UNORM_S8_UINT),
            
            // HDR formats
            TextureFormat::Rgb10a2Unorm => Ok(vk::Format::A2R10G10B10_UNORM_PACK32),
            TextureFormat::Rg11b10Float => Ok(vk::Format::B10G11R11_UFLOAT_PACK32),
        }
    }

    // Helper to convert `TextureUsage` to `vk::ImageUsageFlags` and `vk::ImageAspectFlags`
    fn map_texture_usage_to_vk(&self, usages: &[TextureUsage]) -> (vk::ImageUsageFlags, vk::ImageAspectFlags) {
        let mut image_usage = vk::ImageUsageFlags::empty();
        let mut image_aspect = vk::ImageAspectFlags::empty();

        for usage in usages {
            match usage {
                TextureUsage::CopySrc => image_usage |= vk::ImageUsageFlags::TRANSFER_SRC,
                TextureUsage::CopyDst => image_usage |= vk::ImageUsageFlags::TRANSFER_DST,
                TextureUsage::TextureBinding => {
                    image_usage |= vk::ImageUsageFlags::SAMPLED;
                    image_aspect |= vk::ImageAspectFlags::COLOR; // Assuming color textures for now
                }
                TextureUsage::RenderAttachment => {
                    image_usage |= vk::ImageUsageFlags::COLOR_ATTACHMENT;
                    image_aspect |= vk::ImageAspectFlags::COLOR;
                }
                TextureUsage::StorageBinding => {
                    image_usage |= vk::ImageUsageFlags::STORAGE;
                    image_aspect |= vk::ImageAspectFlags::COLOR;
                }
            }
        }
        (image_usage, image_aspect)
    }

    // Helper to get memory properties for external memory
    // This part is highly platform-dependent (Linux `FD`, Windows `HANDLE`)
    #[cfg(target_os = "linux")]
    fn get_external_memory_fd_info(&self, memory: vk::DeviceMemory) -> Result<i32> {
        // Export the memory as a Linux FD using VK_KHR_external_memory_fd
        let get_fd_info = vk::MemoryGetFdInfoKHR {
            s_type: vk::StructureType::MEMORY_GET_FD_INFO_KHR,
            p_next: std::ptr::null(),
            memory,
            handle_type: vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD,
            _marker: std::marker::PhantomData,
        };

        unsafe {
            self.external_memory_fd
                .get_memory_fd(&get_fd_info)
                .map_err(|e| GeyserError::VulkanApiError(format!("Failed to get FD: {:?}", e)))
        }
    }

    #[cfg(target_os = "windows")]
    fn get_external_memory_win32_info(&self, memory: vk::DeviceMemory) -> Result<u64> {
        // Export the memory as a Windows HANDLE using VK_KHR_external_memory_win32
        let get_handle_info = vk::MemoryGetWin32HandleInfoKHR {
            s_type: vk::StructureType::MEMORY_GET_WIN32_HANDLE_INFO_KHR,
            p_next: std::ptr::null(),
            memory,
            handle_type: vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32,
            _marker: std::marker::PhantomData,
        };

        unsafe {
            self.external_memory_win32
                .get_memory_win32_handle(&get_handle_info)
                .map(|handle| handle as u64)
                .map_err(|e| GeyserError::VulkanApiError(format!("Failed to get Win32 handle: {:?}", e)))
        }
    }

    // --- Synchronization Primitive Methods ---

    /// Create an exportable semaphore for cross-process synchronization
    pub fn create_exportable_semaphore(&self) -> Result<vk::Semaphore> {
        let handle_types = {
            #[cfg(target_os = "linux")]
            { vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_FD }
            #[cfg(target_os = "windows")]
            { vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_WIN32 }
            #[cfg(not(any(target_os = "linux", target_os = "windows")))]
            { vk::ExternalSemaphoreHandleTypeFlags::empty() }
        };

        let mut export_semaphore_info = vk::ExportSemaphoreCreateInfo {
            s_type: vk::StructureType::EXPORT_SEMAPHORE_CREATE_INFO,
            p_next: std::ptr::null(),
            handle_types,
            _marker: std::marker::PhantomData,
        };

        let semaphore_create_info = vk::SemaphoreCreateInfo {
            s_type: vk::StructureType::SEMAPHORE_CREATE_INFO,
            p_next: &mut export_semaphore_info as *mut _ as *const std::ffi::c_void,
            flags: vk::SemaphoreCreateFlags::empty(),
            _marker: std::marker::PhantomData,
        };

        unsafe {
            self.device.create_semaphore(&semaphore_create_info, None)
                .map_err(|e| GeyserError::VulkanApiError(format!("Failed to create semaphore: {:?}", e)))
        }
    }

    /// Export a semaphore handle for sharing
    #[cfg(target_os = "windows")]
    pub fn export_semaphore_win32(&self, semaphore: vk::Semaphore) -> Result<VulkanSemaphoreHandle> {
        let get_handle_info = vk::SemaphoreGetWin32HandleInfoKHR {
            s_type: vk::StructureType::SEMAPHORE_GET_WIN32_HANDLE_INFO_KHR,
            p_next: std::ptr::null(),
            semaphore,
            handle_type: vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_WIN32,
            _marker: std::marker::PhantomData,
        };

        let raw_handle = unsafe {
            self.external_semaphore_win32
                .get_semaphore_win32_handle(&get_handle_info)
                .map(|h| h as u64)
                .map_err(|e| GeyserError::VulkanApiError(format!("Failed to export semaphore: {:?}", e)))?
        };

        self.exported_semaphores.lock().unwrap().insert(raw_handle, semaphore);

        Ok(VulkanSemaphoreHandle {
            raw_handle,
            handle_type: vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_WIN32,
        })
    }

    #[cfg(target_os = "linux")]
    pub fn export_semaphore_fd(&self, semaphore: vk::Semaphore) -> Result<VulkanSemaphoreHandle> {
        let get_fd_info = vk::SemaphoreGetFdInfoKHR {
            s_type: vk::StructureType::SEMAPHORE_GET_FD_INFO_KHR,
            p_next: std::ptr::null(),
            semaphore,
            handle_type: vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_FD,
            _marker: std::marker::PhantomData,
        };

        let raw_handle = unsafe {
            self.external_semaphore_fd
                .get_semaphore_fd(&get_fd_info)
                .map(|fd| fd as u64)
                .map_err(|e| GeyserError::VulkanApiError(format!("Failed to export semaphore: {:?}", e)))?
        };

        self.exported_semaphores.lock().unwrap().insert(raw_handle, semaphore);

        Ok(VulkanSemaphoreHandle {
            raw_handle,
            handle_type: vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_FD,
        })
    }

    /// Import a semaphore from an external handle
    #[cfg(target_os = "windows")]
    pub fn import_semaphore_win32(&self, handle: &VulkanSemaphoreHandle) -> Result<vk::Semaphore> {
        let mut import_info = vk::ImportSemaphoreWin32HandleInfoKHR {
            s_type: vk::StructureType::IMPORT_SEMAPHORE_WIN32_HANDLE_INFO_KHR,
            p_next: std::ptr::null(),
            semaphore: vk::Semaphore::null(),
            flags: vk::SemaphoreImportFlags::empty(),
            handle_type: vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_WIN32,
            handle: handle.raw_handle as isize,
            name: std::ptr::null(),
            _marker: std::marker::PhantomData,
        };

        // First create the semaphore
        let semaphore = self.create_exportable_semaphore()?;
        import_info.semaphore = semaphore;

        unsafe {
            self.external_semaphore_win32
                .import_semaphore_win32_handle(&import_info)
                .map_err(|e| GeyserError::VulkanApiError(format!("Failed to import semaphore: {:?}", e)))?;
        }

        Ok(semaphore)
    }

    #[cfg(target_os = "linux")]
    pub fn import_semaphore_fd(&self, handle: &VulkanSemaphoreHandle) -> Result<vk::Semaphore> {
        let mut import_info = vk::ImportSemaphoreFdInfoKHR {
            s_type: vk::StructureType::IMPORT_SEMAPHORE_FD_INFO_KHR,
            p_next: std::ptr::null(),
            semaphore: vk::Semaphore::null(),
            flags: vk::SemaphoreImportFlags::empty(),
            handle_type: vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_FD,
            fd: handle.raw_handle as i32,
            _marker: std::marker::PhantomData,
        };

        // First create the semaphore
        let semaphore = self.create_exportable_semaphore()?;
        import_info.semaphore = semaphore;

        unsafe {
            self.external_semaphore_fd
                .import_semaphore_fd(&import_info)
                .map_err(|e| GeyserError::VulkanApiError(format!("Failed to import semaphore: {:?}", e)))?;
        }

        Ok(semaphore)
    }

    /// Create an exportable fence for CPU-side synchronization
    pub fn create_exportable_fence(&self) -> Result<vk::Fence> {
        let handle_types = {
            #[cfg(target_os = "linux")]
            { vk::ExternalFenceHandleTypeFlags::OPAQUE_FD }
            #[cfg(target_os = "windows")]
            { vk::ExternalFenceHandleTypeFlags::OPAQUE_WIN32 }
            #[cfg(not(any(target_os = "linux", target_os = "windows")))]
            { vk::ExternalFenceHandleTypeFlags::empty() }
        };

        let mut export_fence_info = vk::ExportFenceCreateInfo {
            s_type: vk::StructureType::EXPORT_FENCE_CREATE_INFO,
            p_next: std::ptr::null(),
            handle_types,
            _marker: std::marker::PhantomData,
        };

        let fence_create_info = vk::FenceCreateInfo {
            s_type: vk::StructureType::FENCE_CREATE_INFO,
            p_next: &mut export_fence_info as *mut _ as *const std::ffi::c_void,
            flags: vk::FenceCreateFlags::empty(),
            _marker: std::marker::PhantomData,
        };

        unsafe {
            self.device.create_fence(&fence_create_info, None)
                .map_err(|e| GeyserError::VulkanApiError(format!("Failed to create fence: {:?}", e)))
        }
    }

    /// Export a fence handle for sharing
    #[cfg(target_os = "windows")]
    pub fn export_fence_win32(&self, fence: vk::Fence) -> Result<VulkanFenceHandle> {
        let get_handle_info = vk::FenceGetWin32HandleInfoKHR {
            s_type: vk::StructureType::FENCE_GET_WIN32_HANDLE_INFO_KHR,
            p_next: std::ptr::null(),
            fence,
            handle_type: vk::ExternalFenceHandleTypeFlags::OPAQUE_WIN32,
            _marker: std::marker::PhantomData,
        };

        let raw_handle = unsafe {
            self.external_fence_win32
                .get_fence_win32_handle(&get_handle_info)
                .map(|h| h as u64)
                .map_err(|e| GeyserError::VulkanApiError(format!("Failed to export fence: {:?}", e)))?
        };

        self.exported_fences.lock().unwrap().insert(raw_handle, fence);

        Ok(VulkanFenceHandle {
            raw_handle,
            handle_type: vk::ExternalFenceHandleTypeFlags::OPAQUE_WIN32,
        })
    }

    #[cfg(target_os = "linux")]
    pub fn export_fence_fd(&self, fence: vk::Fence) -> Result<VulkanFenceHandle> {
        let get_fd_info = vk::FenceGetFdInfoKHR {
            s_type: vk::StructureType::FENCE_GET_FD_INFO_KHR,
            p_next: std::ptr::null(),
            fence,
            handle_type: vk::ExternalFenceHandleTypeFlags::OPAQUE_FD,
            _marker: std::marker::PhantomData,
        };

        let raw_handle = unsafe {
            self.external_fence_fd
                .get_fence_fd(&get_fd_info)
                .map(|fd| fd as u64)
                .map_err(|e| GeyserError::VulkanApiError(format!("Failed to export fence: {:?}", e)))?
        };

        self.exported_fences.lock().unwrap().insert(raw_handle, fence);

        Ok(VulkanFenceHandle {
            raw_handle,
            handle_type: vk::ExternalFenceHandleTypeFlags::OPAQUE_FD,
        })
    }

    /// Import a fence from an external handle
    #[cfg(target_os = "windows")]
    pub fn import_fence_win32(&self, handle: &VulkanFenceHandle) -> Result<vk::Fence> {
        let mut import_info = vk::ImportFenceWin32HandleInfoKHR {
            s_type: vk::StructureType::IMPORT_FENCE_WIN32_HANDLE_INFO_KHR,
            p_next: std::ptr::null(),
            fence: vk::Fence::null(),
            flags: vk::FenceImportFlags::empty(),
            handle_type: vk::ExternalFenceHandleTypeFlags::OPAQUE_WIN32,
            handle: handle.raw_handle as isize,
            name: std::ptr::null(),
            _marker: std::marker::PhantomData,
        };

        // First create the fence
        let fence = self.create_exportable_fence()?;
        import_info.fence = fence;

        unsafe {
            self.external_fence_win32
                .import_fence_win32_handle(&import_info)
                .map_err(|e| GeyserError::VulkanApiError(format!("Failed to import fence: {:?}", e)))?;
        }

        Ok(fence)
    }

    #[cfg(target_os = "linux")]
    pub fn import_fence_fd(&self, handle: &VulkanFenceHandle) -> Result<vk::Fence> {
        let mut import_info = vk::ImportFenceFdInfoKHR {
            s_type: vk::StructureType::IMPORT_FENCE_FD_INFO_KHR,
            p_next: std::ptr::null(),
            fence: vk::Fence::null(),
            flags: vk::FenceImportFlags::empty(),
            handle_type: vk::ExternalFenceHandleTypeFlags::OPAQUE_FD,
            fd: handle.raw_handle as i32,
            _marker: std::marker::PhantomData,
        };

        // First create the fence
        let fence = self.create_exportable_fence()?;
        import_info.fence = fence;

        unsafe {
            self.external_fence_fd
                .import_fence_fd(&import_info)
                .map_err(|e| GeyserError::VulkanApiError(format!("Failed to import fence: {:?}", e)))?;
        }

        Ok(fence)
    }

    /// Cleanup exported semaphore
    pub fn release_semaphore(&self, handle: &VulkanSemaphoreHandle) -> Result<()> {
        if let Some(semaphore) = self.exported_semaphores.lock().unwrap().remove(&handle.raw_handle) {
            unsafe {
                self.device.destroy_semaphore(semaphore, None);
            }
        }
        Ok(())
    }

    /// Cleanup exported fence
    pub fn release_fence(&self, handle: &VulkanFenceHandle) -> Result<()> {
        if let Some(fence) = self.exported_fences.lock().unwrap().remove(&handle.raw_handle) {
            unsafe {
                self.device.destroy_fence(fence, None);
            }
        }
        Ok(())
    }
}

impl TextureShareManager for VulkanTextureShareManager {
    fn create_shareable_texture(&self, descriptor: &TextureDescriptor) -> Result<Box<dyn SharedTexture>> {
        let vk_format = self.map_texture_format_to_vk(descriptor.format)?;
        let (vk_usage, _) = self.map_texture_usage_to_vk(&descriptor.usage);

        // Required for external memory export
        let handle_types = {
            #[cfg(target_os = "linux")]
            { vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD }
            #[cfg(target_os = "windows")]
            { vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32 }
            #[cfg(not(any(target_os = "linux", target_os = "windows")))]
            { vk::ExternalMemoryHandleTypeFlags::empty() }
        };

        let mut external_memory_create_info = vk::ExternalMemoryImageCreateInfo {
            s_type: vk::StructureType::EXTERNAL_MEMORY_IMAGE_CREATE_INFO,
            p_next: std::ptr::null(),
            handle_types,
            _marker: std::marker::PhantomData,
        };

        let image_create_info = vk::ImageCreateInfo {
            s_type: vk::StructureType::IMAGE_CREATE_INFO,
            p_next: &mut external_memory_create_info as *mut _ as *const std::ffi::c_void,
            flags: vk::ImageCreateFlags::empty(),
            image_type: vk::ImageType::TYPE_2D,
            format: vk_format,
            extent: vk::Extent3D {
                width: descriptor.width,
                height: descriptor.height,
                depth: 1,
            },
            mip_levels: 1,
            array_layers: 1,
            samples: vk::SampleCountFlags::TYPE_1,
            tiling: vk::ImageTiling::OPTIMAL,
            usage: vk_usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
            initial_layout: vk::ImageLayout::UNDEFINED,
            _marker: std::marker::PhantomData,
        };

        let image = unsafe { self.device.create_image(&image_create_info, None) }?;

        let requirements = unsafe { self.device.get_image_memory_requirements(image) };

        let allocation = self.allocator.lock().unwrap().allocate(&AllocationCreateDesc {
            name: descriptor.label.as_deref().unwrap_or("geyser-shared-texture"),
            requirements,
            location: MemoryLocation::GpuOnly, // Or appropriate location for sharing
            linear: false,
            allocation_scheme: AllocationScheme::DedicatedImage(image),
        })
        .map_err(|e| GeyserError::VulkanApiError(format!("Failed to allocate image memory: {}", e)))?;

        unsafe {
            self.device.bind_image_memory(image, allocation.memory(), allocation.offset())?;
        }

        Ok(Box::new(VulkanSharedTexture {
            device: self.device.clone(),
            allocation: Some(allocation),
            image,
            image_view: None, // Can be created later if needed
            descriptor: descriptor.clone(),
            exported_handle: None,
        }))
    }

    fn export_texture(&self, texture: &dyn SharedTexture) -> Result<ApiTextureHandle> {
        let vulkan_texture = texture
            .as_any()
            .downcast_ref::<VulkanSharedTexture>()
            .ok_or(GeyserError::Other("Provided texture is not a VulkanSharedTexture".to_string()))?;

        let allocation = vulkan_texture.allocation.as_ref()
            .ok_or(GeyserError::Other("Texture has no allocation to export".to_string()))?;

        let memory = unsafe { allocation.memory() };
        let memory_requirements = unsafe { self.device.get_image_memory_requirements(vulkan_texture.image) };

        // Export the external memory handle (platform-specific)
        #[cfg(target_os = "windows")]
        let raw_handle = self.get_external_memory_win32_info(memory)?;

        #[cfg(target_os = "linux")]
        let raw_handle = self.get_external_memory_fd_info(memory)? as u64;

        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        return Err(GeyserError::OperationNotSupported);

        // Query memory properties to get memory type index
        let memory_properties = unsafe {
            self.instance.get_physical_device_memory_properties(self.physical_device)
        };
        
        // Find memory type index for the allocation
        let memory_type_index = (0..memory_properties.memory_type_count)
            .find(|&i| {
                (memory_requirements.memory_type_bits & (1 << i)) != 0
            })
            .unwrap_or(0);

        let handle = VulkanTextureShareHandle {
            raw_handle,
            memory_type_index,
            size: memory_requirements.size,
            handle_type: {
                #[cfg(target_os = "windows")]
                { vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32 }
                #[cfg(target_os = "linux")]
                { vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD }
                #[cfg(not(any(target_os = "linux", target_os = "windows")))]
                { vk::ExternalMemoryHandleTypeFlags::empty() }
            },
            dedicated_allocation: true,
        };

        // Store the vk::DeviceMemory to ensure it stays alive
        self.exported_resources.lock().unwrap().insert(handle.raw_handle, memory);

        Ok(ApiTextureHandle::Vulkan(handle))
    }

    fn import_texture(&self, handle: ApiTextureHandle, descriptor: &TextureDescriptor) -> Result<Box<dyn SharedTexture>> {
        let vulkan_handle = match handle {
            ApiTextureHandle::Vulkan(h) => h,
            _ => return Err(GeyserError::InvalidTextureHandle),
        };

        let vk_format = self.map_texture_format_to_vk(descriptor.format)?;
        let (vk_usage, _) = self.map_texture_usage_to_vk(&descriptor.usage);

        // Create the image first with external memory info
        let mut external_memory_create_info = vk::ExternalMemoryImageCreateInfo {
            s_type: vk::StructureType::EXTERNAL_MEMORY_IMAGE_CREATE_INFO,
            p_next: std::ptr::null(),
            handle_types: vulkan_handle.handle_type,
            _marker: std::marker::PhantomData,
        };

        let image_create_info = vk::ImageCreateInfo {
            s_type: vk::StructureType::IMAGE_CREATE_INFO,
            p_next: &mut external_memory_create_info as *mut _ as *const std::ffi::c_void,
            flags: vk::ImageCreateFlags::empty(),
            image_type: vk::ImageType::TYPE_2D,
            format: vk_format,
            extent: vk::Extent3D {
                width: descriptor.width,
                height: descriptor.height,
                depth: 1,
            },
            mip_levels: 1,
            array_layers: 1,
            samples: vk::SampleCountFlags::TYPE_1,
            tiling: vk::ImageTiling::OPTIMAL,
            usage: vk_usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
            initial_layout: vk::ImageLayout::UNDEFINED,
            _marker: std::marker::PhantomData,
        };

        let image = unsafe { self.device.create_image(&image_create_info, None) }?;

        // Platform-specific import of external memory
        #[cfg(target_os = "windows")]
        let imported_memory = {
            let mut import_win32_info = vk::ImportMemoryWin32HandleInfoKHR {
                s_type: vk::StructureType::IMPORT_MEMORY_WIN32_HANDLE_INFO_KHR,
                p_next: std::ptr::null(),
                handle_type: vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32,
                handle: vulkan_handle.raw_handle as isize,
                name: std::ptr::null(),
                _marker: std::marker::PhantomData,
            };

            let mut dedicated_alloc_info = vk::MemoryDedicatedAllocateInfo {
                s_type: vk::StructureType::MEMORY_DEDICATED_ALLOCATE_INFO,
                p_next: &mut import_win32_info as *mut _ as *const std::ffi::c_void,
                image,
                buffer: vk::Buffer::null(),
                _marker: std::marker::PhantomData,
            };

            let alloc_info = vk::MemoryAllocateInfo {
                s_type: vk::StructureType::MEMORY_ALLOCATE_INFO,
                p_next: &mut dedicated_alloc_info as *mut _ as *const std::ffi::c_void,
                allocation_size: vulkan_handle.size,
                memory_type_index: vulkan_handle.memory_type_index,
                _marker: std::marker::PhantomData,
            };

            unsafe {
                self.device.allocate_memory(&alloc_info, None)
                    .map_err(|e| GeyserError::VulkanApiError(format!("Failed to import Win32 memory: {:?}", e)))?
            }
        };

        #[cfg(target_os = "linux")]
        let imported_memory = {
            let mut import_fd_info = vk::ImportMemoryFdInfoKHR {
                s_type: vk::StructureType::IMPORT_MEMORY_FD_INFO_KHR,
                p_next: std::ptr::null(),
                handle_type: vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD,
                fd: vulkan_handle.raw_handle as i32,
                _marker: std::marker::PhantomData,
            };

            let mut dedicated_alloc_info = vk::MemoryDedicatedAllocateInfo {
                s_type: vk::StructureType::MEMORY_DEDICATED_ALLOCATE_INFO,
                p_next: &mut import_fd_info as *mut _ as *const std::ffi::c_void,
                image,
                buffer: vk::Buffer::null(),
                _marker: std::marker::PhantomData,
            };

            let alloc_info = vk::MemoryAllocateInfo {
                s_type: vk::StructureType::MEMORY_ALLOCATE_INFO,
                p_next: &mut dedicated_alloc_info as *mut _ as *const std::ffi::c_void,
                allocation_size: vulkan_handle.size,
                memory_type_index: vulkan_handle.memory_type_index,
                _marker: std::marker::PhantomData,
            };

            unsafe {
                self.device.allocate_memory(&alloc_info, None)
                    .map_err(|e| GeyserError::VulkanApiError(format!("Failed to import FD memory: {:?}", e)))?
            }
        };

        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        return Err(GeyserError::OperationNotSupported);

        unsafe { self.device.bind_image_memory(image, imported_memory, 0)?; }

        // Store the imported memory to ensure its lifetime
        self.exported_resources.lock().unwrap().insert(vulkan_handle.raw_handle, imported_memory);

        Ok(Box::new(VulkanSharedTexture {
            device: self.device.clone(),
            allocation: None, // No allocation managed by `gpu_allocator` here, it's externally imported
            image,
            image_view: None,
            descriptor: descriptor.clone(),
            exported_handle: Some(vulkan_handle),
        }))
    }

    fn release_texture_handle(&self, handle: ApiTextureHandle) -> Result<()> {
        let raw_handle_key = match handle {
            ApiTextureHandle::Vulkan(h) => h.raw_handle,
            _ => return Err(GeyserError::InvalidTextureHandle),
        };

        if let Some(memory) = self.exported_resources.lock().unwrap().remove(&raw_handle_key) {
            unsafe {
                self.device.free_memory(memory, None);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
