// Example: Share texture between two Vulkan contexts
// This demonstrates the basic workflow of creating a texture in one context,
// exporting it, and importing it into another context.

use geyser::{
    vulkan::VulkanTextureShareManager,
    common::{TextureDescriptor, TextureFormat, TextureUsage},
    TextureShareManager,
    SharedTexture,
};
use ash::{
    vk,
    Entry,
    Instance,
    Device,
};
use std::{
    ffi::{CStr, CString},
    sync::Arc,
};
use anyhow::{Result, Context};

// Helper function to create a basic Vulkan setup (Instance, Device, Allocator)
// For a real app, this would be more robust.
fn create_vulkan_context() -> Result<(Arc<Instance>, Arc<Device>, vk::PhysicalDevice, u32, vk::Queue)> {
    let entry = unsafe { Entry::load() }?;
    let app_name = CString::new("GeyserVulkanExample").unwrap();
    let engine_name = CString::new("Geyser").unwrap();

    let app_info = vk::ApplicationInfo::builder()
        .application_name(&app_name)
        .application_version(0)
        .engine_name(&engine_name)
        .engine_version(0)
        .api_version(vk::make_api_version(0, 1, 0, 0));

    let create_info = vk::InstanceCreateInfo::builder().application_info(&app_info);
    let instance = unsafe { entry.create_instance(&create_info, None) }?;

    let physical_devices = unsafe { instance.enumerate_physical_devices() }?;
    let physical_device = physical_devices[0]; // Just pick the first one

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
        .context("No suitable queue family found")?;

    let queue_priority = 1.0;
    let queue_create_info = vk::DeviceQueueCreateInfo::builder()
        .queue_family_index(queue_family_index)
        .queue_priorities(&[queue_priority]);

    // Enable external memory extensions
    let device_extensions = [
        #[cfg(target_os = "linux")]
        ash::extensions::khr::ExternalMemoryFd::name().as_ptr(),
        #[cfg(target_os = "windows")]
        ash::extensions::khr::ExternalMemoryWin32::name().as_ptr(),
        ash::extensions::khr::ExternalMemory::name().as_ptr(),
    ];

    let device_create_info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(&[*queue_create_info])
        .enabled_extension_names(&device_extensions);

    let device = unsafe { instance.create_device(physical_device, &device_create_info, None) }?;
    let queue = unsafe { device.get_device_queue(queue_family_index, 0) };

    Ok((Arc::new(instance), Arc::new(device), physical_device, queue_family_index, queue))
}

fn main() -> Result<()> {
    println!("=== Geyser Vulkan to Vulkan Texture Sharing Example ===\n");

    // Context 1 (e.g., Application 1)
    println!("Creating Vulkan Context 1...");
    let (instance1, device1, physical_device1, queue_family_index1, _queue1) = create_vulkan_context()?;
    let manager1 = VulkanTextureShareManager::new(
        instance1.clone(),
        device1.clone(),
        physical_device1,
        queue_family_index1,
    )?;
    println!("✓ Context 1 created\n");

    // Create a shareable texture in Context 1
    let texture_desc = TextureDescriptor {
        width: 1024,
        height: 768,
        format: TextureFormat::Rgba8Unorm,
        usage: vec![
            TextureUsage::TextureBinding,
            TextureUsage::RenderAttachment,
            TextureUsage::CopySrc,
            TextureUsage::CopyDst
        ],
        label: Some("SharedTextureFromApp1".to_string()),
    };

    println!("App 1: Creating shareable texture...");
    let shareable_texture1 = manager1.create_shareable_texture(&texture_desc)?;
    println!("✓ Texture created");
    println!("  - Width: {}", shareable_texture1.width());
    println!("  - Height: {}", shareable_texture1.height());
    println!("  - Format: {:?}\n", shareable_texture1.format());

    // Export the handle
    println!("App 1: Exporting texture handle...");
    let exported_handle = manager1.export_texture(shareable_texture1.as_ref())?;
    println!("✓ Exported handle: {:?}\n", exported_handle);

    // Context 2 (e.g., Application 2, potentially a separate process)
    // For this example, we'll simulate it in the same process.
    println!("Creating Vulkan Context 2...");
    let (instance2, device2, physical_device2, queue_family_index2, _queue2) = create_vulkan_context()?;
    let manager2 = VulkanTextureShareManager::new(
        instance2.clone(),
        device2.clone(),
        physical_device2,
        queue_family_index2,
    )?;
    println!("✓ Context 2 created\n");

    // Import the texture handle into Context 2
    println!("App 2: Importing texture handle...");
    let imported_texture2 = manager2.import_texture(exported_handle.clone(), &texture_desc)?;
    println!("✓ Texture imported");
    println!("  - Width: {}", imported_texture2.width());
    println!("  - Height: {}", imported_texture2.height());
    println!("  - Format: {:?}\n", imported_texture2.format());

    // At this point, `imported_texture2` should be a valid Vulkan image in `device2`
    // that refers to the same GPU memory as `shareable_texture1` in `device1`.

    // Clean up
    println!("App 2: Releasing imported texture handle...");
    manager2.release_texture_handle(exported_handle.clone())?;
    println!("✓ Released\n");

    println!("App 1: Dropping original shareable texture...");
    drop(shareable_texture1); // This will drop the texture and potentially its allocation
    println!("✓ Dropped\n");

    println!("=== Example finished successfully ===");
    println!("\nNOTE: This example uses placeholder external memory handles.");
    println!("For real cross-process sharing, platform-specific external memory");
    println!("extensions (VK_KHR_external_memory_fd/win32) need to be fully implemented.");

    Ok(())
}
