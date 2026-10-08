package com.skate3.engine

import org.junit.Assert.assertEquals
import org.junit.Test

class GamepadMappingTest {
    @Test fun yIsInverted() {
        assertEquals(-32767, GamepadMapping.stickY(1f))
        assertEquals(32767, GamepadMapping.stickY(-1f))
        assertEquals(0, GamepadMapping.stickY(0f))
        assertEquals(32767, GamepadMapping.stick(1f))
        assertEquals(-32767, GamepadMapping.stick(-1f))
    }

    @Test fun stickClamps() {
        assertEquals(32767, GamepadMapping.stick(5f))
        assertEquals(-32768, GamepadMapping.stick(-5f))
    }

    @Test fun triggerReaches255() {
        assertEquals(255, GamepadMapping.trigger(0.98f))
        assertEquals(255, GamepadMapping.trigger(1f))
        assertEquals(0, GamepadMapping.trigger(0f))
        assertEquals(128, GamepadMapping.trigger(0.5f))
        assertEquals(249, GamepadMapping.trigger(0.976f))
        assertEquals(0, GamepadMapping.trigger(-0.2f))
    }

    @Test fun hatToDpad() {
        assertEquals(0, GamepadMapping.hat(0f, 0f))
        assertEquals(0x1, GamepadMapping.hat(0f, -1f))
        assertEquals(0x2, GamepadMapping.hat(0f, 1f))
        assertEquals(0x4, GamepadMapping.hat(-1f, 0f))
        assertEquals(0x8, GamepadMapping.hat(1f, 0f))
        assertEquals(0x5, GamepadMapping.hat(-1f, -1f))
        assertEquals(0, GamepadMapping.hat(0.4f, -0.4f))
    }

    @Test fun buttonTable() {
        assertEquals(0x1000, GamepadMapping.buttonBit(96))
        assertEquals(0x2000, GamepadMapping.buttonBit(97))
        assertEquals(0x4000, GamepadMapping.buttonBit(99))
        assertEquals(0x8000, GamepadMapping.buttonBit(100))
        assertEquals(0x100, GamepadMapping.buttonBit(102))
        assertEquals(0x200, GamepadMapping.buttonBit(103))
        assertEquals(0x40, GamepadMapping.buttonBit(106))
        assertEquals(0x80, GamepadMapping.buttonBit(107))
        assertEquals(0x10, GamepadMapping.buttonBit(108))
        assertEquals(0x20, GamepadMapping.buttonBit(109))
        assertEquals(0x1, GamepadMapping.buttonBit(19))
        assertEquals(0x2, GamepadMapping.buttonBit(20))
        assertEquals(0x4, GamepadMapping.buttonBit(21))
        assertEquals(0x8, GamepadMapping.buttonBit(22))
        assertEquals(0, GamepadMapping.buttonBit(110))
        assertEquals(0, GamepadMapping.buttonBit(23))
    }
}
