package org.disastermesh.android.domain

import java.io.ByteArrayOutputStream
import java.io.OutputStream
import java.nio.charset.StandardCharsets
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream

data class DiagnosticMetadata(
    val appVersion: String,
    val protocolVersion: String,
    val dbSchema: Int,
    val androidApi: Int,
    val deviceManufacturer: String,
    val deviceModel: String,
)

data class RedactedDiagnosticEvent(
    val createdAtMs: Long,
    val category: Int,
    val severity: Int,
    val numericValue: Long?,
    val detailCode: Int?,
)

object DiagnosticArchive {
    private val entries = listOf("README.txt", "metadata.json", "relay.txt", "events.csv")

    fun preview(): List<String> = entries.toList()

    fun create(
        metadata: DiagnosticMetadata,
        relay: RelayDiagnosticsSnapshot,
        events: List<RedactedDiagnosticEvent>,
    ): ByteArray = ByteArrayOutputStream().use { output ->
        write(output, metadata, relay, events)
        output.toByteArray().also { require(it.size <= 1_048_576) }
    }

    fun write(
        output: OutputStream,
        metadata: DiagnosticMetadata,
        relay: RelayDiagnosticsSnapshot,
        events: List<RedactedDiagnosticEvent>,
    ) {
        require(events.size <= 1_000)
        ZipOutputStream(output, StandardCharsets.UTF_8).use { zip ->
            zip.entry(
                "README.txt",
                "사용자가 선택해 생성한 제한된 진단입니다. 앱/OS/기기 모델과 집계 상태를 포함하며 " +
                    "메시지, 위치, 연락처, 키, DB는 포함하지 않습니다.\n",
            )
            zip.entry("metadata.json", metadataJson(metadata))
            zip.entry("relay.txt", relay.redactedLines().joinToString("\n", postfix = "\n"))
            zip.entry("events.csv", buildString {
                appendLine("created_at_ms,category,severity,numeric_value,detail_code")
                events.forEach { event ->
                    require(event.createdAtMs >= 0 && event.category >= 0 && event.severity in 0..3)
                    append(event.createdAtMs).append(',').append(event.category).append(',')
                        .append(event.severity).append(',').append(event.numericValue ?: "")
                        .append(',').append(event.detailCode ?: "").appendLine()
                }
            })
        }
    }

    private fun metadataJson(value: DiagnosticMetadata): String = """
        {"app_version":"${escape(value.appVersion)}","protocol_version":"${escape(value.protocolVersion)}","db_schema":${value.dbSchema},"android_api":${value.androidApi},"device_manufacturer":"${escape(value.deviceManufacturer)}","device_model":"${escape(value.deviceModel)}"}
    """.trimIndent() + "\n"

    private fun escape(value: String): String = buildString {
        value.take(128).forEach { character ->
            when (character) {
                '\\' -> append("\\\\")
                '"' -> append("\\\"")
                in '\u0000'..'\u001f' -> append('?')
                else -> append(character)
            }
        }
    }

    private fun ZipOutputStream.entry(name: String, content: String) {
        putNextEntry(ZipEntry(name).apply { time = 0L })
        write(content.toByteArray(StandardCharsets.UTF_8))
        closeEntry()
    }
}
