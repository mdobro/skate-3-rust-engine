# Phase 0 notes

## What changed

- `skate-game` is now a library (`src/lib.rs`, `[lib] name = "skate_game"`) plus the three existing bins. `main.rs` keeps the desktop steps (update recovery, `--extract-ocean-pca`, crash supervisor, logging, `Config::from_env`) and calls `skate_game::run(Launch)`. The other two bins still `include!("main.rs")`.
- Public API: `run`, `Launch { config, paths }`, `Config` (`from_env`, `for_android`), `check_assets(asset_root, map)`, `extract_ocean_pca`, `init_logging`, `recover_update`, `crash_report_entry`, `platform_paths`.
- `platform_paths`: `PlatformPaths::desktop()` keeps the old locations (exe dir `data` and `mods`, `%LOCALAPPDATA%\Skate3RustEngine\...`, `SKATE3_MODS`). `PlatformPaths::android(files_dir)` uses `installation`, `mods`, `custom-characters`, `crashes`, `tmp`. `get()` returns the installed paths or the desktop layout. `run` installs the launch paths.
- Android asset root: `<data_dir>/installation.json` (reusing `setup::installed`), else `<data_dir>/assets` when `private/game.json` exists, else "No game data imported".
- `std_fs_reader`: `StdFsAssetReader` plus `local(root)`, which picks the reader per platform. Android registers it as `AssetSourceId::Default` (before `DefaultPlugins`), `mods`, `characters` and `online-characters`. Desktop is unchanged. `FileAssetReader` does exist on Android in bevy_asset 0.18.1, but it calls `current_exe()` for its base path, so the std reader is used.
- Gated on Android: `updater` (stub in `updater_android.rs`), `crash_report::entry` (panic hook writing `crash-<unix>.txt` and logcat), SDL3 (`sdl3` is a non-Android dependency; the Android backend is `Unavailable` until Phase 3; SDL-only items in `controller_kind.rs` are gated), importer in `custom_models.rs`, Steam relay, "open mods folder".
- Logging: a logcat `fmt` writer in `profiling.rs` (tag `skate3`, via `android_log-sys`). `console!` and `report_meta!` macros send `REPORT_META` and error lines to stderr on desktop and to tracing on Android.
- New crate `crates/skate-android` (cdylib): `#[bevy_main]` entry, JNI `checkAssets` and `verifyImport`. The crate body is `cfg(target_os = "android")`, so it is empty on host builds and `cargo check --workspace` still works.
- CI: `.github/workflows/portable.yml`.

## Checks

```
cargo check -p skate-game
cargo test -p skate-game --no-run
export ANDROID_NDK_HOME=/opt/android-sdk/ndk/27.2.12479018 ANDROID_HOME=/opt/android-sdk
cargo ndk -t arm64-v8a check -p skate-android --no-default-features
```

## Deviations and open issues

- `Config`, `Difficulty` and `multiplayer::Options` became `pub` so the public `Launch` type is well-formed.
- `check_assets` re-implements the `--check-assets` flow (load default map if none given) instead of sharing code with `run`; the `--check-assets` CLI flag still works through `run`.
- Windows code paths were edited only by moving path lookups into `platform_paths`; they could not be built here.
- The Android temp dir is created by `skate-android` before `run`; code that calls `std::env::temp_dir()` directly (tests only) is untouched.
