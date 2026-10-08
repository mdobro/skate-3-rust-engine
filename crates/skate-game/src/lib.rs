#![cfg_attr(target_os = "android", allow(dead_code, unused_imports, unused_variables))]
macro_rules! console {
    ($($t:tt)*) => {{
        #[cfg(not(target_os = "android"))]
        eprintln!($($t)*);
        #[cfg(target_os = "android")]
        bevy::log::info!($($t)*);
    }};
}
macro_rules! report_meta {
    ($($t:tt)*) => { console!("REPORT_META {}", format_args!($($t)*)) };
}

#[cfg(target_os = "android")]
mod android_lifecycle;
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
mod android_perf;
mod render_caps;
mod frame_timing;
mod animation;
mod crash_report;
mod crash_context;
mod multiplayer;
mod apt_vm;
mod apt_display;
mod apt_movie;
mod apt_text;
mod apt_scene;
mod hud_runtime;
mod scoring_runtime;
mod scoring_hud;
mod animation_pose;
mod app;
mod assets;
mod camera;
mod config;
mod setup;
#[cfg(not(target_os = "android"))]
mod updater;
#[cfg(target_os = "android")]
#[path = "updater_android.rs"]
mod updater;
mod map_library;
mod map_render;
mod map_transition;
mod difficulty;
mod custom_difficulty;
mod graph_host;
mod graph_runtime;
mod input;
mod session_marker;
mod physics;
mod skater_animation;
mod verification;
mod performance;
mod profiling;
mod graphics_menu;
mod modding;
mod customiser;
mod customiser_parts;
mod customiser_material;
mod custom_models;
mod teleport_menu;
mod render_capacity;
mod shadow_quality;
mod retail_render;
mod retail_character;
mod retail_exposure;
mod retail_irradiance;
mod retail_sky;
mod presentation;
mod debug_cam;
mod replay;
mod world;
mod grind_world;
mod skate_world;
mod map_validation;
mod water_splash;
mod game_audio;
pub(crate) mod world_audio;
pub(crate) mod ui_audio;
mod trigger_volumes;
pub mod platform_paths;
#[allow(dead_code)]
mod std_fs_reader;
pub use config::Config;

pub struct Launch {
    pub config: Config,
    pub paths: platform_paths::PlatformPaths,
}

/// Keeps the log subscriber and any trace capture alive.
pub struct LogGuard(#[allow(dead_code)] profiling::Guard);

pub fn init_logging() -> Result<LogGuard, String> {
    profiling::init().map(LogGuard)
}

/// Desktop update recovery. `Ok(true)` means the process should exit.
pub fn recover_update() -> Result<bool, String> {
    updater::recover()
}

/// Desktop crash supervisor. `Some(code)` means exit with that code.
pub fn crash_report_entry() -> Option<i32> {
    crash_report::entry()
}

/// Validates an imported installation without opening a window.
pub fn check_assets(asset_root: &std::path::Path, map: Option<&std::path::Path>) -> Result<(), String> {
    let asset_root = asset_root.canonicalize().map_err(|e| format!("Asset root {}: {e}", asset_root.display()))?;
    let map_path = match map {
        Some(path) => Some(path.to_owned()),
        None => map_library::default_map(&asset_root)?,
    };
    let map = map_path.as_deref().map(skate_data::skate_map::SkateMap::load).transpose()?;
    let map_path = map_path.map(|p| p.canonicalize().map_err(|e| e.to_string())).transpose()?;
    let difficulty = difficulty::Difficulty::load(&asset_root)?;
    skate_data::input_config::StockGameplayConfig::load(&asset_root).map_err(|e| e.to_string())?;
    let manifest = skate_data::GameAssets::load(&asset_root).map_err(|e| e.to_string())?;
    let graphs = graph_runtime::StockGraphs::load(&asset_root, &manifest)?;
    if let Some(map) = &map {
        skate_world::validate_runtime(map)?;
    }
    let physics = physics::GamePhysics::load_with_difficulty(&asset_root, map.as_ref(), difficulty)?;
    physics::SkaterRuntime::load(&asset_root, &graphs, &physics, difficulty.profile_key())?;
    physics::PlayerControls::load(&asset_root)?;
    camera::CameraRuntime::load(&asset_root)?;
    trigger_volumes::check(map_path.as_deref(), map.as_ref())?;
    Ok(())
}

pub fn run(launch: Launch) -> bevy::app::AppExit {
    let Launch { config, paths } = launch;
    paths.install();
    #[cfg(target_os = "android")]
    crash_report::entry();
    let _startup = bevy::log::info_span!("startup").entered();
    profiling::map_metadata(&config);
    report_meta!("startup=map_fingerprint:{:016x} difficulty:{} multiplayer_requested:{} custom_appearance:{} renderer:Vulkan", config.map_fingerprint, config.difficulty.key(), config.multiplayer.host.is_some() || config.multiplayer.direct.is_some(), config.multiplayer.appearance.is_some());
    report_meta!("stage=gameplay_configuration");
    if let Err(error) = skate_data::input_config::StockGameplayConfig::load(&config.asset_root) {
        console!("{error}");
        return bevy::app::AppExit::error();
    }
    report_meta!("stage=asset_manifest");
    let manifest = match bevy::log::info_span!("load_manifest").in_scope(|| skate_data::GameAssets::load(&config.asset_root)) {
        Ok(manifest) => manifest,
        Err(error) => {
            console!("{error}");
            return bevy::app::AppExit::error();
        }
    };
    report_meta!("stage=stock_graphs");
    let graphs = match bevy::log::info_span!("load_graphs").in_scope(|| graph_runtime::StockGraphs::load(&config.asset_root, &manifest)) {
        Ok(graphs) => graphs,
        Err(error) => {
            console!("{error}");
            return bevy::app::AppExit::error();
        }
    };
    if config.validate_maps {
        return match map_validation::run(&config, &graphs) {
            Ok(()) => bevy::app::AppExit::Success,
            Err(error) => {
                console!("{error}");
                bevy::app::AppExit::error()
            }
        };
    }
    report_meta!("stage=map_validation");
    if let Some(map) = &config.map {
        if let Err(error) = skate_world::validate_runtime(map) {
            console!("{error}");
            return bevy::app::AppExit::error();
        }
    }
    console!("SKATE_DIFFICULTY mode={} native_index={}", config.difficulty.key(), config.difficulty as u32);
    report_meta!("stage=physics_initialization");
    let mut physics = match bevy::log::info_span!("load_physics").in_scope(|| physics::GamePhysics::load_with_difficulty(&config.asset_root, config.map.as_ref(), config.difficulty)) {
        Ok(physics) => physics,
        Err(error) => {
            console!("{error}");
            return bevy::app::AppExit::error();
        }
    };
    if config.multiplayer.spawn_offset != 0.0 {
        let mut spawn = physics.board.part_transforms()[skate_core::physics::board::BodyId::Deck.index()];
        spawn.translation.x += config.multiplayer.spawn_offset;
        physics.board.set_transform(spawn);
    }
    report_meta!("stage=skater_initialization");
    let skater = match bevy::log::info_span!("load_skater").in_scope(|| physics::SkaterRuntime::load(&config.asset_root, &graphs, &physics, config.difficulty.profile_key())) {
        Ok(skater) => skater,
        Err(error) => {
            console!("{error}");
            return bevy::app::AppExit::error();
        }
    };
    report_meta!("stage=controls_initialization");
    let controls = match physics::PlayerControls::load(&config.asset_root) {
        Ok(controls) => controls,
        Err(error) => { console!("{error}"); return bevy::app::AppExit::error(); }
    };
    if config.check_assets {
        if let Err(error) = camera::CameraRuntime::load(&config.asset_root) {
            console!("{error}");
            return bevy::app::AppExit::error();
        }
        match trigger_volumes::check(config.map_path.as_deref(), config.map.as_ref()) {
            Ok(count) => console!("SKATE_TRIGGERS_READY volumes={count}"),
            Err(error) => {
                console!("{error}");
                return bevy::app::AppExit::error();
            }
        }
        console!("SKATE_ASSETS_READY");
        return bevy::app::AppExit::Success;
    }
    report_meta!("stage=renderer_and_app_initialization");
    let mut app = app::build(config, manifest, graphs, physics, skater);
    app.insert_resource(controls);
    report_meta!("stage=app_run");
    drop(_startup);
    app.run()
}

#[cfg(test)]
#[path = "tests/action_host.rs"]
mod action_host_tests;

/// Writes the retail water/ocean animation table for setup (see
/// skate_data::ocean_pca). Runs before any game initialization.
pub fn extract_ocean_pca() -> Option<i32> {
    let mut args = std::env::args_os().skip(1);
    if args.next()? != "--extract-ocean-pca" {
        return None;
    }
    let (Some(xex), Some(out), None) = (args.next(), args.next(), args.next()) else {
        eprintln!("Usage: skate3rust --extract-ocean-pca <default.xex> <ocean-pca.json>");
        return Some(2);
    };
    let result = std::fs::read(&xex)
        .map_err(|e| format!("{}: {e}", std::path::Path::new(&xex).display()))
        .and_then(|bytes| skate_data::ocean_pca::json_from_xex(&bytes))
        .and_then(|json| std::fs::write(&out, json).map_err(|e| format!("{}: {e}", std::path::Path::new(&out).display())));
    match result {
        Ok(()) => {
            println!("OCEAN_PCA_READY");
            Some(0)
        }
        Err(error) => {
            eprintln!("Ocean PCA extraction failed: {error}");
            Some(1)
        }
    }
}
