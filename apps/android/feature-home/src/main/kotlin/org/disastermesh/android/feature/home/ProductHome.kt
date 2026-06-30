package org.disastermesh.android.feature.home

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp

@Composable
fun ProductHome(
    contactReady: Boolean,
    onContacts: () -> Unit,
    onCheckIn: () -> Unit,
    onSos: () -> Unit,
    onRelay: () -> Unit,
    onDiagnostics: () -> Unit,
) {
    Column(
        modifier = Modifier.fillMaxSize().padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        Text("DisasterMesh")
        Text("Bluetooth 접촉 기회에 저장·운반·전달합니다. 전달 시점이나 성공은 보장되지 않습니다.")
        Button(onClick = onContacts) { Text("신뢰할 연락처 관리") }
        Button(onClick = onCheckIn, enabled = contactReady) { Text("안전 상태 알리기") }
        Button(
            onClick = onSos,
            enabled = contactReady,
            modifier = Modifier.semantics { contentDescription = "선택한 연락처에게 비공개 구조 요청 작성" },
        ) { Text("비공개 SOS") }
        Button(onClick = onRelay) { Text("릴레이 모드") }
        Button(onClick = onDiagnostics) { Text("제한된 진단 내보내기") }
    }
}
