package org.disastermesh.android.domain

import java.io.ByteArrayInputStream
import java.util.zip.ZipInputStream
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Test

class DiagnosticArchiveTest {
    @Test
    fun archiveHasFixedEntriesAndCannotCarryIdentifiersOrMessageContent() {
        val archive = DiagnosticArchive.create(
            DiagnosticMetadata("0.1", "1.0", 1, 36, "vendor", "model"),
            RelayDiagnosticsSnapshot(0, 1, 2, 3, 4, 0, emptyMap()),
            listOf(RedactedDiagnosticEvent(1, 2, 1, null, 3)),
        )
        val contents = linkedMapOf<String, String>()
        ZipInputStream(ByteArrayInputStream(archive)).use { zip ->
            while (true) {
                val entry = zip.nextEntry ?: break
                contents[entry.name] = zip.readBytes().decodeToString()
            }
        }
        assertEquals(DiagnosticArchive.preview().toSet(), contents.keys)
        val all = contents.values.joinToString("\n").lowercase()
        for (forbidden in listOf("private_key", "safety_number", "packet_id", "message_body", "latitude", "longitude")) {
            assertFalse(all.contains(forbidden))
        }
    }
}
