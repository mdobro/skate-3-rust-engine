package com.skate3.engine

/** JNI entry points implemented by libskate_android.so. See docs/android/android-app.md. */
object NativeBridge {
    init {
        System.loadLibrary("skate_android")
    }

    /** Returns "" when the installation at [installRoot] passes the asset check, else error text. */
    @JvmStatic external fun checkAssets(installRoot: String): String

    /** Returns "" when android-manifest.json hashes match, else error text. */
    @JvmStatic external fun verifyImport(installRoot: String): String

    @JvmStatic external fun gamepadConnected(deviceId: Int, name: String, vendor: Int, product: Int)

    @JvmStatic external fun gamepadDisconnected(deviceId: Int)

    @JvmStatic external fun gamepadState(
        deviceId: Int, buttons: Int, lx: Int, ly: Int, rx: Int, ry: Int, lt: Int, rt: Int,
    )
}
