package org.disastermesh.android.feature.settings

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp

data class AppInfoUiModel(
    val appVersion: String,
    val communicationReady: Boolean,
    val engineReady: Boolean?,
    val contactCount: Int,
)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(
    model: AppInfoUiModel,
    onRequestPermissions: () -> Unit,
    onReviewSafety: () -> Unit,
    onBack: () -> Unit,
) {
    Scaffold(
        topBar = { TopAppBar(title = { Text("앱 정보 및 설정") }) },
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 20.dp, vertical = 12.dp),
            verticalArrangement = Arrangement.spacedBy(14.dp),
        ) {
            StatusCard(
                title = "현재 상태",
                lines = listOf(
                    if (model.communicationReady) "Bluetooth 통신 준비됨" else "Bluetooth 통신 중지됨",
                    when (model.engineReady) {
                        null -> "로컬 암호화 저장소 여는 중"
                        true -> "로컬 암호화 저장소 준비됨"
                        false -> "로컬 암호화 저장소 점검 필요"
                    },
                    "신뢰 연락처 ${model.contactCount}개",
                ),
            )
            StatusCard(
                title = "버전",
                lines = listOf(
                    "DisasterMesh ${model.appVersion}",
                    "DME v1 · BLE-CLA v1 · DB schema 1",
                    "Android offline edition",
                ),
            )
            StatusCard(
                title = "개인정보와 안전",
                lines = listOf(
                    "계정, 광고, 분석 SDK와 인터넷 권한을 사용하지 않습니다.",
                    "전달 성공과 공식 긴급 구조 접수는 보장되지 않습니다.",
                    "제한된 진단은 사용자가 선택할 때만 파일로 저장됩니다.",
                ),
            )
            if (!model.communicationReady) {
                Button(onClick = onRequestPermissions, modifier = Modifier.fillMaxWidth()) {
                    Text("Bluetooth 권한 확인")
                }
            }
            OutlinedButton(onClick = onReviewSafety, modifier = Modifier.fillMaxWidth()) {
                Text("안전 고지 다시 보기")
            }
            OutlinedButton(onClick = onBack, modifier = Modifier.fillMaxWidth()) {
                Text("홈으로 돌아가기")
            }
        }
    }
}

@Composable
private fun StatusCard(title: String, lines: List<String>) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(
            modifier = Modifier.padding(18.dp),
            verticalArrangement = Arrangement.spacedBy(7.dp),
        ) {
            Text(title, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
            lines.forEach { line ->
                Text(
                    text = line,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}
