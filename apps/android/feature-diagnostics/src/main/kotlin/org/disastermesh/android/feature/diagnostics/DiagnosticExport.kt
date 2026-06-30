package org.disastermesh.android.feature.diagnostics

import android.content.Context
import android.os.Build
import org.disastermesh.android.domain.DiagnosticArchive
import org.disastermesh.android.domain.DiagnosticMetadata
import org.disastermesh.android.domain.RelayDiagnosticsSnapshot

fun diagnosticArchivePreview(): List<String> = DiagnosticArchive.preview()

fun buildDiagnosticArchive(context: Context): ByteArray {
    val packageInfo = context.packageManager.getPackageInfo(context.packageName, 0)
    return DiagnosticArchive.create(
        metadata = DiagnosticMetadata(
            appVersion = packageInfo.versionName ?: "unknown",
            protocolVersion = "1.0",
            dbSchema = 1,
            androidApi = Build.VERSION.SDK_INT,
            deviceManufacturer = Build.MANUFACTURER,
            deviceModel = Build.MODEL,
        ),
        relay = RelayDiagnosticsSnapshot(
            activeLinks = 0,
            encounters = 0,
            bytesSent = 0,
            bytesReceived = 0,
            bundlesCommitted = 0,
            partialTransfers = 0,
            failures = emptyMap(),
        ),
        events = emptyList(),
    )
}
