//! Bevy Plugin for Geyser Texture Sharing
//!
//! This module provides a Bevy plugin that enables zero-copy texture sharing
//! between Geyser and Bevy's rendering system via wgpu.
//!
//! # Features
//! - Import Geyser-managed textures as Bevy Image assets
//! - Export Bevy textures for cross-process sharing
//! - Automatic synchronization and lifecycle management
//! - Support for Vulkan backend (wgpu-hal)
//!
//! # Usage
//! ```ignore
//! use bevy::prelude::*;
//! use geyser::bevy_plugin::GeyserPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(GeyserPlugin)
//!     .run();
//! ```

use bevy::prelude::*;
use bevy::render::{
    RenderApp,
    extract_resource::ExtractResource,
    renderer::RenderDevice,
};
use std::sync::Arc;

#[cfg(feature = "vulkan")]
use crate::{
    vulkan::VulkanTextureShareManager,
    common::{ApiTextureHandle, TextureDescriptor},
    SharedTexture,
};

/// Bevy plugin for Geyser texture sharing
pub struct GeyserPlugin;

impl Plugin for GeyserPlugin {
    fn build(&self, app: &mut App) {
        // Add resources to main app
        app.init_resource::<GeyserState>();
        
        // Register events
        app.add_event::<ImportGeyserTexture>();
        app.add_event::<ExportBevyTexture>();
        
        // Add systems for texture management
        app.add_systems(Update, (
            process_shared_texture_events,
            cleanup_expired_textures,
        ));
    }

    fn finish(&self, app: &mut App) {
        // Initialize render-world resources
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<GeyserRenderState>()
                .add_systems(
                    bevy::render::Render,
                    extract_geyser_textures.in_set(bevy::render::RenderSet::ExtractCommands),
                );
        }
    }
}

/// Main-world state for Geyser integration
#[derive(Resource, Default)]
pub struct GeyserState {
    /// Mapping from entity to shared texture handle
    pub shared_textures: std::collections::HashMap<Entity, SharedTextureData>,
}

/// Data for a shared texture
pub struct SharedTextureData {
    pub api_handle: ApiTextureHandle,
    pub descriptor: TextureDescriptor,
    pub image_handle: Handle<Image>,
}

/// Render-world state for Geyser
#[derive(Resource, Default, ExtractResource, Clone)]
pub struct GeyserRenderState {
    #[cfg(feature = "vulkan")]
    pub manager: Option<Arc<VulkanTextureShareManager>>,
}

/// Component to mark an entity as having a Geyser-managed texture
#[derive(Component)]
pub struct GeyserSharedTexture {
    pub api_handle: ApiTextureHandle,
}

/// Event to request importing a Geyser texture into Bevy
#[derive(Event)]
pub struct ImportGeyserTexture {
    pub api_handle: ApiTextureHandle,
    pub descriptor: TextureDescriptor,
    /// Optional entity to attach the texture to
    pub target_entity: Option<Entity>,
}

/// Event to request exporting a Bevy texture via Geyser
#[derive(Event)]
pub struct ExportBevyTexture {
    pub image_handle: Handle<Image>,
    /// Optional entity to track the exported texture
    pub source_entity: Option<Entity>,
}

/// System to process texture import/export requests
fn process_shared_texture_events(
    mut state: ResMut<GeyserState>,
    mut import_events: EventReader<ImportGeyserTexture>,
    mut export_events: EventReader<ExportBevyTexture>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    // Process import requests
    for event in import_events.read() {
        info!("Processing Geyser texture import request");
        
        // For now, create a placeholder Bevy image
        // TODO: Use wgpu-hal to import the actual Vulkan texture
        let size = bevy::render::render_resource::Extent3d {
            width: event.descriptor.width,
            height: event.descriptor.height,
            depth_or_array_layers: 1,
        };
        
        let format = match event.descriptor.format {
            crate::common::TextureFormat::Rgba8Unorm => {
                bevy::render::render_resource::TextureFormat::Rgba8Unorm
            }
            _ => {
                warn!("Unsupported texture format, defaulting to Rgba8Unorm");
                bevy::render::render_resource::TextureFormat::Rgba8Unorm
            }
        };
        
        let image = Image::new_fill(
            size,
            bevy::render::render_resource::TextureDimension::D2,
            &[0, 0, 0, 255],
            format,
        );
        
        let image_handle = images.add(image);
        
        // Store the mapping
        let entity = event.target_entity.unwrap_or_else(|| commands.spawn_empty().id());
        
        commands.entity(entity).insert(GeyserSharedTexture {
            api_handle: event.api_handle.clone(),
        });
        
        state.shared_textures.insert(
            entity,
            SharedTextureData {
                api_handle: event.api_handle.clone(),
                descriptor: event.descriptor.clone(),
                image_handle,
            },
        );
        
        info!("Imported Geyser texture for entity {:?}", entity);
    }
    
    // Process export requests
    for event in export_events.read() {
        info!("Processing Bevy texture export request");
        // TODO: Implement export from Bevy Image to Geyser handle
        warn!("Texture export not yet implemented");
    }
}

/// System to clean up expired shared textures
fn cleanup_expired_textures(
    mut state: ResMut<GeyserState>,
    query: Query<Entity, With<GeyserSharedTexture>>,
) {
    // Remove entries for despawned entities
    state.shared_textures.retain(|entity, _| query.contains(*entity));
}

/// Extract system to move Geyser state to render world
fn extract_geyser_textures(
    state: Extract<Res<GeyserState>>,
    mut render_state: ResMut<GeyserRenderState>,
) {
    // TODO: Extract texture handles and prepare for rendering
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_builds() {
        let mut app = App::new();
        app.add_plugins(GeyserPlugin);
        // Plugin should build without errors
    }
}
