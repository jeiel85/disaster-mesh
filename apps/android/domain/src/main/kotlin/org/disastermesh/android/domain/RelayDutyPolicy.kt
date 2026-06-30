package org.disastermesh.android.domain

enum class RelayMode { STANDBY, EMERGENCY, FIXED }

enum class RelayScope { ALL, P0_P1_P2, P0_P1, OWN_P0_AND_DIRECT, DIRECT_ONLY, PAUSED }

data class RelayDeviceState(
    val mode: RelayMode,
    val batteryPercent: Int,
    val charging: Boolean,
    val thermalSevere: Boolean,
    val storageLow: Boolean,
)

data class RelayDutyPlan(
    val scanSeconds: Int,
    val sleepSeconds: Int,
    val advertiseSparse: Boolean,
    val scope: RelayScope,
    val finishCurrentThenPause: Boolean,
    val reason: String,
)

object RelayDutyPolicy {
    fun evaluate(state: RelayDeviceState): RelayDutyPlan {
        require(state.batteryPercent in 0..100)
        if (state.thermalSevere) {
            return RelayDutyPlan(0, 300, true, RelayScope.PAUSED, true, "thermal_severe")
        }
        if (state.storageLow) {
            return RelayDutyPlan(5, 300, true, RelayScope.DIRECT_ONLY, false, "storage_low")
        }
        if (state.batteryPercent < 10) {
            return RelayDutyPlan(5, 300, true, RelayScope.OWN_P0_AND_DIRECT, false, "battery_critical")
        }
        val base = when (state.mode) {
            RelayMode.STANDBY -> RelayDutyPlan(10, 50, false, RelayScope.P0_P1_P2, false, "standby")
            RelayMode.EMERGENCY -> RelayDutyPlan(20, 10, false, RelayScope.ALL, false, "emergency")
            RelayMode.FIXED -> if (state.charging) {
                RelayDutyPlan(55, 5, false, RelayScope.ALL, false, "fixed_charging")
            } else {
                RelayDutyPlan(20, 10, false, RelayScope.ALL, false, "fixed_not_charging")
            }
        }
        return if (state.batteryPercent < 20) {
            base.copy(
                scanSeconds = maxOf(5, base.scanSeconds / 2),
                sleepSeconds = base.sleepSeconds + base.scanSeconds / 2,
                scope = RelayScope.P0_P1,
                reason = "battery_low",
            )
        } else {
            base
        }
    }
}
