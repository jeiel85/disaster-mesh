package org.disastermesh.android.feature.home

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp

@Composable
fun ProductHome(
    contactReady: Boolean,
    communicationReady: Boolean,
    engineReady: Boolean?,
    contactCount: Int,
    onContacts: () -> Unit,
    onCheckIn: () -> Unit,
    onSos: () -> Unit,
    onRelay: () -> Unit,
    onDiagnostics: () -> Unit,
    onSettings: () -> Unit,
) {
    Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 20.dp, vertical = 24.dp),
            verticalArrangement = Arrangement.spacedBy(14.dp),
        ) {
            Text(
                text = "DisasterMesh",
                style = MaterialTheme.typography.headlineLarge,
                fontWeight = FontWeight.Bold,
            )
            Text(
                text = "연결이 끊긴 순간에도, 미리 신뢰한 연락처를 향해 암호화 메시지를 보관·운반합니다.",
                style = MaterialTheme.typography.bodyLarge,
            )
            Text(
                text = "전달 시점이나 성공은 보장되지 않으며 공식 긴급 구조 서비스가 아닙니다.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            Card(modifier = Modifier.fillMaxWidth()) {
                Column(
                    modifier = Modifier.padding(18.dp),
                    verticalArrangement = Arrangement.spacedBy(7.dp),
                ) {
                    Text(
                        text = "현재 준비 상태",
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.SemiBold,
                    )
                    StatusLine(
                        ready = communicationReady,
                        readyText = "Bluetooth 통신 준비됨",
                        blockedText = "Bluetooth 통신 중지됨 · 권한과 상태를 확인하세요",
                    )
                    when (engineReady) {
                        null -> Text(
                            text = "… 로컬 암호화 저장소 여는 중",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        else -> StatusLine(
                            ready = engineReady,
                            readyText = "로컬 암호화 저장소 준비됨",
                            blockedText = "로컬 암호화 저장소 점검 필요",
                        )
                    }
                    Text(
                        text = "신뢰 연락처 ${contactCount}개",
                        style = MaterialTheme.typography.bodyMedium,
                    )
                }
            }

            Text("메시지와 안전", style = MaterialTheme.typography.titleMedium)
            Button(onClick = onContacts, modifier = Modifier.fillMaxWidth()) {
                Text("신뢰할 연락처 관리")
            }
            Button(onClick = onCheckIn, enabled = contactReady, modifier = Modifier.fillMaxWidth()) {
                Text("안전 상태 알리기")
            }
            Button(
                onClick = onSos,
                enabled = contactReady,
                modifier = Modifier
                    .fillMaxWidth()
                    .semantics { contentDescription = "선택한 연락처에게 비공개 구조 요청 작성" },
            ) { Text("비공개 SOS") }

            Text("기기와 운영", style = MaterialTheme.typography.titleMedium)
            OutlinedButton(onClick = onRelay, modifier = Modifier.fillMaxWidth()) {
                Text("릴레이 모드")
            }
            OutlinedButton(onClick = onDiagnostics, modifier = Modifier.fillMaxWidth()) {
                Text("제한된 진단 내보내기")
            }
            OutlinedButton(onClick = onSettings, modifier = Modifier.fillMaxWidth()) {
                Text("설정 및 앱 정보")
            }
        }
    }
}

@Composable
private fun StatusLine(ready: Boolean, readyText: String, blockedText: String) {
    Text(
        text = if (ready) "● $readyText" else "○ $blockedText",
        style = MaterialTheme.typography.bodyMedium,
        color = if (ready) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.error,
    )
}
