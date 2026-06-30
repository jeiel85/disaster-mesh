package org.disastermesh.android.domain

import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Test

class RelayDiagnosticsTest {
    @Test
    fun exportSurfaceContainsOnlyAggregateValues() {
        val lines = RelayDiagnosticsSnapshot(
            activeLinks = 1,
            encounters = 2,
            bytesSent = 3,
            bytesReceived = 4,
            bundlesCommitted = 5,
            partialTransfers = 6,
            failures = mapOf(RelayFailureCategory.QUOTA to 7),
        ).redactedLines().joinToString("\n")
        for (forbidden in listOf("peer", "packet", "contact", "message", "payload")) {
            assertFalse(lines.lowercase().contains(forbidden))
        }
    }

    @Test
    fun negativeCountersAreRejected() {
        assertThrows(IllegalArgumentException::class.java) {
            RelayDiagnosticsSnapshot(-1, 0, 0, 0, 0, 0, emptyMap())
        }
    }
}
