package org.disastermesh.android.feature.relay

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

enum class RelayModeOption { STANDBY, EMERGENCY, FIXED }

@Composable
fun RelayStatusScreen(onStart: (RelayModeOption) -> Unit, onStop: () -> Unit, onBack: () -> Unit) {
    Column(Modifier.fillMaxSize().padding(24.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text("릴레이 모드")
        Text("지속 알림이 보이는 동안만 백그라운드 접촉을 시도합니다. OS·전원·주변 기기에 따라 전달은 보장되지 않습니다.")
        Button(onClick = { onStart(RelayModeOption.STANDBY) }) { Text("대기 모드 시작") }
        Button(onClick = { onStart(RelayModeOption.EMERGENCY) }) { Text("비상 모드 시작") }
        Button(onClick = { onStart(RelayModeOption.FIXED) }) { Text("충전 중 고정 릴레이 시작") }
        Button(onClick = onStop) { Text("릴레이 중지") }
        Button(onClick = onBack) { Text("뒤로") }
    }
}
