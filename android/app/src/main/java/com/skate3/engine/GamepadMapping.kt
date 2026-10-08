package com.skate3.engine

import kotlin.math.max
import kotlin.math.roundToInt

/** Pure Android-to-XInput mapping math. Key codes are literal so JVM tests need no framework. */
object GamepadMapping {
    const val DPAD_UP = 0x1
    const val DPAD_DOWN = 0x2
    const val DPAD_LEFT = 0x4
    const val DPAD_RIGHT = 0x8
    const val START = 0x10
    const val BACK = 0x20
    const val LS = 0x40
    const val RS = 0x80
    const val LB = 0x100
    const val RB = 0x200
    const val A = 0x1000
    const val B = 0x2000
    const val X = 0x4000
    const val Y = 0x8000

    // android.view.KeyEvent codes
    const val KEYCODE_DPAD_UP = 19
    const val KEYCODE_DPAD_DOWN = 20
    const val KEYCODE_DPAD_LEFT = 21
    const val KEYCODE_DPAD_RIGHT = 22
    const val KEYCODE_BUTTON_A = 96
    const val KEYCODE_BUTTON_B = 97
    const val KEYCODE_BUTTON_X = 99
    const val KEYCODE_BUTTON_Y = 100
    const val KEYCODE_BUTTON_L1 = 102
    const val KEYCODE_BUTTON_R1 = 103
    const val KEYCODE_BUTTON_L2 = 104
    const val KEYCODE_BUTTON_R2 = 105
    const val KEYCODE_BUTTON_THUMBL = 106
    const val KEYCODE_BUTTON_THUMBR = 107
    const val KEYCODE_BUTTON_START = 108
    const val KEYCODE_BUTTON_SELECT = 109

    /** XInput bit for a key code, or 0 when the key is not a mapped button. */
    fun buttonBit(keyCode: Int): Int = when (keyCode) {
        KEYCODE_BUTTON_A -> A
        KEYCODE_BUTTON_B -> B
        KEYCODE_BUTTON_X -> X
        KEYCODE_BUTTON_Y -> Y
        KEYCODE_BUTTON_L1 -> LB
        KEYCODE_BUTTON_R1 -> RB
        KEYCODE_BUTTON_THUMBL -> LS
        KEYCODE_BUTTON_THUMBR -> RS
        KEYCODE_BUTTON_START -> START
        KEYCODE_BUTTON_SELECT -> BACK
        KEYCODE_DPAD_UP -> DPAD_UP
        KEYCODE_DPAD_DOWN -> DPAD_DOWN
        KEYCODE_DPAD_LEFT -> DPAD_LEFT
        KEYCODE_DPAD_RIGHT -> DPAD_RIGHT
        else -> 0
    }

    /** Stick axis in [-1, 1] to i16 range. */
    fun stick(v: Float): Int = (v * 32767f).roundToInt().coerceIn(-32768, 32767)

    /** Android Y is down-positive; XInput is up-positive. */
    fun stickY(v: Float): Int = stick(-v).coerceIn(-32768, 32767)

    /** Trigger axis in [0, 1] to 0..255, exactly 255 from 0.98. */
    fun trigger(v: Float): Int = if (v >= 0.98f) 255 else (v * 255f).roundToInt().coerceIn(0, 255)

    fun triggerMax(a: Float, b: Float): Float = max(a, b)

    /** HAT_X / HAT_Y to dpad bits, threshold 0.5. */
    fun hat(x: Float, y: Float): Int {
        var bits = 0
        if (x <= -0.5f) bits = bits or DPAD_LEFT
        if (x >= 0.5f) bits = bits or DPAD_RIGHT
        if (y <= -0.5f) bits = bits or DPAD_UP
        if (y >= 0.5f) bits = bits or DPAD_DOWN
        return bits
    }
}
