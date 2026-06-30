package org.disastermesh.android.domain

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class RelayDutyPolicyTest {
    @Test
    fun normalModesMatchNormativeDutyCycles() {
        assertEquals(10 to 50, RelayDutyPolicy.evaluate(state(RelayMode.STANDBY)).let { it.scanSeconds to it.sleepSeconds })
        assertEquals(20 to 10, RelayDutyPolicy.evaluate(state(RelayMode.EMERGENCY)).let { it.scanSeconds to it.sleepSeconds })
        assertEquals(55 to 5, RelayDutyPolicy.evaluate(state(RelayMode.FIXED, charging = true)).let { it.scanSeconds to it.sleepSeconds })
    }

    @Test
    fun criticalBatteryThermalAndStorageFailSafe() {
        assertEquals(RelayScope.OWN_P0_AND_DIRECT, RelayDutyPolicy.evaluate(state(battery = 9)).scope)
        assertEquals(RelayScope.DIRECT_ONLY, RelayDutyPolicy.evaluate(state(storageLow = true)).scope)
        val thermal = RelayDutyPolicy.evaluate(state(thermal = true))
        assertEquals(RelayScope.PAUSED, thermal.scope)
        assertTrue(thermal.finishCurrentThenPause)
        assertEquals(300, thermal.sleepSeconds)
    }

    private fun state(
        mode: RelayMode = RelayMode.STANDBY,
        battery: Int = 80,
        charging: Boolean = false,
        thermal: Boolean = false,
        storageLow: Boolean = false,
    ) = RelayDeviceState(mode, battery, charging, thermal, storageLow)
}
