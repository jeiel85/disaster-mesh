package org.disastermesh.android.feature.diagnostics

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import org.disastermesh.android.domain.RelayDiagnosticsSnapshot

@Composable
fun RelayDiagnosticsScreen(snapshot: RelayDiagnosticsSnapshot) {
    Column(
        modifier = Modifier.fillMaxSize().padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Text("릴레이 진단 — 식별자와 메시지 내용은 기록하지 않습니다")
        snapshot.redactedLines().forEach { line -> Text(line) }
    }
}
