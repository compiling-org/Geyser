// Example: Share texture between two Metal contexts using IOSurface
// This demonstrates the basic workflow of creating a texture in one Metal context,
// exporting it via IOSurface, and importing it into another Metal context.

#[cfg(target_os = "macos")]
use geyser::{
    metal::MetalTextureShareManager,
    common::{TextureDescriptor, TextureFormat, TextureUsage},
    TextureShareManager,
    SharedTexture,
};
#[cfg(target_os = "macos")]
use metal::{Device, MTLDevice};
#[cfg(target_os = "macos")]
use std::sync::Arc;
#[cfg(target_os = "macos")]
use anyhow::Result;

#[cfg(target_os = "macos")]
fn main() -> Result<()> {
    println!("=== Geyser Metal to Metal Texture Sharing Example ===\n");

    // Context 1 (e.g., Application 1)
    println!("Creating Metal Context 1...");
    let device1 = Arc::new(Device::system_default().expect("No Metal device found"));
    let manager1 = MetalTextureShareManager::new(device1.clone())?;
    println!("✓ Context 1 created\n");

    // Create a shareable texture in Context 1
    let texture_desc = TextureDescriptor {
        width: 512,
        height: 512,
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

    // Export the handle (IOSurface ID)
    println!("App 1: Exporting texture handle (IOSurface)...");
    let exported_handle = manager1.export_texture(shareable_texture1.as_ref())?;
    println!("✓ Exported handle: {:?}\n", exported_handle);

    // Context 2 (e.g., Application 2, potentially a separate process)
    // For this example, we'll simulate it in the same process.
    println!("Creating Metal Context 2...");
    let device2 = Arc::new(Device::system_default().expect("No Metal device found"));
    let manager2 = MetalTextureShareManager::new(device2.clone())?;
    println!("✓ Context 2 created\n");

    // Import the texture handle into Context 2
    println!("App 2: Importing texture handle (IOSurface lookup)...");
    let imported_texture2 = manager2.import_texture(exported_handle.clone(), &texture_desc)?;
    println!("✓ Texture imported");
    println!("  - Width: {}", imported_texture2.width());
    println!("  - Height: {}", imported_texture2.height());
    println!("  - Format: {:?}\n", imported_texture2.format());

    // At this point, `imported_texture2` should be a valid MTLTexture in `device2`
    // that refers to the same GPU memory (IOSurface) as `shareable_texture1` in `device1`.

    // Clean up
    println!("App 2: Releasing imported texture handle...");
    manager2.release_texture_handle(exported_handle.clone())?;
    println!("✓ Released\n");

    println!("App 1: Dropping original shareable texture...");
    drop(shareable_texture1); // This will drop the texture and the IOSurface reference
    println!("✓ Dropped\n");

    println!("=== Example finished successfully ===");
    println!("\nNOTE: This example demonstrates IOSurface-based texture sharing on macOS.");
    println!("The IOSurface ID can be passed between processes for true cross-process sharing.");

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn main() -> Result<(), anyhow::Error> {
    println!("=== Metal to Metal Texture Sharing Example ===");
    println!("\nThis example is only available on macOS.");
    println!("Metal and IOSurface are macOS/iOS specific technologies.");
    Ok(())
}
