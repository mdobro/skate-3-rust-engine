package com.skate3.engine

import android.content.Context
import android.hardware.input.InputManager
import android.util.Log
import android.view.InputDevice
import android.view.KeyEvent
import android.view.MotionEvent

/** Tracks gamepads and forwards XInput-style state to native code. Main thread only. */
class GamepadBridge(context: Context) : InputManager.InputDeviceListener {
    private class Pad(
        var keys: Int = 0,
        var hat: Int = 0,
        var lx: Int = 0, var ly: Int = 0, var rx: Int = 0, var ry: Int = 0,
        var lt: Int = 0, var rt: Int = 0,
        var analogTriggers: Boolean = false,
        var l2Key: Boolean = false, var r2Key: Boolean = false,
    ) {
        var sent: IntArray? = null
    }

    private val im = context.getSystemService(Context.INPUT_SERVICE) as InputManager
    private val pads = HashMap<Int, Pad>()
    private var registered = false

    fun start() {
        if (!registered) {
            im.registerInputDeviceListener(this, null)
            registered = true
        }
        for (id in im.inputDeviceIds) addDevice(id)
    }

    fun stop() {
        if (registered) {
            im.unregisterInputDeviceListener(this)
            registered = false
        }
        for (id in pads.keys.toList()) removeDevice(id)
    }

    private fun isPad(d: InputDevice?): Boolean {
        if (d == null || d.isVirtual) return false
        val s = d.sources
        return s and InputDevice.SOURCE_GAMEPAD == InputDevice.SOURCE_GAMEPAD ||
            s and InputDevice.SOURCE_JOYSTICK == InputDevice.SOURCE_JOYSTICK
    }

    private fun addDevice(id: Int) {
        val d = InputDevice.getDevice(id)
        if (!isPad(d) || pads.containsKey(id)) return
        d!!
        val pad = Pad()
        pad.analogTriggers = listOf(
            MotionEvent.AXIS_LTRIGGER, MotionEvent.AXIS_RTRIGGER,
            MotionEvent.AXIS_BRAKE, MotionEvent.AXIS_GAS,
        ).any { d.getMotionRange(it, InputDevice.SOURCE_JOYSTICK) != null }
        pads[id] = pad
        Log.i(TAG, "Controller connected: %s vendor=%04x product=%04x".format(d.name, d.vendorId, d.productId))
        NativeBridge.gamepadConnected(id, d.name ?: "", d.vendorId, d.productId)
        send(id, pad, force = true)
    }

    private fun removeDevice(id: Int) {
        if (pads.remove(id) != null) {
            Log.i(TAG, "Controller disconnected: id=$id")
            NativeBridge.gamepadDisconnected(id)
        }
    }

    override fun onInputDeviceAdded(deviceId: Int) = addDevice(deviceId)

    override fun onInputDeviceRemoved(deviceId: Int) = removeDevice(deviceId)

    override fun onInputDeviceChanged(deviceId: Int) {
        val d = InputDevice.getDevice(deviceId)
        if (isPad(d)) {
            if (!pads.containsKey(deviceId)) addDevice(deviceId)
        } else {
            removeDevice(deviceId)
        }
    }

    private fun send(id: Int, p: Pad, force: Boolean = false) {
        var b = p.keys or p.hat
        val lt = if (!p.analogTriggers && p.l2Key) 255 else p.lt
        val rt = if (!p.analogTriggers && p.r2Key) 255 else p.rt
        val cur = intArrayOf(b, p.lx, p.ly, p.rx, p.ry, lt, rt)
        if (!force && cur.contentEquals(p.sent)) return
        p.sent = cur
        NativeBridge.gamepadState(id, b, p.lx, p.ly, p.rx, p.ry, lt, rt)
    }

    /** Returns true when the event came from a gamepad and was consumed. */
    fun onKey(e: KeyEvent): Boolean {
        val d = e.device
        val fromPad = e.source and InputDevice.SOURCE_GAMEPAD == InputDevice.SOURCE_GAMEPAD ||
            e.source and InputDevice.SOURCE_JOYSTICK == InputDevice.SOURCE_JOYSTICK ||
            (e.source and InputDevice.SOURCE_DPAD == InputDevice.SOURCE_DPAD && isPad(d))
        if (!fromPad) return false
        val id = e.deviceId
        if (!pads.containsKey(id)) addDevice(id)
        val p = pads[id] ?: return false
        val down = e.action == KeyEvent.ACTION_DOWN
        when (e.keyCode) {
            KeyEvent.KEYCODE_BUTTON_L2 -> p.l2Key = down
            KeyEvent.KEYCODE_BUTTON_R2 -> p.r2Key = down
            else -> {
                val bit = GamepadMapping.buttonBit(e.keyCode)
                if (bit != 0) p.keys = if (down) p.keys or bit else p.keys and bit.inv()
            }
        }
        send(id, p)
        return true
    }

    fun onMotion(e: MotionEvent): Boolean {
        val s = e.source
        if (s and InputDevice.SOURCE_JOYSTICK != InputDevice.SOURCE_JOYSTICK &&
            s and InputDevice.SOURCE_GAMEPAD != InputDevice.SOURCE_GAMEPAD
        ) return false
        if (e.action != MotionEvent.ACTION_MOVE) return false
        val id = e.deviceId
        if (!pads.containsKey(id)) addDevice(id)
        val p = pads[id] ?: return false
        p.lx = GamepadMapping.stick(e.getAxisValue(MotionEvent.AXIS_X))
        p.ly = GamepadMapping.stickY(e.getAxisValue(MotionEvent.AXIS_Y))
        p.rx = GamepadMapping.stick(e.getAxisValue(MotionEvent.AXIS_Z))
        p.ry = GamepadMapping.stickY(e.getAxisValue(MotionEvent.AXIS_RZ))
        p.hat = GamepadMapping.hat(e.getAxisValue(MotionEvent.AXIS_HAT_X), e.getAxisValue(MotionEvent.AXIS_HAT_Y))
        p.lt = GamepadMapping.trigger(
            GamepadMapping.triggerMax(e.getAxisValue(MotionEvent.AXIS_LTRIGGER), e.getAxisValue(MotionEvent.AXIS_BRAKE)),
        )
        p.rt = GamepadMapping.trigger(
            GamepadMapping.triggerMax(e.getAxisValue(MotionEvent.AXIS_RTRIGGER), e.getAxisValue(MotionEvent.AXIS_GAS)),
        )
        send(id, p)
        return true
    }

    companion object {
        private const val TAG = "skate3"
    }
}
