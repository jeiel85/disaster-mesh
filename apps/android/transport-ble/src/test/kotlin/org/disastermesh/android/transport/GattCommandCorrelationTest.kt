package org.disastermesh.android.transport

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class GattCommandCorrelationTest {
    @Test
    fun `callback completes exactly the matching command`() {
        val model = GattCommandCorrelation()
        assertEquals(BeginCommandResult.Accepted, model.begin(commandId = 7, linkId = 2))
        assertEquals(CompleteCommandResult.Completed, model.complete(commandId = 7, linkId = 2))
        assertNull(model.inFlightCommand(2))
    }

    @Test
    fun `second operation on one link is rejected`() {
        val model = GattCommandCorrelation()
        model.begin(commandId = 1, linkId = 9)
        assertEquals(
            BeginCommandResult.Violation(CorrelationViolation.LINK_BUSY),
            model.begin(commandId = 2, linkId = 9),
        )
    }

    @Test
    fun `mismatched and double callbacks are violations`() {
        val model = GattCommandCorrelation()
        model.begin(commandId = 4, linkId = 5)
        assertEquals(
            CompleteCommandResult.Violation(
                CorrelationViolation.CALLBACK_COMMAND_MISMATCH,
            ),
            model.complete(commandId = 6, linkId = 5),
        )
        assertEquals(
            CompleteCommandResult.Violation(CorrelationViolation.NO_IN_FLIGHT_COMMAND),
            model.complete(commandId = 4, linkId = 5),
        )
    }

    @Test
    fun `command ids are never reused`() {
        val model = GattCommandCorrelation()
        model.begin(commandId = 11, linkId = 1)
        model.complete(commandId = 11, linkId = 1)
        assertEquals(
            BeginCommandResult.Violation(CorrelationViolation.COMMAND_ID_REUSED),
            model.begin(commandId = 11, linkId = 2),
        )
    }
}
