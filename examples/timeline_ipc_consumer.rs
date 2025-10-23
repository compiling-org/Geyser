// Timeline Semaphore IPC Consumer
//
// This example demonstrates receiving and waiting on timeline semaphores
// across process boundaries. Timeline semaphores provide better synchronization
// than binary semaphores for multi-process frame pipelining.
//
// Run timeline_ipc_producer FIRST, then run this in a separate terminal.

mod ipc_utils;

use geyser::{
    vulkan::{VulkanTextureShareManager, VulkanTextureShareHandle, VulkanSemaphoreHandle},
    common::{ApiTextureHandle, TextureDescriptor, TextureFormat, TextureUsage},
    TextureShareManager,
};
use ash::{vk, Entry, Instance, Device};
use std::{
    ffi::CString,
    sync::Arc,
};
use anyhow::{Result, Context};
use ipc_utils::{IpcChannelPair, IpcMessage, string_to_format};

fn create_vulkan_context() -> Result<(Arc<Instance>, Arc<Device>, vk::PhysicalDevice, u32, vk::Queue)> {
    let entry = unsafe { Entry::load() }?;
    let app_name = CString::new("TimelineIPCConsumer").unwrap();
    let engine_name = CString::new("Geyser").unwrap();

    let app_info = vk::ApplicationInfo {
        s_type: vk::StructureType::APPLICATION_INFO,
        p_next: std::ptr::null(),
        p_application_name: app_name.as_ptr(),
        application_version: 0,
        p_engine_name: engine_name.as_ptr(),
        engine_version: 0,
        api_version: vk::make_api_version(0, 1, 2, 0),
        ..Default::default()
    };

    let create_info = vk::InstanceCreateInfo {
        s_type: vk::StructureType::INSTANCE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: vk::InstanceCreateFlags::empty(),
        p_application_info: &app_info,
        enabled_layer_count: 0,
        pp_enabled_layer_names: std::ptr::null(),
        enabled_extension_count: 0,
        pp_enabled_extension_names: std::ptr::null(),
        ..Default::default()
    };

    let instance = unsafe { entry.create_instance(&create_info, None) }?;
    let physical_devices = unsafe { instance.enumerate_physical_devices() }?;
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
        .context("No suitable queue family found")?;

    let queue_priority = 1.0;
    let queue_priorities = [queue_priority];
    let queue_create_info = vk::DeviceQueueCreateInfo {
        s_type: vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: vk::DeviceQueueCreateFlags::empty(),
        queue_family_index,
        queue_count: 1,
        p_queue_priorities: queue_priorities.as_ptr(),
        ..Default::default()
    };

    let device_extensions = [
        #[cfg(target_os = "linux")]
        ash::khr::external_memory_fd::NAME.as_ptr(),
        #[cfg(target_os = "windows")]
        ash::khr::external_memory_win32::NAME.as_ptr(),
        ash::khr::external_memory::NAME.as_ptr(),
        #[cfg(target_os = "linux")]
        ash::khr::external_semaphore_fd::NAME.as_ptr(),
        #[cfg(target_os = "windows")]
        ash::khr::external_semaphore_win32::NAME.as_ptr(),
        ash::khr::external_semaphore::NAME.as_ptr(),
    ];

    let queue_create_infos = [queue_create_info];
    let device_create_info = vk::DeviceCreateInfo {
        s_type: vk::StructureType::DEVICE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: vk::DeviceCreateFlags::empty(),
        queue_create_info_count: queue_create_infos.len() as u32,
        p_queue_create_infos: queue_create_infos.as_ptr(),
        enabled_layer_count: 0,
        pp_enabled_layer_names: std::ptr::null(),
        enabled_extension_count: device_extensions.len() as u32,
        pp_enabled_extension_names: device_extensions.as_ptr(),
        p_enabled_features: std::ptr::null(),
        ..Default::default()
    };

    let device = unsafe { instance.create_device(physical_device, &device_create_info, None) }?;
    let queue = unsafe { device.get_device_queue(queue_family_index, 0) };

    Ok((Arc::new(instance), Arc::new(device), physical_device, queue_family_index, queue))
}

fn main() -> Result<()> {
    println!("╔════════════════════════════════════════════════════════╗");
    println!("║   Timeline Semaphore IPC Consumer (Multi-Process)     ║");
    println!("╚════════════════════════════════════════════════════════╝\n");

    let channels = IpcChannelPair::consumer();

    println!("[1/5] Initializing Vulkan context...");
    let (instance, device, physical_device, queue_family_index, _queue) = create_vulkan_context()?;
    let manager = VulkanTextureShareManager::new(
        instance.clone(),
        device.clone(),
        physical_device,
        queue_family_index,
    )?;
    println!("✓ Vulkan context initialized\n");

    println!("[2/5] Waiting for texture handle from producer...");
    let texture_message = channels.receive.receive(30)?;
    let (raw_handle, memory_type_index, size, width, height, format_str) = match texture_message {
        IpcMessage::TextureHandle {
            raw_handle,
            memory_type_index,
            size,
            width,
            height,
            format,
        } => (raw_handle, memory_type_index, size, width, height, format),
        _ => anyhow::bail!("Expected TextureHandle message"),
    };
    
    println!("✓ Received texture handle (0x{:X})\n", raw_handle);

    println!("[3/5] Waiting for timeline semaphore handle...");
    let semaphore_message = channels.receive.receive(5)?;
    let semaphore_raw_handle = match semaphore_message {
        IpcMessage::SemaphoreHandle { raw_handle } => raw_handle,
        _ => anyhow::bail!("Expected SemaphoreHandle message"),
    };
    
    println!("✓ Received timeline semaphore handle (0x{:X})\n", semaphore_raw_handle);

    println!("[4/5] Importing shared resources...");
    
    // Import texture
    let texture_handle = VulkanTextureShareHandle {
        raw_handle,
        memory_type_index,
        size,
        handle_type: {
            #[cfg(target_os = "windows")]
            { vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32 }
            #[cfg(target_os = "linux")]
            { vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD }
        },
        dedicated_allocation: true,
    };
    
    let format = string_to_format(&format_str)
        .map_err(|e| anyhow::anyhow!("Failed to parse format: {}", e))?;
    
    let texture_desc = TextureDescriptor {
        width,
        height,
        format,
        usage: vec![
            TextureUsage::TextureBinding,
            TextureUsage::RenderAttachment,
            TextureUsage::CopySrc,
            TextureUsage::CopyDst,
        ],
        label: Some("ImportedTimelineTexture".to_string()),
    };
    
    let imported_texture = manager.import_texture(
        ApiTextureHandle::Vulkan(texture_handle.clone()),
        &texture_desc,
    )?;
    
    println!("✓ Texture imported ({} x {})", imported_texture.width(), imported_texture.height());

    // Import timeline semaphore
    let semaphore_handle = VulkanSemaphoreHandle {
        raw_handle: semaphore_raw_handle,
        handle_type: {
            #[cfg(target_os = "windows")]
            { vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_WIN32 }
            #[cfg(target_os = "linux")]
            { vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_FD }
        },
        is_timeline: true,
    };
    
    #[cfg(target_os = "windows")]
    let imported_semaphore = manager.import_timeline_semaphore_win32(&semaphore_handle, 0)?;
    #[cfg(target_os = "linux")]
    let imported_semaphore = manager.import_timeline_semaphore_fd(&semaphore_handle, 0)?;
    
    println!("✓ Timeline semaphore imported\n");

    // Wait for producer ready
    let ready_msg = channels.receive.receive(5)?;
    match ready_msg {
        IpcMessage::ProducerReady => println!("✓ Producer is ready"),
        _ => anyhow::bail!("Expected ProducerReady message"),
    }
    
    channels.send.send(&IpcMessage::ConsumerReady)?;
    println!("✓ Signaled ready to producer\n");

    println!("[5/5] Processing frames with timeline synchronization...\n");
    
    let mut frames_processed = 0;
    loop {
        let message = channels.receive.receive(60)?;
        
        match message {
            IpcMessage::FrameReady { frame_number } => {
                let expected_value = (frame_number + 1) as u64;
                
                println!("  Frame {}: Waiting for timeline value {}...", frame_number, expected_value);
                
                // Wait for the specific frame's timeline value
                manager.wait_timeline_semaphore(imported_semaphore, expected_value, u64::MAX)?;
                
                // Query current value to verify
                let current_value = manager.get_timeline_semaphore_value(imported_semaphore)?;
                println!("  Frame {}: Complete! (current value: {})", frame_number, current_value);
                
                // In real app: use texture here for display/processing
                println!("  Frame {}: Processing shared texture...\n", frame_number);
                
                frames_processed += 1;
            }
            IpcMessage::Shutdown => {
                println!("Received shutdown signal from producer");
                break;
            }
            _ => {
                println!("Warning: Unexpected message type");
            }
        }
    }

    println!("\n╔════════════════════════════════════════════════════════╗");
    println!("║                  Consumer Complete                     ║");
    println!("╚════════════════════════════════════════════════════════╝");
    
    println!("\nFrames processed: {}", frames_processed);
    println!("Timeline semaphore handle: 0x{:X}", semaphore_raw_handle);
    
    println!("\nTimeline Semaphore Benefits:");
    println!("• Wait for specific frame values (not just on/off)");
    println!("• Query current value to check progress");
    println!("• No need to manually track frame state");
    println!("• Better pipelining with less synchronization overhead\n");

    // Cleanup
    drop(imported_texture);
    unsafe {
        device.destroy_semaphore(imported_semaphore, None);
    }
    manager.release_texture_handle(ApiTextureHandle::Vulkan(texture_handle))?;
    channels.clear_all()?;

    Ok(())
}
