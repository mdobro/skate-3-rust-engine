# Android app (Kotlin side)

The Gradle project lives in `android/` (package `com.skate3.engine`, minSdk 29, targetSdk 35, arm64-v8a only).

## Build

```
export ANDROID_HOME=/opt/android-sdk
cd android
./gradlew assembleDebug                      # builds Rust via cargo-ndk, then the APK
./gradlew assembleDebug -PskipCargo=true     # Kotlin side only
./gradlew testDebugUnitTest -PskipCargo=true
```

`cargoNdkBuildDebug` and `cargoNdkBuildRelease` run from the repo root:
`cargo ndk -t arm64-v8a --platform 29 --link-libcxx-shared -o android/app/src/main/jniLibs build -p skate-android --no-default-features [--release]`.
They run before `preDebugBuild` and `preReleaseBuild`, and put `libskate_android.so` and `libc++_shared.so` in `jniLibs/arm64-v8a/`.

Release signing reads `SKATE_KEYSTORE`, `SKATE_KEYSTORE_PASSWORD`, `SKATE_KEY_ALIAS`, `SKATE_KEY_PASSWORD` from Gradle properties or the environment. Without `SKATE_KEYSTORE` the release APK is unsigned. `versionCode` is `git rev-list --count HEAD` and `versionName` is `0.1.0-<short sha>`.

`androidx.games:games-activity` is pinned to exactly 4.4.0, which the `android-activity` 0.6.1 crate requires.

## Activities

- `LauncherActivity`: shows whether `<getExternalFilesDir(null)>/installation/` has data, imports a zip through the system picker, and starts the game.
- `SkateActivity` (GameActivity, `android.app.lib_name = skate_android`): immersive landscape, owns `GamepadBridge`.

Import streams `installation/*` entries of the zip into `<files>/installation.new/` (with path traversal checks and a free space check of 1.05x the zip size), calls `verifyImport` and `checkAssets` on it, then swaps it in as `installation/`.

## JNI contract

All functions are `@JvmStatic external` on `object com.skate3.engine.NativeBridge`, so the Rust symbols are `Java_com_skate3_engine_NativeBridge_<name>`. The library is `libskate_android.so`, loaded with `System.loadLibrary("skate_android")` in the object initializer. `checkAssets` and `verifyImport` are called from a background thread; the gamepad calls come from the main thread.

| Kotlin | Notes |
|---|---|
| `checkAssets(installRoot: String): String` | `""` on success, else error text. `installRoot` is the `installation` folder (contains `android-manifest.json`, `assets/`, `maps/`). |
| `verifyImport(installRoot: String): String` | `""` on success, else error text. Checks `android-manifest.json` sizes and SHA-256. |
| `gamepadConnected(deviceId: Int, name: String, vendor: Int, product: Int)` | Sent once per device per `onResume` and when a device appears. |
| `gamepadDisconnected(deviceId: Int)` | Sent on removal and on `onPause` for every tracked device. |
| `gamepadState(deviceId, buttons, lx, ly, rx, ry, lt, rt: Int)` | Only sent when the state changed, and once right after connect. |

`buttons` uses XInput bits: DPAD_UP 0x1, DPAD_DOWN 0x2, DPAD_LEFT 0x4, DPAD_RIGHT 0x8, START 0x10, BACK 0x20, LS 0x40, RS 0x80, LB 0x100, RB 0x200, A 0x1000, B 0x2000, X 0x4000, Y 0x8000.

Sticks are i16 values (-32768..32767) in XInput convention (Y up is positive, Kotlin negates Android's Y). Triggers are 0..255, exactly 255 from a 0.98 pull. Kotlin applies no deadzone. Digital L2/R2 keys give 255 only when the device has no analog trigger axis. The Backbone/home button (`BUTTON_MODE`) is ignored.

The Rust side should treat `deviceId` as an opaque Android device id (not a slot index), drop state for unknown ids or after disconnect, and bump its packet number on each `gamepadState`. Gamepad key and motion events are consumed in `SkateActivity`, so B never becomes BACK.

Logcat tag is `skate3`. Each device logs `Controller connected: <name> vendor=%04x product=%04x`.

## Window, lifecycle and present mode (Phase 2)

All of this is `cfg(target_os = "android")`; desktop is unchanged.

- No fixed 1280x800 window; the surface decides the size. `graphics_menu` no longer applies the saved width/height to the window, the Resolution row is read-only (shows the surface size), and the Alt+Enter fullscreen toggle is not run.
- Present mode is `AutoVsync` (Fifo).
- `WinitSettings.unfocused_mode` is `reactive_low_power(1 s)`.
- `android_lifecycle.rs` reads `AppLifecycle`: on `WillSuspend`/`Suspended` it pauses every `AudioSink` and `Time<Virtual>` (only if it was not already paused), and sets `Suspended`, which `game_audio::native::follow_volume` ORs into `silenced` so the native AEMS stream stays paused. `WillResume`/`Running` undoes it.
- The Gradle cargo task sets `CARGO_INCREMENTAL=0` and `CARGO_PROFILE_{DEV,RELEASE}_DEBUG=0` so the `.so` has no debuginfo. `Cargo.toml` profiles are untouched.

## Phase 3 (Rust input)

`skate_game::android_input` (`input/platform.rs`, `mod android`) mirrors the SDL backend's slot table, and `crates/skate-android/src/gamepad.rs` holds the three JNI exports.

- `gamepadConnected` takes the first free of four slots (connect order, no player-index preference), keyed by the opaque Android device id. A known id is ignored, a fifth pad is logged and dropped. The log line is `Controller N: identified as <summary>`.
- `gamepadDisconnected` frees the slot, so the next pad reuses it. `gamepadState` for an unknown id is ignored; the packet number advances only when the state changes.
- State is clamped by `android_state` (sticks to `i16`, triggers to `0..=255`); Y is already up-positive from Kotlin. Slots report XInput subtype 1 like SDL pads, so gameplay, customiser and the Esc menu read them through the same `poll`.
- Identity is `ControllerKind::from_android`: backend `android`, driver `android#<id>`, model table and `settings/controller.json` `models` first. Vendor `0x358a` (Backbone Labs) with any product id is `Backbone One`, family Xbox One / Series (Xbox prompts). Unknown pads are `Standard` and keep the reported name.

## Smoke test

```
scripts/android-smoke.sh                       # build APK; with a device: install, launch, tail logcat (skate3, RustStdoutStderr)
scripts/android-smoke.sh push-data <dir>       # adb push <dir>/. to /sdcard/Android/data/com.skate3.engine/files/installation/
```

## Status

Implemented: Rust game on GameActivity, zip import with hash and asset checks, launcher with mods folder and last crash report, lifecycle pause/resume, Android gamepad bridge with Backbone identity, "Connect your controller" overlay (`controller_prompt.rs`, shown while no slot is ready), PC export, CI APK build.

Verified here (no device): `cargo check -p skate-game` (desktop), `input::` tests, `skate-data` `android_import` tests, `tools.test_export_android`, `./gradlew assembleDebug testDebugUnitTest` with the APK holding `libskate_android.so` and `libc++_shared.so` and exporting `android_main`, `GameActivity_onCreate` and the five `NativeBridge` JNI symbols.

Not verified: anything running on hardware.

### On-device checklist (Galaxy S26 Ultra + Backbone)

1. Boot: install, import the zip, Play. `adb logcat -s skate3` shows `REPORT_META` stages through `app_run`. University renders in landscape at 60 fps (frame-stats overlay). Background and resume do not crash.
2. Without a pad the "Connect your controller" text shows and disappears when the pad connects.
3. Backbone: Esc menu Controller row shows Backbone One with vendor/product IDs; B brakes and does not exit; a full trigger pull grabs; ollie, kickflip, heelflip flicks, manuals and grabs register; menus navigable from Start; unplug and re-plug mid-session recovers.
4. Import: corrupted zip is rejected; every map passes verification.
5. Mods: a Lua mod in the shown mods folder loads.
6. Soak: 15 minutes on DownTown, watching memory, thermals and frame time.
