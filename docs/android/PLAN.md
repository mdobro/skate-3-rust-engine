# Android port plan: Native Android app for the Skate 3 Rust Engine (Backbone controller)

## Context

The engine (Rust 2024 + Bevy 0.18.1, with `crates/skate-game` as the binary) only ships for Windows. Three things tie it there:
- First-run setup is a PyInstaller/Tkinter program (`tools/setup.py` → `tools/asset_pipeline/install.py`) that converts the user's Xbox 360 ISO.
- Gamepad input comes from SDL3 on its own thread, with an XInput fallback (`crates/skate-game/src/input/platform.rs`).
- Startup spawns `.exe` helpers: the crash supervisor, the updater and setup.

The goal is an Android APK that plays the game natively with a **Backbone** controller on a **Snapdragon 8 Gen 2 or newer** phone (Adreno 740+, 8–12 GB RAM).

**Primary test device: Samsung Galaxy S26 Ultra** (Snapdragon 8 Elite for Galaxy, recent Adreno, 12 GB+ RAM). Notes for this device:
- The S26 Ultra's GPU and memory are comfortably above the target, so Phase 4 should mostly mean tuning, not cutting features.
- Samsung Game Booster and Game Launcher can throttle or cap frame rate. Test both with and without them.
- The Backbone may connect over USB-C or Bluetooth, depending on the model. Android reports both as the same kind of gamepad through `InputManager`, so Phase 3 handles both. Log the vendor and product IDs for both connections.

**Game data:** the ISO is converted **on the PC** with the existing setup, which is faster and already works. The app then imports that converted asset set. The app ships no EA assets and never downloads any; the third-party zip link is not wired in.

**Why native instead of the existing web port:** the engine needs things a browser limits:
- **Real threads:** the map-loader thread, rapier, the audio DSP, and Bevy's parallel systems.
- **More than 4 GB of address space:** wasm32 is capped at 4 GB, and DownTown alone peaks at about 1.7 GB before GPU textures.
- **Direct Vulkan** with compute and storage buffers.
- **Low-latency gamepad input** instead of the polled browser Gamepad API.

Helpful findings from the code:
- Simulation, gameplay input and flick-it recognition are platform-neutral. A backend only has to produce `skate_core::input::xbox::XboxState` (`crates/skate-core/src/input/xbox.rs`).
- Rendering is already Vulkan-only (`app.rs`).
- Textures are already RGBA8, so there is no BCn problem.
- `android-activity` and `ndk` are already in `Cargo.lock`.

---

## Architecture

```
PC (Windows, existing setup)                       Phone
ISO → skate3setup.exe → data/installations/<id>/   ──zip──►  Import (SAF picker or adb push)
                         "Export for Android"                 → <external files>/installation/
android/                            (new Gradle project)
  LauncherActivity   data status, "Import game data (.zip)", Play
  SkateActivity      extends GameActivity; loads libskate_android.so; gamepad → JNI
  GamepadBridge      InputManager listener + key/motion → native slot state
crates/skate-android/  (new) cdylib, #[bevy_main], JNI exports
```

## Phase 0: Make `skate-game` portable (Windows builds unchanged)

1. **Split into a library and a binary.**
   - Move the `mod` declarations and `main()` body from `crates/skate-game/src/main.rs` into a new `src/lib.rs` as `pub fn run(launch: Launch) -> AppExit`. `Launch` holds the parsed `Config` plus platform paths.
   - `main.rs` keeps the desktop-only steps: `updater::recover`, `extract_ocean_pca`, `crash_report::entry`, `Config::from_env`.
   - Also expose the `--check-assets` block as a `pub fn`, so the app can validate an import.
2. **Dependencies** (`crates/skate-game/Cargo.toml`):
   - Move `sdl3` under `[target.'cfg(not(target_os = "android"))'.dependencies]`.
   - Android builds use `--no-default-features`, which drops `dev-dynamic` (`bevy/dynamic_linking`).
   - Never build `skate-steam-relay` for Android.
3. **cfg-gate desktop-only code**, with Android no-ops:
   - `crash_report.rs`: the supervisor re-spawns `current_exe()`. On Android, use a panic hook that writes to app storage and logcat, reusing `crash_context`.
   - `updater.rs`: no-op on Android.
   - `setup.rs`: `asset_root()` runs `skate3setup.exe`. On Android, read `installation.json` under the app data dir instead.
   - `custom_models.rs`: hide the importer (it spawns `skate3setup.exe --character-import`).
   - `multiplayer/transport.rs`: hide the Steam relay. Direct UDP can stay.
   - `modding/menu.rs:283`: hide "open folder".
4. **Paths.**
   - Add a `platform_paths` module with the data dir, mods dir, custom-characters dir and crash dir.
   - On Android these come from `getExternalFilesDir(null)` passed over JNI.
   - It replaces the `%LOCALAPPDATA%`, `current_exe().parent()` and `temp_dir()` uses in `setup.rs`, `config.rs`, `custom_models.rs`, `modding/mod.rs:221` and `crash_report.rs`.
   - `settings/` stays at `asset_root.parent()/settings`.
5. **Asset sources.**
   - On Android, Bevy's default reader is the APK `AssetManager`, so `AssetPlugin.file_path` is ignored.
   - Add a small `StdFsAssetReader` (an `AssetReader` over `std::fs`). Register it as `AssetSourceId::Default` before `DefaultPlugins` in `app.rs`.
   - Use it on Android for `mods://` (`modding/mod.rs:211`) and `characters://` / `online-characters://` (`custom_models.rs:147-151`). These currently use `FileAssetReader`; check whether that type compiles on Android.
6. **Logging:** `profiling.rs` installs its own stderr subscriber because `LogPlugin` is disabled. Add a logcat layer with tag `skate3`, and route the `REPORT_META` lines through it.
7. **CI:** add a Linux `cargo check`/`cargo test` job and an Android `cargo ndk -t arm64-v8a check -p skate-android --no-default-features` job to `.github/workflows/`.

## Phase 1: Game data from the PC

1. **"Export for Android" on the PC.**
   - Add `tools/export_android.py`, plus a button in the setup GUI (`tools/setup.py`).
   - It zips the active installation root, the folder that `data/installation.json` points to: `maps/`, `maps.json`, `settings/` and `assets/`. Audio is already WAV, and speech is optional.
   - Include a `release.json`-style manifest of file hashes and the pipeline fingerprints from `tools/asset_pipeline/versions.py`. The app uses it to check that the import is complete and compatible.
   - Use the zip "store" method: the data is already compressed, so deflating again only wastes time.
2. **Import on the phone.**
   - `LauncherActivity` → "Import game data" opens the Storage Access Framework (SAF) zip picker. It streams the zip into `<external files>/installation.new/`, verifies the hashes, calls the exported `check_assets`, then atomically renames the folder to `installation/`.
   - Show progress and check free space first.
   - Developer path: `adb push` the extracted folder to `/sdcard/Android/data/<pkg>/files/installation/`.
3. **Updates:** if the engine's pipeline fingerprints change, the app says "re-export from PC". The existing per-group receipts make that re-run on the PC quick.
4. **Optional later step, only if Phase 3 measurements need it:** during export, convert map textures to **ASTC 6x6** on the PC. That means a new codec in `crates/skate-data/src/skate_map/texture_decode.rs` and a bump to the map pipeline version. It saves phone memory and costs the phone nothing.

## Phase 2: Boot on Android

1. **New crate `crates/skate-android`.**
   - `crate-type = ["cdylib"]`, depending on `skate-game` with default features off, plus `jni`.
   - `#[bevy_main]` builds `Launch` from the paths `SkateActivity` passes, then calls `skate_game::run`.
2. **Gradle project `android/`.**
   - `minSdk 29`, `targetSdk 35`, arm64-v8a only.
   - Pin `androidx.games:games-activity` to the exact version the locked `android-activity` crate expects. A mismatch crashes at startup.
   - A Gradle task runs `cargo ndk -t arm64-v8a -o app/src/main/jniLibs build --release -p skate-android --no-default-features`.
   - Package `libc++_shared.so`, which oboe (cpal's Android backend) needs.
   - Lua (mlua, vendored), zstd-sys and oboe all build with the NDK.
3. **Window and lifecycle.**
   - Landscape (`sensorLandscape`), immersive fullscreen, `FLAG_KEEP_SCREEN_ON`.
   - `configChanges` covers keyboard and navigation, so plugging in the Backbone doesn't recreate the activity.
   - On Android, drop the fixed 1280×800 window and the fullscreen toggle (`graphics_menu.rs:655`).
   - Set `WinitSettings.unfocused_mode` to low-power.
   - Pause the audio output (`game_audio/native.rs`) on suspend, using Bevy's `AppLifecycle` events.

## Phase 3: Backbone controller input

The Backbone One (USB-C) appears to Android as a standard HID gamepad, with Xbox-layout keycodes and axes. SDL3 is not used on Android, because it needs its own `SDLActivity`, which conflicts with GameActivity.

1. **Kotlin `GamepadBridge`** inside `SkateActivity`:
   - Override `dispatchKeyEvent` and `dispatchGenericMotionEvent`. For gamepad and joystick sources, update the per-device state and **consume the event**. Otherwise B turns into BACK and closes the app.
   - Track connects and disconnects with `InputManager.InputDeviceListener`.
   - JNI calls: `nativeConnect(id, name, vendor, product)`, `nativeDisconnect(id)`, and `nativeState(id, buttons, lx, ly, rx, ry, lt, rt)`.
2. **Mapping to `XboxState`:**

   | Android input | `XboxState` value |
   |---|---|
   | `A`/`B`/`X`/`Y` | 0x1000 / 0x2000 / 0x4000 / 0x8000 |
   | `L1`/`R1` | 0x100 / 0x200 |
   | `THUMBL`/`THUMBR` | 0x40 / 0x80 |
   | `START`/`SELECT` | 0x10 / 0x20 |
   | D-pad from `HAT_X/Y` or `DPAD_*` keycodes | 0x1–0x8 |
   | `AXIS_X/Y` | left stick |
   | `AXIS_Z/RZ` | right stick |
   | Triggers: `max(LTRIGGER, BRAKE)` and `max(RTRIGGER, GAS)` | `u8` |

   - Sticks are scaled by 32767, with Y negated.
   - Add **no** extra deadzone: `xbox::convert` already applies the retail 0.25 radial deadzone, and flick-it is tuned to it.
   - Triggers must reach exactly 255 at full pull, since ground grabs and board drop need 1.0. Map anything ≥ 0.98 to 255.
3. **Rust backend.**
   - In `crates/skate-game/src/input/platform.rs`, add an `android` module that mirrors `mod sdl`'s `Shared` 4-slot array, with a packet number that bumps on every change.
   - Add a `Backend::Android` arm to `backend()` and `poll_cached`.
   - Everything downstream is unchanged:
     - `ControllerInput::collect` (`controllers.rs`)
     - `GameplayActions`, `gesture_input.rs` and `physics/controls.rs`
     - menu navigation (`customiser::navigation`, `graphics_menu.rs`)
4. **Identity.**
   - Add `ControllerKind::from_android(...)` in `input/controller_kind.rs`.
   - Add a Backbone model-table entry. The vendor ID is believed to be `0x358a`; confirm it from the connect log on the device.
   - The Esc → EXTRAS "Controller" row then reads "Backbone One". `settings/controller.json` `models` can still override it.
5. **UX.**
   - Show a "Connect your controller" overlay while no slot is ready.
   - Start already opens the Esc menu.
   - The Backbone/home button belongs to the Backbone app and stays unmapped.

## Phase 4: Mobile rendering and performance on Adreno 740+

1. **Startup capability check** in `retail_render.rs` / `render_capacity.rs`:
   - `max_texture_array_layers` against `MAX_PAGE_LAYERS = 2048`: clamp or split pages if lower.
   - Cube-array textures, fragment-stage storage buffers (`retail_material_bindings.wgsl`) and compute (`retail_exposure.wgsl`).
   - Log the results. Adreno 7xx supports all of these, so this is a guard rather than a rewrite.
2. **Mobile defaults** for `graphics_menu.rs` settings on first Android run:
   - render scale about 70%;
   - 60 fps cap (30 optional);
   - `AutoVsync`;
   - a new "Shadow quality" row that controls cascades and map size for the shadow lights in `retail_character.rs:215-255` and `world.rs:75`.
3. **GPU preprocessing and the vendored occlusion pyramid** (`vendor/bevy_pbr`, `vendor/bevy_core_pipeline`): if they fail validation on Adreno, turn them off on Android.
4. **Measure** memory (`dumpsys meminfo`) and frame time on DownTown. ASTC (Phase 1.4) is done only if needed.

## Phase 5: Packaging

- Release signing, and `versionCode` from the git revision.
- Crab icon from `docs/images/skating-crab.png`.
- "Delete game data" / "Re-import" options.
- Lua mods from `<external files>/mods`.
- An Android CI job that builds a debug APK. Game data is never bundled.

---

## Critical files

- **Engine entry and config:** `crates/skate-game/src/main.rs` → new `lib.rs`, `Cargo.toml`, `app.rs`, `setup.rs`, `config.rs`.
- **Desktop-only code to gate:** `crash_report.rs`, `updater.rs`, `profiling.rs`, `custom_models.rs`, `modding/{mod,menu}.rs`, `multiplayer/transport.rs`.
- **Input:** `input/platform.rs`, `input/controller_kind.rs`.
- **Rendering:** `graphics_menu.rs`, `retail_render.rs`, `render_capacity.rs`, `retail_character.rs`.
- **New:** `crates/skate-android/`, `android/`, `tools/export_android.py` plus a button in `tools/setup.py`, and new CI jobs.

## Verification

1. **Desktop regression:**
   - `BUILD.bat`/`PLAY.bat` still work, and the `release.yml` build passes.
   - `cargo test --workspace` passes.
   - The Python tests in `tools/asset_pipeline/test_*.py` pass.
2. **Builds:** `cargo ndk -t arm64-v8a build -p skate-android --release --no-default-features` and `./gradlew assembleDebug` succeed.
3. **Host unit tests** in `input/tests/`:
   - the Android → `XboxState` mapping (Y inversion, trigger reaching 255, D-pad from hat or keycodes);
   - `ControllerKind::from_android` with the Backbone IDs;
   - an export/import round-trip of the manifest hashes.
4. **Import:** export on the PC, import on the phone. Hash verification and `check_assets` pass for every map, and a corrupted zip is rejected.
5. **Boot:** `adb logcat -s skate3` shows the `REPORT_META` stages through `app_run`. University renders in landscape at 60 fps on the frame-stats overlay. Background and resume work without crashing.
6. **Backbone:**
   - The Controller row shows Backbone One with its vendor and product IDs.
   - B brakes and doesn't exit the app.
   - A full trigger pull grabs.
   - Ollie, kickflip and heelflip flicks, manuals and grabs register.
   - The menus are fully navigable from Start.
   - Unplugging and re-plugging mid-session recovers.
7. **Soak:** 15 minutes on DownTown, watching memory, thermals and frame time.
