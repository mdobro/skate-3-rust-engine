use skate_game::{Config, Launch, platform_paths::PlatformPaths};

fn main() -> bevy::app::AppExit {
    match skate_game::recover_update() {
        Ok(true) => return bevy::app::AppExit::Success,
        Err(error) => { eprintln!("{error}"); return bevy::app::AppExit::Success; },
        Ok(false) => {}
    }
    // Setup tool mode: `--extract-ocean-pca <default.xex> <ocean-pca.json>`.
    // Before the crash supervisor, whose report window would block setup on
    // a failed (non-zero) extraction.
    if let Some(code) = skate_game::extract_ocean_pca() { std::process::exit(code); }
    if let Some(code) = skate_game::crash_report_entry() { std::process::exit(code); }
    let _trace = match skate_game::init_logging() {
        Ok(guard) => guard,
        Err(error) => { eprintln!("{error}"); return bevy::app::AppExit::error(); }
    };
    let startup = bevy::log::info_span!("startup").entered();
    eprintln!("REPORT_META stage=configuration_and_installation");
    let config = match bevy::log::info_span!("load_configuration_and_map").in_scope(Config::from_env) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}");
            return bevy::app::AppExit::error();
        }
    };
    drop(startup);
    skate_game::run(Launch { config, paths: PlatformPaths::desktop() })
}
