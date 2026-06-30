package org.disastermesh.android.transport

enum class CorrelationViolation {
    COMMAND_ID_REUSED,
    LINK_BUSY,
    NO_IN_FLIGHT_COMMAND,
    CALLBACK_COMMAND_MISMATCH,
}

sealed interface BeginCommandResult {
    data object Accepted : BeginCommandResult
    data class Violation(val reason: CorrelationViolation) : BeginCommandResult
}

sealed interface CompleteCommandResult {
    data object Completed : CompleteCommandResult
    data class Violation(val reason: CorrelationViolation) : CompleteCommandResult
}

/** Pure fake-adapter model for one in-flight GATT operation per link. */
class GattCommandCorrelation {
    private val inFlightByLink = mutableMapOf<Long, Long>()
    private val issuedCommandIds = mutableSetOf<Long>()

    @Synchronized
    fun begin(commandId: Long, linkId: Long): BeginCommandResult {
        if (!issuedCommandIds.add(commandId)) {
            return BeginCommandResult.Violation(CorrelationViolation.COMMAND_ID_REUSED)
        }
        if (inFlightByLink.putIfAbsent(linkId, commandId) != null) {
            return BeginCommandResult.Violation(CorrelationViolation.LINK_BUSY)
        }
        return BeginCommandResult.Accepted
    }

    @Synchronized
    fun complete(commandId: Long, linkId: Long): CompleteCommandResult {
        val expected = inFlightByLink[linkId]
            ?: return CompleteCommandResult.Violation(
                CorrelationViolation.NO_IN_FLIGHT_COMMAND,
            )
        if (expected != commandId) {
            inFlightByLink.remove(linkId)
            return CompleteCommandResult.Violation(
                CorrelationViolation.CALLBACK_COMMAND_MISMATCH,
            )
        }
        inFlightByLink.remove(linkId)
        return CompleteCommandResult.Completed
    }

    @Synchronized
    fun failAll(linkId: Long) {
        inFlightByLink.remove(linkId)
    }

    @Synchronized
    fun inFlightCommand(linkId: Long): Long? = inFlightByLink[linkId]
}
