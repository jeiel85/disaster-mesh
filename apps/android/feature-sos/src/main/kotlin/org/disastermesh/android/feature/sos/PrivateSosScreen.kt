package org.disastermesh.android.feature.sos

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
fun PrivateSosScreen(
    recipientName: String,
    onBack: () -> Unit,
    onSend: (category: Int, description: String, people: Int, severeInjuries: Int, manualLocation: String?, movement: String) -> Unit,
) {
    var category by remember { mutableStateOf(1) }
    var description by remember { mutableStateOf("") }
    var people by remember { mutableStateOf("1") }
    var injuries by remember { mutableStateOf("0") }
    var location by remember { mutableStateOf("") }
    var movement by remember { mutableStateOf("") }
    Column(Modifier.fillMaxSize().padding(24.dp), verticalArrangement = Arrangement.spacedBy(9.dp)) {
        Text("$recipientName 님에게만 보내는 비공개 SOS")
        Text("공공 구조기관 신고가 아니며 전달·응답·구조를 보장하지 않습니다. 가능한 다른 구조 수단도 함께 사용하세요.")
        Button(onClick = { category = category % 6 + 1 }) { Text("상황 분류 $category (눌러 변경)") }
        OutlinedTextField(description, { description = it.take(800) }, label = { Text("도움이 필요한 상황") })
        OutlinedTextField(people, { people = it.filter(Char::isDigit).take(2) }, label = { Text("전체 인원") })
        OutlinedTextField(injuries, { injuries = it.filter(Char::isDigit).take(2) }, label = { Text("중상 인원") })
        OutlinedTextField(location, { location = it.take(200) }, label = { Text("수동 위치 (선택, 권한 불필요)") })
        OutlinedTextField(movement, { movement = it.take(100) }, label = { Text("이동 방향 (선택)") })
        Button(
            enabled = description.isNotBlank(),
            onClick = { onSend(category, description, people.toIntOrNull() ?: 1, injuries.toIntOrNull() ?: 0, location.ifBlank { null }, movement) },
            modifier = Modifier.semantics { contentDescription = "선택한 연락처에게 비공개 SOS 암호화 저장" },
        ) { Text("비공개 SOS 저장") }
        Button(onClick = onBack) { Text("뒤로") }
    }
}
