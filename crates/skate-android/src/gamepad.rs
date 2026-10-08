//! JNI gamepad entry points. `GamepadBridge` calls these from the main thread;
//! they feed `skate_game::android_input`, which polls like the SDL backend.
use jni::{
    JNIEnv,
    objects::{JClass, JString},
    sys::jint,
};
use skate_game::android_input;
use std::panic::{AssertUnwindSafe, catch_unwind};

fn id16(value: jint) -> u16 {
    u16::try_from(value).unwrap_or(0)
}

/// A panic must never unwind into the JVM.
fn guarded(f: impl FnOnce()) {
    let _ = catch_unwind(AssertUnwindSafe(f));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_skate3_engine_NativeBridge_gamepadConnected<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    device_id: jint,
    name: JString<'l>,
    vendor: jint,
    product: jint,
) {
    let name = env.get_string(&name).map(String::from).unwrap_or_default();
    guarded(|| android_input::connected(device_id, &name, id16(vendor), id16(product)));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_skate3_engine_NativeBridge_gamepadDisconnected<'l>(
    _env: JNIEnv<'l>,
    _class: JClass<'l>,
    device_id: jint,
) {
    guarded(|| android_input::disconnected(device_id));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_skate3_engine_NativeBridge_gamepadState<'l>(
    _env: JNIEnv<'l>,
    _class: JClass<'l>,
    device_id: jint,
    buttons: jint,
    lx: jint,
    ly: jint,
    rx: jint,
    ry: jint,
    lt: jint,
    rt: jint,
) {
    let axis = |v: jint| v.clamp(i16::MIN.into(), i16::MAX.into()) as i16;
    let trigger = |v: jint| v.clamp(0, 255) as u8;
    guarded(|| {
        android_input::state(
            device_id,
            (buttons & 0xffff) as u16,
            axis(lx),
            axis(ly),
            axis(rx),
            axis(ry),
            trigger(lt),
            trigger(rt),
        )
    });
}
