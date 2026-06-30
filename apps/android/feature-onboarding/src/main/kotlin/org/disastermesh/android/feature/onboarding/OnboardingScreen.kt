package org.disastermesh.android.feature.onboarding

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
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
    Column(
        modifier = Modifier.fillMaxSize().padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Text("DisasterMesh 시작", style = MaterialTheme.typography.headlineMedium)
        Text("인터넷 없이 주변 Android 기기를 통해 메시지를 보관·운반·전달합니다.")
        Text("전달은 보장되지 않으며 공식 긴급 구조 서비스가 아닙니다.")
        Text("장기 수신 키가 유출되면 저장된 과거 암호문이 위험할 수 있습니다.")
        Text(
            if (communicationReady) {
                "Bluetooth 권한과 상태가 준비되었습니다."
            } else {
                "통신 기능 중지됨: Bluetooth와 필수 권한을 확인하세요. 메시지 작성과 연락처 관리는 계속할 수 있습니다."
            },
        )
        if (!communicationReady) {
            Button(onClick = onRequestPermissions) { Text("Bluetooth 권한 요청") }
        }
        Button(onClick = onContinue) { Text("한계를 이해하고 계속") }
    }
}
