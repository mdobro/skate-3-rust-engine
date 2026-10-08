#![cfg(target_os = "android")]
//! Android entry point: `GameActivity` loads this library and calls `android_main`.
mod gamepad;
use jni::{
    JNIEnv,
    objects::{JClass, JString},
    sys::jstring,
};
use skate_game::{Config, Launch, platform_paths::PlatformPaths};
use std::path::PathBuf;

#[bevy::prelude::bevy_main]
fn main() {
    let Some(files) = bevy::android::ANDROID_APP.get().and_then(|app| app.external_data_path()) else {
        return;
    };
    let paths = PlatformPaths::android(files).install().clone();
    let _ = std::fs::create_dir_all(&paths.temp_dir);
    let _log = match skate_game::init_logging() {
        Ok(guard) => guard,
        Err(_) => return,
    };
    match Config::for_android() {
        Ok(config) => {
            skate_game::run(Launch { config, paths });
        }
        Err(error) => bevy::log::error!("{error}"),
    }
}

fn reply(env: &mut JNIEnv, message: String) -> jstring {
    env.new_string(message).map_or(std::ptr::null_mut(), |s| s.into_raw())
}

fn path_arg(env: &mut JNIEnv, root: &JString) -> Result<PathBuf, String> {
    env.get_string(root).map(|s| PathBuf::from(String::from(s))).map_err(|e| e.to_string())
}

/// Returns "" when the imported installation is usable, else the error text.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_skate3_engine_NativeBridge_checkAssets<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    install_root: JString<'l>,
) -> jstring {
    let result = path_arg(&mut env, &install_root)
        .and_then(|root| skate_game::check_assets(&root.join("assets"), None));
    reply(&mut env, result.err().unwrap_or_default())
}

/// Returns "" when every file in `android-manifest.json` is present and intact.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_skate3_engine_NativeBridge_verifyImport<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    install_root: JString<'l>,
) -> jstring {
    let result = path_arg(&mut env, &install_root).and_then(|root| {
        let summary = skate_data::android_import::verify_installation(&root)?;
        if summary.ok() {
            return Ok(());
        }
        let list = |label: &str, names: &[String]| {
            (!names.is_empty()).then(|| {
                let shown: Vec<&str> = names.iter().take(5).map(String::as_str).collect();
                format!("{label} ({}): {}", names.len(), shown.join(", "))
            })
        };
        let parts: Vec<String> = [list("missing", &summary.missing), list("mismatched", &summary.mismatched)]
            .into_iter().flatten().collect();
        Err(parts.join("; "))
    });
    reply(&mut env, result.err().unwrap_or_default())
}
