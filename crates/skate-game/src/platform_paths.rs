//! Per-platform storage locations. Desktop keeps the portable-package layout;
//! Android uses the app's external files directory.
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Clone, Debug, bevy::prelude::Resource)]
pub struct PlatformPaths {
    /// Holds `installation.json` and the imported game data.
    pub data_dir: PathBuf,
    pub mods_dir: PathBuf,
    pub custom_characters_dir: PathBuf,
    pub crash_dir: PathBuf,
    pub temp_dir: PathBuf,
}

static PATHS: OnceLock<PlatformPaths> = OnceLock::new();

impl PlatformPaths {
    pub fn desktop() -> Self {
        let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(PathBuf::from));
        let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
        Self {
            data_dir: exe_dir.as_ref().map_or_else(|| "data".into(), |d| d.join("data")),
            mods_dir: std::env::var_os("SKATE3_MODS").map(PathBuf::from)
                .unwrap_or_else(|| exe_dir.map_or_else(|| "mods".into(), |d| d.join("mods"))),
            custom_characters_dir: local.join("Skate3RustEngine/custom-characters"),
            crash_dir: local.join("Skate3RustEngine/CrashReports"),
            temp_dir: std::env::temp_dir(),
        }
    }

    pub fn android(files_dir: PathBuf) -> Self {
        Self {
            data_dir: files_dir.join("installation"),
            mods_dir: files_dir.join("mods"),
            custom_characters_dir: files_dir.join("custom-characters"),
            crash_dir: files_dir.join("crashes"),
            temp_dir: files_dir.join("tmp"),
        }
    }

    /// First call wins. Later calls leave the installed paths untouched.
    pub fn install(self) -> &'static PlatformPaths {
        PATHS.get_or_init(|| self)
    }
}

/// The installed paths, or the desktop layout when none were installed.
pub fn get() -> &'static PlatformPaths {
    PATHS.get_or_init(PlatformPaths::desktop)
}
