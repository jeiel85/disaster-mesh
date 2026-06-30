package org.disastermesh.android.feature.onboarding

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
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

@Composable
fun OnboardingScreen(
    communicationReady: Boolean,
    onRequestPermissions: () -> Unit,
    onContinue: () -> Unit,
) {
    Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 20.dp, vertical = 28.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Text("DisasterMesh 시작", style = MaterialTheme.typography.headlineLarge)
            Text(
                "인터넷 없이 주변 Android 기기를 통해 메시지를 보관·운반·전달합니다.",
                style = MaterialTheme.typography.bodyLarge,
            )
            Card(modifier = Modifier.fillMaxWidth()) {
                Column(
                    modifier = Modifier.padding(18.dp),
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    Text("시작 전에 꼭 확인하세요", style = MaterialTheme.typography.titleMedium)
                    Text("• 전달은 보장되지 않으며 공식 긴급 구조 서비스가 아닙니다.")
                    Text("• 연락처는 만나서 QR 문자열을 교환한 뒤에만 추가할 수 있습니다.")
                    Text("• 장기 수신 키가 유출되면 저장된 과거 암호문이 위험할 수 있습니다.")
                }
            }
            Text(
                text = if (communicationReady) {
                    "Bluetooth 권한과 상태가 준비되었습니다."
                } else {
                    "통신 기능 중지됨: Bluetooth와 필수 권한을 확인하세요. 메시지 작성과 연락처 관리는 계속할 수 있습니다."
                },
                color = if (communicationReady) {
                    MaterialTheme.colorScheme.primary
                } else {
                    MaterialTheme.colorScheme.error
                },
            )
            if (!communicationReady) {
                Button(onClick = onRequestPermissions, modifier = Modifier.fillMaxWidth()) {
                    Text("Bluetooth 권한 요청")
                }
            }
            Button(onClick = onContinue, modifier = Modifier.fillMaxWidth()) {
                Text("한계를 이해하고 계속")
            }
        }
    }
}
