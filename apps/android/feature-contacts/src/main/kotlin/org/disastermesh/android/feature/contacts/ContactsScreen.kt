package org.disastermesh.android.feature.contacts

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
import androidx.compose.ui.unit.dp

enum class ContactTrustLabel(val display: String) {
    UNVERIFIED("미확인"),
    VERIFIED_IN_PERSON("대면 확인됨"),
    KEY_CHANGED("키 변경됨 — 재확인 필요"),
    REVOKED("폐기됨"),
}

data class ContactRow(
    val displayName: String,
    val displayId: String,
    val safetyNumber: String,
    val trust: ContactTrustLabel,
)

@Composable
fun ContactsScreen(
    ownContactQr: String?,
    contacts: List<ContactRow>,
    onImportQr: (String) -> Unit,
    onOpenConversation: () -> Unit,
) {
    var qrInput by remember { mutableStateOf("") }
    Column(
        modifier = Modifier.fillMaxSize().padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("내 연락처 QR 문자열")
        Text(ownContactQr ?: "identity가 열리면 QR이 표시됩니다.")
        OutlinedTextField(
            value = qrInput,
            onValueChange = { qrInput = it.take(512) },
            label = { Text("DM1: 연락처 문자열 가져오기") },
        )
        Button(onClick = { onImportQr(qrInput) }, enabled = qrInput.startsWith("DM1:")) {
            Text("서명 검증 후 미확인 연락처로 가져오기")
        }
        contacts.forEach { contact ->
            Text("${contact.displayName} · ${contact.displayId}")
            Text("${contact.trust.display} · 안전번호 ${contact.safetyNumber}")
        }
        Button(onClick = onOpenConversation, enabled = contacts.isNotEmpty()) {
            Text("일대일 대화 열기")
        }
    }
}
