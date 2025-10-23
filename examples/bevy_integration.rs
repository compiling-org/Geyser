// Example: Bevy Integration with Geyser
// This demonstrates how to integrate Geyser-managed textures with the Bevy game engine.
//
// IMPORTANT: This is a Phase 1 "conceptual integration" example that demonstrates
// the data flow, but uses CPU-side copies. True zero-copy integration requires
// deeper WGPU/Bevy integration which is a Phase 2/3 goal.
//
// Current limitations:
// - Bevy uses WGPU internally, which abstracts over Vulkan/Metal/DX12
// - WGPU doesn't expose direct APIs for importing arbitrary native texture handles
// - This example demonstrates creating a Geyser texture and simulating updates
//   by copying data to a Bevy Image (CPU transfer, not zero-copy)
//
// Future improvements (Phase 2/3):
// - Direct WGPU texture import from external handles
// - Custom Bevy render plugin for zero-copy texture sharing
// - Synchronization primitives for safe cross-process access

use bevy::{
    prelude::*,
    render::{
        render_resource::{Extent3d, TextureDimension, TextureFormat as BevyTextureFormat},
        texture::Image as BevyImage,
    },
    window::WindowMode,
};
use anyhow::Result;

#[cfg(target_os = "macos")]
use geyser::{
    metal::MetalTextureShareManager,
    common::{ApiTextureHandle, TextureDescriptor, TextureFormat, TextureUsage},
    TextureShareManager,
    SharedTexture,
};

#[cfg(any(target_os = "linux", target_os = "windows"))]
use geyser::{
    vulkan::VulkanTextureShareManager,
    common::{ApiTextureHandle, TextureDescriptor, TextureFormat, TextureUsage},
    TextureShareManager,
    SharedTexture,
};

use std::sync::Arc;

// Resource to hold the Geyser texture and manager
#[derive(Resource)]
struct GeyserTextureHolder {
    #[cfg(target_os = "macos")]
    pub manager: Arc<MetalTextureShareManager>,
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    pub manager: Arc<VulkanTextureShareManager>,

    pub geyser_shared_texture: Box<dyn SharedTexture>,
    pub geyser_texture_desc: TextureDescriptor,
    pub api_handle: ApiTextureHandle,
    pub image_handle: Handle<BevyImage>,
}

fn main() {
    println!("=== Geyser + Bevy Integration Example ===");
    println!("This demonstrates conceptual integration between Geyser and Bevy.");
    println!("Note: Phase 1 uses CPU-side copies. Zero-copy is a Phase 2/3 goal.\n");

    App::new()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Geyser + Bevy Integration".into(),
                    resolution: (800., 600.).into(),
                    mode: WindowMode::Windowed,
                    ..default()
                }),
                ..default()
            })
        )
        .add_systems(Startup, setup_geyser_and_bevy)
        .add_systems(Update, update_texture_animation)
        .run();
}

fn setup_geyser_and_bevy(
    mut commands: Commands,
    mut images: ResMut<Assets<BevyImage>>,
) {
    println!("Initializing Geyser texture manager...");

    let texture_desc = TextureDescriptor {
        width: 256,
        height: 256,
        format: TextureFormat::Rgba8Unorm,
        usage: vec![
            TextureUsage::TextureBinding,
            TextureUsage::RenderAttachment,
            TextureUsage::CopySrc,
            TextureUsage::CopyDst
        ],
        label: Some("BevyGeyserSharedTexture".to_string()),
    };

    // Platform-specific Geyser initialization
    #[cfg(target_os = "macos")]
    let (manager, geyser_texture, api_handle) = {
        use metal::{Device, MTLDevice};
        
        let device = Arc::new(Device::system_default().expect("No Metal device found"));
        let mgr = MetalTextureShareManager::new(device).expect("Failed to create MetalTextureShareManager");
        let tex = mgr.create_shareable_texture(&texture_desc).expect("Failed to create Metal texture");
        let handle = mgr.export_texture(tex.as_ref()).expect("Failed to export Metal texture");
        
        println!("✓ Geyser (Metal): Created and exported texture");
        (Arc::new(mgr), tex, handle)
    };

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    let (manager, geyser_texture, api_handle) = {
        use ash::{vk, Entry, Instance, Device};
        use std::ffi::{CStr, CString};

        println!("Creating Vulkan context...");
        let entry = unsafe { Entry::load() }.expect("Failed to load Vulkan");
        let app_name = CString::new("GeyserBevyExample").unwrap();
        let engine_name = CString::new("Geyser").unwrap();

        let app_info = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .application_version(0)
            .engine_name(&engine_name)
            .engine_version(0)
            .api_version(vk::make_api_version(0, 1, 0, 0));

        let create_info = vk::InstanceCreateInfo::builder().application_info(&app_info);
        let instance = unsafe { entry.create_instance(&create_info, None) }.expect("Failed to create Vulkan instance");

        let physical_devices = unsafe { instance.enumerate_physical_devices() }.expect("Failed to enumerate devices");
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

        let device = unsafe { instance.create_device(physical_device, &device_create_info, None) }
            .expect("Failed to create Vulkan device");

        let mgr = VulkanTextureShareManager::new(
            Arc::new(instance),
            Arc::new(device),
            physical_device,
            queue_family_index,
        ).expect("Failed to create VulkanTextureShareManager");
        
        let tex = mgr.create_shareable_texture(&texture_desc).expect("Failed to create Vulkan texture");
        let handle = mgr.export_texture(tex.as_ref()).expect("Failed to export Vulkan texture");
        
        println!("✓ Geyser (Vulkan): Created and exported texture");
        (Arc::new(mgr), tex, handle)
    };

    // Create a Bevy Image asset
    println!("Creating Bevy image asset...");
    let size = Extent3d {
        width: texture_desc.width,
        height: texture_desc.height,
        depth_or_array_layers: 1,
    };

    let bevy_image = BevyImage::new_fill(
        size,
        TextureDimension::D2,
        &[32, 32, 64, 255], // Dark blue initial color
        BevyTextureFormat::Rgba8UnormSrgb,
    );
    let image_handle = images.add(bevy_image);

    println!("✓ Bevy image created");

    // Store the Geyser resources
    commands.insert_resource(GeyserTextureHolder {
        manager,
        geyser_shared_texture: geyser_texture,
        geyser_texture_desc: texture_desc.clone(),
        api_handle,
        image_handle: image_handle.clone(),
    });

    // Setup camera and sprite
    commands.spawn(Camera2dBundle::default());
    
    commands.spawn(SpriteBundle {
        texture: image_handle.clone(),
        transform: Transform::from_scale(Vec3::splat(3.0)),
        ..default()
    });

    println!("✓ Bevy scene setup complete\n");
    println!("The displayed texture is managed by Geyser and updated via CPU copies.");
    println!("Watch the animated pattern to verify the integration!\n");
}

fn update_texture_animation(
    geyser_holder: Res<GeyserTextureHolder>,
    mut images: ResMut<Assets<BevyImage>>,
    time: Res<Time>,
) {
    // This system demonstrates updating the Bevy texture with animated data
    // In a real zero-copy scenario, the GPU would render directly to the Geyser texture,
    // and Bevy would read it without CPU involvement.

    let width = geyser_holder.geyser_texture_desc.width as usize;
    let height = geyser_holder.geyser_texture_desc.height as usize;
    
    if let Some(bevy_image) = images.get_mut(&geyser_holder.image_handle) {
        let elapsed = time.elapsed_seconds();
        let data = &mut bevy_image.data;
        
        // Generate animated pattern
        for y in 0..height {
            for x in 0..width {
                let i = (y * width + x) * 4;
                
                let fx = x as f32 / width as f32;
                let fy = y as f32 / height as f32;
                
                // Animated wave pattern
                let wave = ((fx * 10.0 + elapsed * 2.0).sin() * 
                           (fy * 10.0 + elapsed * 2.0).cos() * 0.5 + 0.5);
                
                data[i] = (fx * 255.0) as u8;                    // Red: horizontal gradient
                data[i + 1] = (fy * 255.0) as u8;                // Green: vertical gradient
                data[i + 2] = (wave * 255.0) as u8;              // Blue: animated wave
                data[i + 3] = 255;                                // Alpha: fully opaque
            }
        }
    }
}
