//! Metal backend for texture sharing.

use std::{collections::HashMap, sync::Arc, any::Any};
use crate::{
    common::{ApiTextureHandle, TextureDescriptor, TextureFormat, TextureUsage},
    error::{GeyserError, Result},
    SharedTexture, TextureShareManager,
};
use core_graphics::display_link::CVTimeStamp;
use core_graphics::base::CGFloat;
use core_graphics::surface::{IOSurface, IOSurfaceProperties};
use metal::{
    MTLDevice, MTLTexture, MTLTextureDescriptor, MTLStorageMode, 
    MTLTextureUsage, MTLPixelFormat, MTLSharedEvent, MTLSharedEventListener,
};

/// Metal-specific texture share handle.
/// This will typically contain an IOSurface ID for sharing between processes.
#[derive(Debug, Clone)]
pub struct MetalTextureShareHandle {
    pub io_surface_id: u32,
}

/// Metal event handle for synchronization.
#[derive(Debug, Clone)]
pub struct MetalEventHandle {
    pub shared_event_id: u64,
}

pub struct MetalSharedTexture {
    device: Arc<MTLDevice>,
    texture: MTLTexture,
    io_surface: Option<IOSurface>,
    descriptor: TextureDescriptor,
    pub(crate) exported_handle: Option<MetalTextureShareHandle>,
}

impl SharedTexture for MetalSharedTexture {
    fn width(&self) -> u32 { self.descriptor.width }
    fn height(&self) -> u32 { self.descriptor.height }
    fn format(&self) -> TextureFormat { self.descriptor.format }
    fn usage(&self) -> &[TextureUsage] { &self.descriptor.usage }
    fn as_any(&self) -> &dyn Any { self }
}

pub struct MetalTextureShareManager {
    device: Arc<MTLDevice>,
    exported_surfaces: std::sync::Mutex<HashMap<u32, IOSurface>>,
    exported_events: std::sync::Mutex<HashMap<u64, MTLSharedEvent>>,
}

impl MetalTextureShareManager {
    pub fn new(device: Arc<MTLDevice>) -> Result<Self> {
        Ok(Self {
            device,
            exported_surfaces: std::sync::Mutex::new(HashMap::new()),
            exported_events: std::sync::Mutex::new(HashMap::new()),
        })
    }

    fn map_texture_format_to_mtl(&self, format: TextureFormat) -> Result<MTLPixelFormat> {
        match format {
            // 8-bit formats
            TextureFormat::Rgba8Unorm => Ok(MTLPixelFormat::RGBA8Unorm),
            TextureFormat::Bgra8Unorm => Ok(MTLPixelFormat::BGRA8Unorm),
            TextureFormat::Rgba8Srgb => Ok(MTLPixelFormat::RGBA8Unorm_sRGB),
            TextureFormat::Bgra8Srgb => Ok(MTLPixelFormat::BGRA8Unorm_sRGB),
            TextureFormat::R8Unorm => Ok(MTLPixelFormat::R8Unorm),
            TextureFormat::Rg8Unorm => Ok(MTLPixelFormat::RG8Unorm),
            
            // 16-bit formats
            TextureFormat::R16Float => Ok(MTLPixelFormat::R16Float),
            TextureFormat::Rg16Float => Ok(MTLPixelFormat::RG16Float),
            TextureFormat::Rgba16Float => Ok(MTLPixelFormat::RGBA16Float),
            TextureFormat::R16Uint => Ok(MTLPixelFormat::R16Uint),
            TextureFormat::R16Sint => Ok(MTLPixelFormat::R16Sint),
            
            // 32-bit formats
            TextureFormat::R32Float => Ok(MTLPixelFormat::R32Float),
            TextureFormat::Rg32Float => Ok(MTLPixelFormat::RG32Float),
            TextureFormat::Rgba32Float => Ok(MTLPixelFormat::RGBA32Float),
            TextureFormat::R32Uint => Ok(MTLPixelFormat::R32Uint),
            TextureFormat::R32Sint => Ok(MTLPixelFormat::R32Sint),
            
            // Depth/Stencil formats
            TextureFormat::Depth32Float => Ok(MTLPixelFormat::Depth32Float),
            TextureFormat::Depth24Plus => Ok(MTLPixelFormat::Depth32Float), // Metal doesn't have exact 24-bit depth
            TextureFormat::Depth24PlusStencil8 => Ok(MTLPixelFormat::Depth32Float_Stencil8),
            
            // HDR formats
            TextureFormat::Rgb10a2Unorm => Ok(MTLPixelFormat::RGB10A2Unorm),
            TextureFormat::Rg11b10Float => Ok(MTLPixelFormat::RG11B10Float),
        }
    }
    
    /// Calculate bytes per element for a given format.
    /// Used for IOSurface configuration.
    fn bytes_per_element(&self, format: TextureFormat) -> usize {
        match format {
            // 8-bit formats (1-4 bytes)
            TextureFormat::R8Unorm => 1,
            TextureFormat::Rg8Unorm => 2,
            TextureFormat::Rgba8Unorm | TextureFormat::Bgra8Unorm |
            TextureFormat::Rgba8Srgb | TextureFormat::Bgra8Srgb => 4,
            
            // 16-bit formats (2-8 bytes)
            TextureFormat::R16Float | TextureFormat::R16Uint | TextureFormat::R16Sint => 2,
            TextureFormat::Rg16Float => 4,
            TextureFormat::Rgba16Float => 8,
            
            // 32-bit formats (4-16 bytes)
            TextureFormat::R32Float | TextureFormat::R32Uint | TextureFormat::R32Sint => 4,
            TextureFormat::Rg32Float => 8,
            TextureFormat::Rgba32Float => 16,
            
            // Depth/Stencil formats
            TextureFormat::Depth32Float => 4,
            TextureFormat::Depth24Plus => 4,
            TextureFormat::Depth24PlusStencil8 => 8,
            
            // HDR formats
            TextureFormat::Rgb10a2Unorm => 4,
            TextureFormat::Rg11b10Float => 4,
        }
    }

    fn map_texture_usage_to_mtl(&self, usages: &[TextureUsage]) -> MTLTextureUsage {
        let mut mtl_usage = MTLTextureUsage::empty();
        for usage in usages {
            match usage {
                TextureUsage::CopySrc => mtl_usage |= MTLTextureUsage::ShaderRead,
                TextureUsage::CopyDst => mtl_usage |= MTLTextureUsage::ShaderWrite,
                TextureUsage::TextureBinding => mtl_usage |= MTLTextureUsage::ShaderRead,
                TextureUsage::RenderAttachment => mtl_usage |= MTLTextureUsage::RenderTarget,
                TextureUsage::StorageBinding => mtl_usage |= MTLTextureUsage::ShaderWrite,
            }
        }
        mtl_usage
    }

    // --- Synchronization Primitive Methods ---

    /// Create a shared event for cross-process synchronization
    pub fn create_shared_event(&self) -> Result<MTLSharedEvent> {
        self.device.new_shared_event()
            .ok_or(GeyserError::MetalApiError("Failed to create shared event".to_string()))
    }

    /// Export a shared event handle
    pub fn export_shared_event(&self, event: &MTLSharedEvent) -> Result<MetalEventHandle> {
        // MTLSharedEvent can be identified by its shared event handle
        // This is a platform-specific identifier that can be shared between processes
        let shared_event_id = event.shared_event_handle() as u64;

        // Store the event to keep it alive
        self.exported_events.lock().unwrap().insert(shared_event_id, event.clone());

        Ok(MetalEventHandle { shared_event_id })
    }

    /// Import a shared event from a handle
    pub fn import_shared_event(&self, handle: &MetalEventHandle) -> Result<MTLSharedEvent> {
        // Create a new shared event from the handle
        self.device.new_shared_event_with_handle(handle.shared_event_id)
            .ok_or(GeyserError::MetalApiError(format!("Failed to import shared event with handle {}", handle.shared_event_id)))
    }

    /// Signal an event to a specific value
    pub fn signal_event(&self, event: &MTLSharedEvent, value: u64) {
        event.set_signaled_value(value);
    }

    /// Wait for an event to reach a specific value (CPU-side)
    pub fn wait_for_event(&self, event: &MTLSharedEvent, value: u64) -> Result<()> {
        // Note: This is a blocking wait on the CPU
        // For GPU-side synchronization, you would encode wait/signal commands in the command buffer
        if event.signaled_value() >= value {
            Ok(())
        } else {
            // In a real implementation, you'd want to use MTLSharedEventListener
            // for efficient waiting. This is a simplified version.
            Err(GeyserError::Other("Event not signaled to requested value yet".to_string()))
        }
    }

    /// Release a shared event
    pub fn release_shared_event(&self, handle: &MetalEventHandle) -> Result<()> {
        self.exported_events.lock().unwrap().remove(&handle.shared_event_id);
        Ok(())
    }
}

impl TextureShareManager for MetalTextureShareManager {
    fn create_shareable_texture(&self, descriptor: &TextureDescriptor) -> Result<Box<dyn SharedTexture>> {
        let mtl_pixel_format = self.map_texture_format_to_mtl(descriptor.format)?;
        let mtl_texture_usage = self.map_texture_usage_to_mtl(&descriptor.usage);

        // 1. Create an IOSurface
        let mut io_surface_props = IOSurfaceProperties::new();
        io_surface_props.set_width(descriptor.width as usize);
        io_surface_props.set_height(descriptor.height as usize);
        // Set bytes per element based on format
        let bytes_per_elem = self.bytes_per_element(descriptor.format);
        io_surface_props.set_bytes_per_element(bytes_per_elem);

        let io_surface = IOSurface::new(&io_surface_props)
            .ok_or(GeyserError::MetalInitializationError("Failed to create IOSurface".to_string()))?;

        // 2. Create a MTLTexture from the IOSurface
        let texture_descriptor = MTLTextureDescriptor::new();
        texture_descriptor.set_pixel_format(mtl_pixel_format);
        texture_descriptor.set_width(descriptor.width as u64);
        texture_descriptor.set_height(descriptor.height as u64);
        texture_descriptor.set_usage(mtl_texture_usage);
        // Use Shared storage mode for IOSurface-backed textures
        // This allows CPU and GPU access, which is necessary for sharing
        texture_descriptor.set_storage_mode(MTLStorageMode::Shared);

        let texture = self.device.new_texture_with_descriptor_from_io_surface(&texture_descriptor, &io_surface)
            .ok_or(GeyserError::MetalApiError("Failed to create MTLTexture from IOSurface".to_string()))?;

        Ok(Box::new(MetalSharedTexture {
            device: self.device.clone(),
            texture,
            io_surface: Some(io_surface),
            descriptor: descriptor.clone(),
            exported_handle: None,
        }))
    }

    fn export_texture(&self, texture: &dyn SharedTexture) -> Result<ApiTextureHandle> {
        let metal_texture = texture
            .as_any()
            .downcast_ref::<MetalSharedTexture>()
            .ok_or(GeyserError::Other("Provided texture is not a MetalSharedTexture".to_string()))?;

        let io_surface = metal_texture.io_surface.as_ref()
            .ok_or(GeyserError::MetalApiError("Cannot export a Metal texture not backed by an owned IOSurface".to_string()))?;

        let io_surface_id = io_surface.get_id();

        // Store a reference to the IOSurface to keep it alive
        self.exported_surfaces.lock().unwrap().insert(io_surface_id, io_surface.clone());

        Ok(ApiTextureHandle::Metal(MetalTextureShareHandle { io_surface_id }))
    }

    fn import_texture(&self, handle: ApiTextureHandle, descriptor: &TextureDescriptor) -> Result<Box<dyn SharedTexture>> {
        let metal_handle = match handle {
            ApiTextureHandle::Metal(h) => h,
            _ => return Err(GeyserError::InvalidTextureHandle),
        };

        let io_surface = IOSurface::lookup(metal_handle.io_surface_id)
            .ok_or(GeyserError::MetalApiError("Failed to lookup IOSurface by ID".to_string()))?;

        let mtl_pixel_format = self.map_texture_format_to_mtl(descriptor.format)?;
        let mtl_texture_usage = self.map_texture_usage_to_mtl(&descriptor.usage);

        let texture_descriptor = MTLTextureDescriptor::new();
        texture_descriptor.set_pixel_format(mtl_pixel_format);
        texture_descriptor.set_width(descriptor.width as u64);
        texture_descriptor.set_height(descriptor.height as u64);
        texture_descriptor.set_usage(mtl_texture_usage);
        // Use Shared storage mode for imported IOSurface-backed textures
        texture_descriptor.set_storage_mode(MTLStorageMode::Shared);

        let texture = self.device.new_texture_with_descriptor_from_io_surface(&texture_descriptor, &io_surface)
            .ok_or(GeyserError::MetalApiError("Failed to create MTLTexture from imported IOSurface".to_string()))?;

        Ok(Box::new(MetalSharedTexture {
            device: self.device.clone(),
            texture,
            io_surface: Some(io_surface),
            descriptor: descriptor.clone(),
            exported_handle: Some(metal_handle),
        }))
    }

    fn release_texture_handle(&self, handle: ApiTextureHandle) -> Result<()> {
        let io_surface_id = match handle {
            ApiTextureHandle::Metal(h) => h.io_surface_id,
            _ => return Err(GeyserError::InvalidTextureHandle),
        };

        self.exported_surfaces.lock().unwrap().remove(&io_surface_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
