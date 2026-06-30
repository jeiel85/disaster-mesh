package org.disastermesh.android.feature.diagnostics

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

@Composable
fun DiagnosticExportScreen(onExport: () -> Unit, onBack: () -> Unit) {
    Column(Modifier.fillMaxSize().padding(24.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
        Text("제한된 진단 ZIP 미리보기")
        diagnosticArchivePreview().forEach { Text("• $it") }
        Text("메시지 본문, 위치, 연락처, 안전번호, 키, 데이터베이스와 peer/packet 식별자는 포함하지 않습니다.")
        Button(onClick = onExport) { Text("저장 위치 선택") }
        Button(onClick = onBack) { Text("뒤로") }
    }
}
