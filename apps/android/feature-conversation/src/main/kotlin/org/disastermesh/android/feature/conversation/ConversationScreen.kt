package org.disastermesh.android.feature.conversation

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

enum class MessageStateLabel(val display: String) {
    STORED("기기에 보관됨"),
    RELAYED("중계망에 복제됨"),
    RECEIPT_CONFIRMED("전달 확인됨"),
    EXPIRED("만료됨"),
    CANCEL_PROPAGATING("취소 전파 중"),
}

data class ConversationRow(
    val text: String,
    val state: MessageStateLabel,
    val packetId: ByteArray? = null,
    val messageId: ByteArray? = null,
)

@Composable
fun ConversationScreen(
    contactName: String,
    rows: List<ConversationRow>,
    communicationReady: Boolean,
    onSend: (String) -> Unit,
    onCancel: (ConversationRow) -> Unit,
    onBack: () -> Unit,
) {
    var draft by remember { mutableStateOf("") }
    Column(
        modifier = Modifier.fillMaxSize().padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text(contactName)
        rows.forEach { row ->
            Text(row.text)
            Text(row.state.display)
            if (row.state == MessageStateLabel.STORED && row.packetId != null && row.messageId != null) {
                Button(onClick = { onCancel(row) }) { Text("전송 취소 전파") }
            }
        }
        OutlinedTextField(
            value = draft,
            onValueChange = { draft = it.take(2_000) },
            label = { Text("메시지") },
        )
        Button(
            onClick = {
                onSend(draft)
                draft = ""
            },
            enabled = communicationReady && draft.isNotBlank() && draft.encodeToByteArray().size <= 7_800,
        ) {
            Text("암호화하여 기기에 보관")
        }
        if (!communicationReady) Text("통신 기능 중지됨 — 작성한 메시지는 전송되지 않습니다.")
        Button(onClick = onBack) { Text("홈으로") }
    }
}
