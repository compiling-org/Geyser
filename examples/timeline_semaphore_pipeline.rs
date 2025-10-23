// Timeline Semaphore Example: Frame Pipelining
//
// This example demonstrates using timeline semaphores for efficient frame pipelining.
// Unlike binary semaphores which require complex state management, timeline semaphores
// use monotonically increasing counter values for synchronization.
//
// Key advantages:
// - No need to track "in-flight" frame count
// - Can wait for any specific frame completion
// - Multiple waits on same semaphore at different values
// - Simplified producer-consumer patterns

use geyser::{
    vulkan::VulkanTextureShareManager,
    common::{TextureDescriptor, TextureFormat, TextureUsage},
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

fn create_vulkan_context() -> Result<(Arc<Instance>, Arc<Device>, vk::PhysicalDevice, u32, vk::Queue)> {
    let entry = unsafe { Entry::load() }?;
    let app_name = CString::new("TimelineSemaphoreExample").unwrap();
    let engine_name = CString::new("Geyser").unwrap();

    let app_info = vk::ApplicationInfo {
        s_type: vk::StructureType::APPLICATION_INFO,
        p_next: std::ptr::null(),
        p_application_name: app_name.as_ptr(),
        application_version: 0,
        p_engine_name: engine_name.as_ptr(),
        engine_version: 0,
        api_version: vk::make_api_version(0, 1, 2, 0), // Vulkan 1.2 for timeline semaphores
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
    println!("║  Timeline Semaphore Frame Pipelining Example          ║");
    println!("╚════════════════════════════════════════════════════════╝\n");

    println!("[1/4] Initializing Vulkan context...");
    let (instance, device, physical_device, queue_family_index, _queue) = create_vulkan_context()?;
    let manager = VulkanTextureShareManager::new(
        instance.clone(),
        device.clone(),
        physical_device,
        queue_family_index,
    )?;
    println!("✓ Vulkan context initialized\n");

    println!("[2/4] Creating timeline semaphore...");
    let initial_value = 0u64;
    let timeline_sem = manager.create_exportable_timeline_semaphore(initial_value)?;
    println!("✓ Timeline semaphore created with initial value: {}\n", initial_value);

    println!("[3/4] Simulating frame pipeline...");
    println!("(Timeline semaphores allow unlimited frames in flight)\n");

    const NUM_FRAMES: usize = 10;
    let mut frame_values: Vec<u64> = Vec::with_capacity(NUM_FRAMES);

    // Producer: Submit frames as fast as possible
    println!("Producer: Submitting {} frames...", NUM_FRAMES);
    for frame_id in 0..NUM_FRAMES {
        let signal_value = (frame_id + 1) as u64;
        frame_values.push(signal_value);
        
        // In real code, you'd submit GPU work here
        // For this example, we signal from the host
        manager.signal_timeline_semaphore(timeline_sem, signal_value)?;
        
        println!("  Frame {}: Signaled timeline value {}", frame_id, signal_value);
        thread::sleep(Duration::from_millis(100)); // Simulate work
    }
    println!("✓ All frames submitted\n");

    // Consumer: Wait for specific frames (can be out of order!)
    println!("[4/4] Consumer: Waiting for frames (demonstrating flexibility)...");
    
    // Wait for frame 5 specifically
    println!("  Waiting for frame 5 (value {})...", frame_values[5]);
    manager.wait_timeline_semaphore(timeline_sem, frame_values[5], u64::MAX)?;
    println!("  ✓ Frame 5 complete!");
    
    // Wait for frame 3 (out of order)
    println!("  Waiting for frame 3 (value {})...", frame_values[3]);
    manager.wait_timeline_semaphore(timeline_sem, frame_values[3], u64::MAX)?;
    println!("  ✓ Frame 3 complete!");
    
    // Query current value
    let current_value = manager.get_timeline_semaphore_value(timeline_sem)?;
    println!("  Current semaphore value: {}", current_value);
    
    // Wait for all frames
    let final_value = *frame_values.last().unwrap();
    println!("  Waiting for all frames (value {})...", final_value);
    manager.wait_timeline_semaphore(timeline_sem, final_value, u64::MAX)?;
    println!("  ✓ All frames complete!\n");

    println!("╔════════════════════════════════════════════════════════╗");
    println!("║                   Example Complete                     ║");
    println!("╚════════════════════════════════════════════════════════╝");
    
    println!("\nKey Takeaways:");
    println!("• Timeline semaphores use monotonically increasing counters");
    println!("• No need to track \"max frames in flight\"");
    println!("• Can wait for any specific frame at any time");
    println!("• Simplified dependency tracking");
    println!("• Better for complex multi-stage pipelines\n");

    // Cleanup
    unsafe {
        device.destroy_semaphore(timeline_sem, None);
    }

    Ok(())
}
