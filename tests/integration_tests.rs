// Integration tests for Geyser texture sharing

use geyser::{
    common::{TextureDescriptor, TextureFormat, TextureUsage},
    TextureShareManager,
};

#[cfg(feature = "vulkan")]
mod vulkan_tests {
    use super::*;
    use geyser::vulkan::VulkanTextureShareManager;
    use ash::{vk, Entry, Instance, Device};
    use std::{ffi::CString, sync::Arc};

    fn create_test_vulkan_context() -> (Arc<Instance>, Arc<Device>, vk::PhysicalDevice, u32) {
        let entry = unsafe { Entry::load().expect("Failed to load Vulkan") };
        let app_name = CString::new("GeyserTest").unwrap();
        
        let app_info = vk::ApplicationInfo {
            s_type: vk::StructureType::APPLICATION_INFO,
            p_next: std::ptr::null(),
            p_application_name: app_name.as_ptr(),
            application_version: 0,
            p_engine_name: std::ptr::null(),
            engine_version: 0,
            api_version: vk::make_api_version(0, 1, 2, 0),
            _marker: std::marker::PhantomData,
        };

        // Enable required extensions for external memory on Windows
        #[cfg(target_os = "windows")]
        let extension_names = [
            ash::khr::external_memory_capabilities::NAME.as_ptr(),
            ash::khr::get_physical_device_properties2::NAME.as_ptr(),
        ];
        
        #[cfg(not(target_os = "windows"))]
        let extension_names: [*const i8; 0] = [];

        let create_info = vk::InstanceCreateInfo {
            s_type: vk::StructureType::INSTANCE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::InstanceCreateFlags::empty(),
            p_application_info: &app_info,
            enabled_layer_count: 0,
            pp_enabled_layer_names: std::ptr::null(),
            enabled_extension_count: extension_names.len() as u32,
            pp_enabled_extension_names: extension_names.as_ptr(),
            _marker: std::marker::PhantomData,
        };
        
        let instance = unsafe { entry.create_instance(&create_info, None).expect("Failed to create instance") };

        let physical_devices = unsafe { instance.enumerate_physical_devices().expect("No devices") };
        let physical_device = physical_devices[0];

        let queue_family_properties = unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
        let queue_family_index = queue_family_properties
            .iter()
            .enumerate()
            .find_map(|(i, props)| {
                if props.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                    Some(i as u32)
                } else {
                    None
                }
            })
            .expect("No suitable queue family");

        let queue_priority = 1.0;
        let queue_create_info = vk::DeviceQueueCreateInfo {
            s_type: vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::DeviceQueueCreateFlags::empty(),
            queue_family_index,
            queue_count: 1,
            p_queue_priorities: &queue_priority,
            _marker: std::marker::PhantomData,
        };

        // Enable required device extensions for external memory
        #[cfg(target_os = "windows")]
        let device_extension_names = [
            ash::khr::external_memory::NAME.as_ptr(),
            ash::khr::external_memory_win32::NAME.as_ptr(),
        ];
        
        #[cfg(target_os = "linux")]
        let device_extension_names = [
            ash::khr::external_memory::NAME.as_ptr(),
            ash::khr::external_memory_fd::NAME.as_ptr(),
        ];
        
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        let device_extension_names: [*const i8; 0] = [];

        let device_create_info = vk::DeviceCreateInfo {
            s_type: vk::StructureType::DEVICE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::DeviceCreateFlags::empty(),
            queue_create_info_count: 1,
            p_queue_create_infos: &queue_create_info,
            enabled_layer_count: 0,
            pp_enabled_layer_names: std::ptr::null(),
            enabled_extension_count: device_extension_names.len() as u32,
            pp_enabled_extension_names: device_extension_names.as_ptr(),
            p_enabled_features: std::ptr::null(),
            _marker: std::marker::PhantomData,
        };

        let device = unsafe { instance.create_device(physical_device, &device_create_info, None).expect("Failed to create device") };

        (Arc::new(instance), Arc::new(device), physical_device, queue_family_index)
    }

    fn test_descriptor() -> TextureDescriptor {
        TextureDescriptor {
            width: 256,
            height: 256,
            format: TextureFormat::Rgba8Unorm,
            usage: vec![TextureUsage::TextureBinding],
            label: Some("TestTexture".to_string()),
        }
    }

    #[test]
    fn test_vulkan_manager_creation() {
        let (instance, device, physical_device, queue_family_index) = create_test_vulkan_context();
        let manager = VulkanTextureShareManager::new(instance, device, physical_device, queue_family_index);
        assert!(manager.is_ok());
    }

    #[test]
    fn test_vulkan_texture_creation() {
        let (instance, device, physical_device, queue_family_index) = create_test_vulkan_context();
        let manager = VulkanTextureShareManager::new(instance, device, physical_device, queue_family_index)
            .expect("Failed to create manager");
        
        let descriptor = test_descriptor();
        let texture = manager.create_shareable_texture(&descriptor);
        
        assert!(texture.is_ok());
        let texture = texture.unwrap();
        assert_eq!(texture.width(), 256);
        assert_eq!(texture.height(), 256);
        assert_eq!(texture.format(), TextureFormat::Rgba8Unorm);
    }

    #[test]
    fn test_vulkan_export() {
        let (instance, device, physical_device, queue_family_index) = create_test_vulkan_context();
        let manager = VulkanTextureShareManager::new(instance, device, physical_device, queue_family_index)
            .expect("Failed to create manager");
        
        let descriptor = test_descriptor();
        let texture = manager.create_shareable_texture(&descriptor).expect("Failed to create texture");
        
        let handle = manager.export_texture(texture.as_ref());
        assert!(handle.is_ok());
    }

    #[test]
    fn test_vulkan_format_mappings() {
        let (instance, device, physical_device, queue_family_index) = create_test_vulkan_context();
        let manager = VulkanTextureShareManager::new(instance, device, physical_device, queue_family_index)
            .expect("Failed to create manager");
        
        // Test various formats
        let formats = vec![
            TextureFormat::Rgba8Unorm,
            TextureFormat::Bgra8Unorm,
            TextureFormat::R16Float,
            TextureFormat::R32Float,
            TextureFormat::Rgba16Float,
            TextureFormat::Depth32Float,
        ];

        for format in formats {
            let desc = TextureDescriptor {
                width: 128,
                height: 128,
                format,
                usage: vec![TextureUsage::TextureBinding],
                label: Some(format!("Test{:?}", format)),
            };
            
            let result = manager.create_shareable_texture(&desc);
            assert!(result.is_ok(), "Failed to create texture with format {:?}", format);
        }
    }
}

#[cfg(feature = "metal")]
#[cfg(target_os = "macos")]
mod metal_tests {
    use super::*;
    use geyser::metal::MetalTextureShareManager;
    use metal::{Device, MTLDevice};
    use std::sync::Arc;

    fn test_descriptor() -> TextureDescriptor {
        TextureDescriptor {
            width: 256,
            height: 256,
            format: TextureFormat::Rgba8Unorm,
            usage: vec![TextureUsage::TextureBinding],
            label: Some("TestTexture".to_string()),
        }
    }

    #[test]
    fn test_metal_manager_creation() {
        let device = Arc::new(Device::system_default().expect("No Metal device"));
        let manager = MetalTextureShareManager::new(device);
        assert!(manager.is_ok());
    }

    #[test]
    fn test_metal_texture_creation() {
        let device = Arc::new(Device::system_default().expect("No Metal device"));
        let manager = MetalTextureShareManager::new(device).expect("Failed to create manager");
        
        let descriptor = test_descriptor();
        let texture = manager.create_shareable_texture(&descriptor);
        
        assert!(texture.is_ok());
        let texture = texture.unwrap();
        assert_eq!(texture.width(), 256);
        assert_eq!(texture.height(), 256);
        assert_eq!(texture.format(), TextureFormat::Rgba8Unorm);
    }

    #[test]
    fn test_metal_export_import() {
        let device = Arc::new(Device::system_default().expect("No Metal device"));
        let manager = MetalTextureShareManager::new(device).expect("Failed to create manager");
        
        let descriptor = test_descriptor();
        let texture = manager.create_shareable_texture(&descriptor).expect("Failed to create texture");
        
        // Export
        let handle = manager.export_texture(texture.as_ref()).expect("Failed to export");
        
        // Import
        let imported = manager.import_texture(handle.clone(), &descriptor);
        assert!(imported.is_ok());
        
        let imported = imported.unwrap();
        assert_eq!(imported.width(), 256);
        assert_eq!(imported.height(), 256);
        
        // Cleanup
        manager.release_texture_handle(handle).expect("Failed to release");
    }

    #[test]
    fn test_metal_format_mappings() {
        let device = Arc::new(Device::system_default().expect("No Metal device"));
        let manager = MetalTextureShareManager::new(device).expect("Failed to create manager");
        
        let formats = vec![
            TextureFormat::Rgba8Unorm,
            TextureFormat::Bgra8Unorm,
            TextureFormat::R16Float,
            TextureFormat::R32Float,
            TextureFormat::Rgba16Float,
            TextureFormat::Depth32Float,
        ];

        for format in formats {
            let desc = TextureDescriptor {
                width: 128,
                height: 128,
                format,
                usage: vec![TextureUsage::TextureBinding],
                label: Some(format!("Test{:?}", format)),
            };
            
            let result = manager.create_shareable_texture(&desc);
            assert!(result.is_ok(), "Failed to create texture with format {:?}", format);
        }
    }
}

// Common tests that don't require specific backends
#[test]
fn test_texture_descriptor_creation() {
    let desc = TextureDescriptor {
        width: 1920,
        height: 1080,
        format: TextureFormat::Rgba8Unorm,
        usage: vec![TextureUsage::RenderAttachment, TextureUsage::TextureBinding],
        label: Some("CommonTest".to_string()),
    };
    
    assert_eq!(desc.width, 1920);
    assert_eq!(desc.height, 1080);
    assert_eq!(desc.format, TextureFormat::Rgba8Unorm);
    assert_eq!(desc.usage.len(), 2);
}

#[test]
fn test_format_display() {
    use std::format;
    let format = TextureFormat::Rgba8Unorm;
    let display_str = format!("{}", format);
    assert!(!display_str.is_empty());
}
