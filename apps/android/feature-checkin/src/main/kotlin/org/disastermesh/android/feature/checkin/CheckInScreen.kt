package org.disastermesh.android.feature.checkin

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp

@Composable
fun CheckInScreen(
    recipientName: String,
    onBack: () -> Unit,
    onSend: (status: Int, people: Int, note: String, manualLocation: String?) -> Unit,
) {
    var status by remember { mutableStateOf(1) }
    var people by remember { mutableStateOf("1") }
    var note by remember { mutableStateOf("") }
    var location by remember { mutableStateOf("") }
    Column(Modifier.fillMaxSize().padding(24.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
        Text("$recipientName 님에게 안전 상태 알리기")
        Text("전달은 보장되지 않으며 확인 영수증이 돌아오기 전에는 완료가 아닙니다.")
        Button(onClick = { status = status % 5 + 1 }) { Text("상태 $status (눌러 변경)") }
        OutlinedTextField(people, { people = it.filter(Char::isDigit).take(2) }, label = { Text("인원 수") })
        OutlinedTextField(note, { note = it.take(500) }, label = { Text("메모 (선택)") })
        OutlinedTextField(location, { location = it.take(200) }, label = { Text("수동 위치 설명 (선택)") })
        Text("위치 권한 없이 보낼 수 있습니다.")
        Button(
            onClick = { onSend(status, people.toIntOrNull() ?: 1, note, location.ifBlank { null }) },
            modifier = Modifier.semantics { contentDescription = "안전 상태를 암호화하여 저장" },
        ) { Text("안전 상태 저장") }
        Button(onClick = onBack) { Text("뒤로") }
    }
}
