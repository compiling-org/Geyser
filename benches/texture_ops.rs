// Benchmark suite for Geyser texture operations
// Run with: cargo bench --features vulkan

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use geyser::{
    common::{TextureDescriptor, TextureFormat, TextureUsage},
    TextureShareManager,
};

#[cfg(feature = "vulkan")]
use geyser::vulkan::VulkanTextureShareManager;
#[cfg(feature = "vulkan")]
use ash::{vk, Entry, Instance, Device};
#[cfg(feature = "vulkan")]
use std::{ffi::CString, sync::Arc};

#[cfg(feature = "vulkan")]
fn create_vulkan_context() -> (Arc<Instance>, Arc<Device>, vk::PhysicalDevice, u32) {
    let entry = unsafe { Entry::load().expect("Failed to load Vulkan") };
    let app_name = CString::new("GeyserBench").unwrap();
    
    let app_info = vk::ApplicationInfo::builder()
        .application_name(&app_name)
        .api_version(vk::make_api_version(0, 1, 0, 0));

    let create_info = vk::InstanceCreateInfo::builder().application_info(&app_info);
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
    let queue_create_info = vk::DeviceQueueCreateInfo::builder()
        .queue_family_index(queue_family_index)
        .queue_priorities(&[queue_priority]);

    let device_create_info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(&[*queue_create_info]);

    let device = unsafe { instance.create_device(physical_device, &device_create_info, None).expect("Failed to create device") };

    (Arc::new(instance), Arc::new(device), physical_device, queue_family_index)
}

#[cfg(feature = "vulkan")]
fn bench_texture_creation(c: &mut Criterion) {
    let (instance, device, physical_device, queue_family_index) = create_vulkan_context();
    let manager = VulkanTextureShareManager::new(instance, device, physical_device, queue_family_index)
        .expect("Failed to create manager");
    
    let mut group = c.benchmark_group("texture_creation");
    
    // Benchmark different texture sizes
    for size in [256, 512, 1024, 2048].iter() {
        let descriptor = TextureDescriptor {
            width: *size,
            height: *size,
            format: TextureFormat::Rgba8Unorm,
            usage: vec![TextureUsage::TextureBinding],
            label: Some(format!("Bench{}x{}", size, size)),
        };
        
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let texture = manager.create_shareable_texture(&descriptor)
                    .expect("Failed to create texture");
                black_box(texture);
            });
        });
    }
    
    group.finish();
}

#[cfg(feature = "vulkan")]
fn bench_texture_export(c: &mut Criterion) {
    let (instance, device, physical_device, queue_family_index) = create_vulkan_context();
    let manager = VulkanTextureShareManager::new(instance, device, physical_device, queue_family_index)
        .expect("Failed to create manager");
    
    let descriptor = TextureDescriptor {
        width: 1024,
        height: 1024,
        format: TextureFormat::Rgba8Unorm,
        usage: vec![TextureUsage::TextureBinding],
        label: Some("BenchExport".to_string()),
    };
    
    let texture = manager.create_shareable_texture(&descriptor)
        .expect("Failed to create texture");
    
    c.bench_function("texture_export", |b| {
        b.iter(|| {
            let handle = manager.export_texture(texture.as_ref())
                .expect("Failed to export");
            black_box(handle);
        });
    });
}

#[cfg(feature = "vulkan")]
fn bench_texture_formats(c: &mut Criterion) {
    let (instance, device, physical_device, queue_family_index) = create_vulkan_context();
    let manager = VulkanTextureShareManager::new(instance, device, physical_device, queue_family_index)
        .expect("Failed to create manager");
    
    let mut group = c.benchmark_group("texture_formats");
    
    let formats = vec![
        ("RGBA8", TextureFormat::Rgba8Unorm),
        ("RGBA16F", TextureFormat::Rgba16Float),
        ("RGBA32F", TextureFormat::Rgba32Float),
        ("Depth32F", TextureFormat::Depth32Float),
    ];
    
    for (name, format) in formats {
        let descriptor = TextureDescriptor {
            width: 1024,
            height: 1024,
            format,
            usage: vec![TextureUsage::TextureBinding],
            label: Some(name.to_string()),
        };
        
        group.bench_with_input(BenchmarkId::from_parameter(name), &descriptor, |b, desc| {
            b.iter(|| {
                let texture = manager.create_shareable_texture(desc)
                    .expect("Failed to create texture");
                black_box(texture);
            });
        });
    }
    
    group.finish();
}

#[cfg(feature = "vulkan")]
fn bench_export_import_roundtrip(c: &mut Criterion) {
    let (instance1, device1, physical_device1, queue_family_index1) = create_vulkan_context();
    let manager1 = VulkanTextureShareManager::new(instance1, device1, physical_device1, queue_family_index1)
        .expect("Failed to create manager1");
    
    let (instance2, device2, physical_device2, queue_family_index2) = create_vulkan_context();
    let manager2 = VulkanTextureShareManager::new(instance2, device2, physical_device2, queue_family_index2)
        .expect("Failed to create manager2");
    
    let descriptor = TextureDescriptor {
        width: 1024,
        height: 1024,
        format: TextureFormat::Rgba8Unorm,
        usage: vec![TextureUsage::TextureBinding],
        label: Some("BenchRoundtrip".to_string()),
    };
    
    c.bench_function("export_import_roundtrip", |b| {
        b.iter(|| {
            // Create
            let texture = manager1.create_shareable_texture(&descriptor)
                .expect("Failed to create");
            
            // Export
            let handle = manager1.export_texture(texture.as_ref())
                .expect("Failed to export");
            
            // Import
            let imported = manager2.import_texture(handle.clone(), &descriptor)
                .expect("Failed to import");
            
            // Cleanup
            manager2.release_texture_handle(handle)
                .expect("Failed to release");
            
            black_box(imported);
        });
    });
}

#[cfg(not(feature = "vulkan"))]
fn bench_dummy(_c: &mut Criterion) {
    // Placeholder when no backends are enabled
    println!("No backend enabled. Run with --features vulkan or --features metal");
}

#[cfg(feature = "vulkan")]
criterion_group!(
    benches,
    bench_texture_creation,
    bench_texture_export,
    bench_texture_formats,
    bench_export_import_roundtrip
);

#[cfg(not(feature = "vulkan"))]
criterion_group!(benches, bench_dummy);

criterion_main!(benches);
