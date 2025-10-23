// Timeline Semaphore IPC Producer
//
// This example demonstrates cross-process synchronization using timeline semaphores.
// Timeline semaphores provide better performance and flexibility than binary semaphores
// for multi-process scenarios.
//
// Run this FIRST, then run timeline_ipc_consumer in a separate terminal.

mod ipc_utils;

use geyser::{
    vulkan::VulkanTextureShareManager,
    common::{ApiTextureHandle, TextureDescriptor, TextureFormat, TextureUsage},
    TextureShareManager,
};
use ash::{vk, Entry, Instance, Device};
use std::{
    ffi::CString,
    sync::Arc,
    thread,
    time::Duration,
};
use anyhow::{Result, Context};
use ipc_utils::{IpcChannelPair, IpcMessage, format_to_string};

fn create_vulkan_context() -> Result<(Arc<Instance>, Arc<Device>, vk::PhysicalDevice, u32, vk::Queue)> {
    let entry = unsafe { Entry::load() }?;
    let app_name = CString::new("TimelineIPCProducer").unwrap();
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
    println!("║   Timeline Semaphore IPC Producer (Multi-Process)     ║");
    println!("╚════════════════════════════════════════════════════════╝\n");

    let channels = IpcChannelPair::producer();
    channels.clear_all()?;

    println!("[1/5] Initializing Vulkan context...");
    let (instance, device, physical_device, queue_family_index, _queue) = create_vulkan_context()?;
    let manager = VulkanTextureShareManager::new(
        instance.clone(),
        device.clone(),
        physical_device,
        queue_family_index,
    )?;
    println!("✓ Vulkan context initialized\n");

    println!("[2/5] Creating shareable texture...");
    let texture_desc = TextureDescriptor {
        width: 1024,
        height: 768,
        format: TextureFormat::Rgba8Unorm,
        usage: vec![
            TextureUsage::TextureBinding,
            TextureUsage::RenderAttachment,
            TextureUsage::CopySrc,
            TextureUsage::CopyDst,
        ],
        label: Some("TimelineIPCTexture".to_string()),
    };

    let texture = manager.create_shareable_texture(&texture_desc)?;
    let exported_handle = manager.export_texture(texture.as_ref())?;
    
    let (raw_handle, memory_type_index, size) = if let ApiTextureHandle::Vulkan(h) = &exported_handle {
        (h.raw_handle, h.memory_type_index, h.size)
    } else {
        anyhow::bail!("Expected Vulkan handle")
    };
    
    println!("✓ Texture created and exported\n");

    println!("[3/5] Creating timeline semaphore (initial value: 0)...");
    let initial_value = 0u64;
    let timeline_sem = manager.create_exportable_timeline_semaphore(initial_value)?;
    
    #[cfg(target_os = "windows")]
    let semaphore_handle = manager.export_timeline_semaphore_win32(timeline_sem)?;
    #[cfg(target_os = "linux")]
    let semaphore_handle = manager.export_timeline_semaphore_fd(timeline_sem)?;
    
    println!("✓ Timeline semaphore created and exported\n");

    println!("[4/5] Sending handles to consumer via IPC...");
    channels.send.send(&IpcMessage::TextureHandle {
        raw_handle,
        memory_type_index,
        size,
        width: texture_desc.width,
        height: texture_desc.height,
        format: format_to_string(texture_desc.format),
    })?;
    
    channels.send.send(&IpcMessage::SemaphoreHandle {
        raw_handle: semaphore_handle.raw_handle,
    })?;
    
    channels.send.send(&IpcMessage::ProducerReady)?;
    println!("✓ Handles sent\n");

    println!("Waiting for consumer to be ready...");
    let response = channels.receive.receive(30)?;
    match response {
        IpcMessage::ConsumerReady => println!("✓ Consumer is ready!\n"),
        _ => anyhow::bail!("Unexpected response from consumer"),
    }

    println!("[5/5] Rendering frames with timeline synchronization...");
    println!("(Timeline values increase with each frame)\\n");

    const NUM_FRAMES: u32 = 10;
    
    for frame_num in 0..NUM_FRAMES {
        let frame_value = (frame_num + 1) as u64;
        
        println!("  Frame {}: Rendering...", frame_num);
        thread::sleep(Duration::from_millis(500)); // Simulate GPU work
        
        // Signal timeline semaphore with frame number
        manager.signal_timeline_semaphore(timeline_sem, frame_value)?;
        println!("  Frame {}: Signaled timeline value {}", frame_num, frame_value);
        
        // Notify consumer
        channels.send.send(&IpcMessage::FrameReady { frame_number: frame_num })?;
        println!("  Frame {}: Notified consumer\n", frame_num);
        
        thread::sleep(Duration::from_millis(300));
    }

    println!("All frames rendered. Sending shutdown...");
    channels.send.send(&IpcMessage::Shutdown)?;
    thread::sleep(Duration::from_millis(1000));

    println!("\n╔════════════════════════════════════════════════════════╗");
    println!("║                  Producer Complete                     ║");
    println!("╚════════════════════════════════════════════════════════╝");
    
    println!("\nTimeline Semaphore Advantages:");
    println!("• Counter-based (not binary on/off)");
    println!("• Consumer can wait for specific frame values");
    println!("• No need to track max frames in flight");
    println!("• Better performance for pipelines\n");

    // Cleanup
    drop(texture);
    unsafe {
        device.destroy_semaphore(timeline_sem, None);
    }
    manager.release_texture_handle(exported_handle)?;
    channels.clear_all()?;

    Ok(())
}
