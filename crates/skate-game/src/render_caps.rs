//! Startup GPU capability check.
//!
//! Logs the adapter and the limits the renderer depends on, publishes the
//! texture-array layer limit to the page allocator in `retail_render`, and
//! warns when something the world shader needs is missing. A guard, not a
//! fallback: there is no second renderer.
use bevy::prelude::*;
use bevy::render::render_resource::DownlevelFlags;
use bevy::render::renderer::{RenderAdapter, RenderAdapterInfo, RenderDevice};

/// Sampled textures per stage the world material needs: 8 pages, cubes, plus
/// the view and shadow bindings Bevy adds. The wgpu default is 16.
const MIN_SAMPLED_TEXTURES: u32 = 16;
/// World material (3), mesh and light buffers Bevy adds.
const MIN_STORAGE_BUFFERS: u32 = 8;

pub(crate) struct RenderCapsPlugin;
impl Plugin for RenderCapsPlugin {
    fn build(&self, app: &mut App) {
        // PreStartup: the map is staged in Startup and needs the page limit.
        app.add_systems(PreStartup, report);
    }
}

fn report(device: Res<RenderDevice>, adapter: Res<RenderAdapter>, info: Res<RenderAdapterInfo>) {
    let limits = device.limits();
    let downlevel = adapter.get_downlevel_capabilities().flags;
    let cube_array = downlevel.contains(DownlevelFlags::CUBE_ARRAY_TEXTURES);
    let compute = downlevel.contains(DownlevelFlags::COMPUTE_SHADERS);
    info!(
        "GPU_CAPS adapter=\"{}\" backend={:?} type={:?} driver=\"{} {}\" max_texture_array_layers={} max_storage_buffers_per_shader_stage={} max_sampled_textures_per_shader_stage={} max_texture_2d={} cube_array={} compute={}",
        info.name,
        info.backend,
        info.device_type,
        info.driver,
        info.driver_info,
        limits.max_texture_array_layers,
        limits.max_storage_buffers_per_shader_stage,
        limits.max_sampled_textures_per_shader_stage,
        limits.max_texture_dimension_2d,
        cube_array,
        compute,
    );
    crate::retail_render::set_page_layer_limit(limits.max_texture_array_layers as usize);
    if !cube_array {
        warn!("GPU_CAPS cube array textures are unsupported: retail environment reflections will fail to bind");
    }
    if !compute {
        warn!("GPU_CAPS compute shaders are unsupported: auto exposure and GPU mesh preprocessing cannot run");
    }
    if limits.max_sampled_textures_per_shader_stage < MIN_SAMPLED_TEXTURES {
        warn!(
            "GPU_CAPS max_sampled_textures_per_shader_stage {} is below the {MIN_SAMPLED_TEXTURES} the world material needs",
            limits.max_sampled_textures_per_shader_stage
        );
    }
    if limits.max_storage_buffers_per_shader_stage < MIN_STORAGE_BUFFERS {
        warn!(
            "GPU_CAPS max_storage_buffers_per_shader_stage {} is below the {MIN_STORAGE_BUFFERS} the world material needs",
            limits.max_storage_buffers_per_shader_stage
        );
    }
    if (limits.max_texture_array_layers as usize) < crate::retail_render::MAX_PAGE_LAYERS {
        info!(
            "GPU_CAPS texture pages are limited to {} layers (default {}); more material slabs will be used",
            limits.max_texture_array_layers,
            crate::retail_render::MAX_PAGE_LAYERS
        );
    }
}
