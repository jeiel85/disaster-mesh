package org.disastermesh.android

import android.Manifest
import android.bluetooth.BluetoothManager
import android.os.BatteryManager
import android.os.Bundle
import android.os.Build
import android.os.SystemClock
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import android.content.pm.PackageManager
import org.disastermesh.android.feature.contacts.ContactRow
import org.disastermesh.android.feature.contacts.ContactTrustLabel
import org.disastermesh.android.feature.contacts.ContactsScreen
import org.disastermesh.android.feature.conversation.ConversationRow
import org.disastermesh.android.feature.conversation.ConversationScreen
import org.disastermesh.android.feature.conversation.MessageStateLabel
import org.disastermesh.android.feature.onboarding.OnboardingScreen
import org.disastermesh.android.feature.home.ProductHome
import org.disastermesh.android.feature.checkin.CheckInScreen
import org.disastermesh.android.feature.sos.PrivateSosScreen
import org.disastermesh.android.feature.relay.RelayStatusScreen
import org.disastermesh.android.feature.diagnostics.DiagnosticExportScreen
import org.disastermesh.android.feature.diagnostics.buildDiagnosticArchive
import org.disastermesh.android.service.EmergencyRelayService
import org.disastermesh.android.security.MasterKeyManager
import org.disastermesh.core.MeshEngine
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.security.SecureRandom

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme { DisasterMeshRoot() }
        }
    }
}

private enum class BootstrapScreen { ONBOARDING, HOME, CONTACTS, CONVERSATION, CHECK_IN, SOS, RELAY, DIAGNOSTICS }

private sealed interface EngineState {
    data object Loading : EngineState
    data class Ready(val engine: MeshEngine, val ownQr: String) : EngineState
    data class Blocked(val reason: String) : EngineState
}

@Composable
private fun DisasterMeshRoot() {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var screen by remember { mutableStateOf(BootstrapScreen.ONBOARDING) }
    var engineState by remember { mutableStateOf<EngineState>(EngineState.Loading) }
    var contacts by remember { mutableStateOf(emptyList<ContactRow>()) }
    var selectedContactId by remember { mutableStateOf<ByteArray?>(null) }
    var messages by remember { mutableStateOf(emptyList<ConversationRow>()) }
    var communicationReady by remember { mutableStateOf(isCommunicationReady(context)) }
    var pendingDiagnosticArchive by remember { mutableStateOf<ByteArray?>(null) }
    val permissionLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions(),
    ) {
        communicationReady = isCommunicationReady(context)
    }
    val bootId = remember { ByteArray(16).also(SecureRandom()::nextBytes) }
    val diagnosticLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument("application/zip"),
    ) { uri ->
        val archive = pendingDiagnosticArchive
        pendingDiagnosticArchive = null
        if (uri != null && archive != null) {
            scope.launch(Dispatchers.IO) {
                context.contentResolver.openOutputStream(uri, "w")?.use { it.write(archive) }
            }
        }
    }

    LaunchedEffect(Unit) {
        engineState = withContext(Dispatchers.IO) {
            try {
                val key = MasterKeyManager(context).loadOrCreate()
                try {
                    val engine = MeshEngine.open(
                        context.getDatabasePath("disastermesh.db").absolutePath,
                        key,
                        "DisasterMesh 사용자",
                        System.currentTimeMillis().toULong(),
                    )
                    EngineState.Ready(engine, engine.ownContactQr(0x1fu))
                } finally {
                    key.fill(0)
                }
            } catch (_: Exception) {
                EngineState.Blocked("로컬 암호화 키 또는 데이터베이스를 열 수 없습니다. 자동 초기화하지 않습니다.")
            }
        }
    }
    DisposableEffect(engineState) {
        onDispose { (engineState as? EngineState.Ready)?.engine?.close() }
    }

    when (screen) {
        BootstrapScreen.ONBOARDING -> OnboardingScreen(
            communicationReady = communicationReady,
            onRequestPermissions = { permissionLauncher.launch(requiredPermissions()) },
            onContinue = { screen = BootstrapScreen.HOME },
        )
        BootstrapScreen.HOME -> ProductHome(
            contactReady = selectedContactId != null,
            onContacts = { screen = BootstrapScreen.CONTACTS },
            onCheckIn = { screen = BootstrapScreen.CHECK_IN },
            onSos = { screen = BootstrapScreen.SOS },
            onRelay = { screen = BootstrapScreen.RELAY },
            onDiagnostics = { screen = BootstrapScreen.DIAGNOSTICS },
        )
        BootstrapScreen.CONTACTS -> ContactsScreen(
            ownContactQr = (engineState as? EngineState.Ready)?.ownQr,
            contacts = contacts,
            onImportQr = { qr ->
                val ready = engineState as? EngineState.Ready ?: return@ContactsScreen
                scope.launch {
                    val imported = withContext(Dispatchers.IO) {
                        runCatching {
                            ready.engine.importContactQr(qr, System.currentTimeMillis().toULong())
                        }.getOrNull()
                    } ?: return@launch
                    selectedContactId = imported.contactId
                    contacts = listOf(
                        ContactRow(
                            displayName = imported.displayName,
                            displayId = imported.displayId,
                            safetyNumber = imported.safetyNumber,
                            trust = ContactTrustLabel.UNVERIFIED,
                        ),
                    )
                }
            },
            onOpenConversation = { screen = BootstrapScreen.CONVERSATION },
        )
        BootstrapScreen.CONVERSATION -> ConversationScreen(
            contactName = contacts.firstOrNull()?.displayName ?: "연락처",
            rows = messages,
            communicationReady = communicationReady,
            onSend = { text ->
                val ready = engineState as? EngineState.Ready ?: return@ConversationScreen
                val contactId = selectedContactId ?: return@ConversationScreen
                scope.launch {
                    val stored = withContext(Dispatchers.IO) {
                        runCatching {
                            ready.engine.sendDirectText(
                                contactId,
                                text,
                                System.currentTimeMillis().toULong(),
                                bootId,
                                SystemClock.elapsedRealtime().toULong(),
                            )
                        }.getOrNull()
                    }
                    if (stored != null) {
                        messages = messages + ConversationRow(
                            text = text,
                            state = MessageStateLabel.STORED,
                            packetId = stored.packetId,
                            messageId = stored.messageId,
                        )
                    }
                }
            },
            onCancel = { row ->
                val ready = engineState as? EngineState.Ready ?: return@ConversationScreen
                val contactId = selectedContactId ?: return@ConversationScreen
                val packetId = row.packetId ?: return@ConversationScreen
                val messageId = row.messageId ?: return@ConversationScreen
                scope.launch {
                    val canceled = withContext(Dispatchers.IO) {
                        runCatching {
                            ready.engine.sendCancel(
                                contactId,
                                packetId,
                                messageId,
                                1u,
                                System.currentTimeMillis().toULong(),
                                bootId,
                                SystemClock.elapsedRealtime().toULong(),
                            )
                        }.isSuccess
                    }
                    if (canceled) {
                        messages = messages.map {
                            if (it.packetId?.contentEquals(packetId) == true) {
                                it.copy(state = MessageStateLabel.CANCEL_PROPAGATING)
                            } else {
                                it
                            }
                        }
                    }
                }
            },
            onBack = { screen = BootstrapScreen.HOME },
        )
        BootstrapScreen.CHECK_IN -> CheckInScreen(
            recipientName = contacts.firstOrNull()?.displayName ?: "연락처",
            onBack = { screen = BootstrapScreen.HOME },
            onSend = { status, people, note, location ->
                val ready = engineState as? EngineState.Ready ?: return@CheckInScreen
                val contactId = selectedContactId ?: return@CheckInScreen
                scope.launch(Dispatchers.IO) {
                    runCatching {
                        ready.engine.sendCheckIn(
                            contactId,
                            status.toUByte(),
                            people.coerceIn(1, 99).toUByte(),
                            note,
                            location,
                            batteryPercent(context),
                            System.currentTimeMillis().toULong(),
                            bootId,
                            SystemClock.elapsedRealtime().toULong(),
                        )
                    }
                }
                screen = BootstrapScreen.HOME
            },
        )
        BootstrapScreen.SOS -> PrivateSosScreen(
            recipientName = contacts.firstOrNull()?.displayName ?: "연락처",
            onBack = { screen = BootstrapScreen.HOME },
            onSend = { category, description, people, injuries, location, movement ->
                val ready = engineState as? EngineState.Ready ?: return@PrivateSosScreen
                val contactId = selectedContactId ?: return@PrivateSosScreen
                scope.launch(Dispatchers.IO) {
                    runCatching {
                        ready.engine.sendPrivateSos(
                            contactId,
                            category.toUByte(),
                            description,
                            people.coerceIn(1, 99).toUByte(),
                            injuries.coerceIn(0, people).toUByte(),
                            location,
                            movement,
                            batteryPercent(context),
                            System.currentTimeMillis().toULong(),
                            bootId,
                            SystemClock.elapsedRealtime().toULong(),
                        )
                    }
                }
                screen = BootstrapScreen.HOME
            },
        )
        BootstrapScreen.RELAY -> RelayStatusScreen(
            onStart = { mode -> EmergencyRelayService.start(context, mode.name) },
            onStop = { EmergencyRelayService.stop(context) },
            onBack = { screen = BootstrapScreen.HOME },
        )
        BootstrapScreen.DIAGNOSTICS -> DiagnosticExportScreen(
            onExport = {
                pendingDiagnosticArchive = buildDiagnosticArchive(context)
                diagnosticLauncher.launch("disastermesh-diagnostics.zip")
            },
            onBack = { screen = BootstrapScreen.HOME },
        )
    }
}

private fun requiredPermissions(): Array<String> = if (Build.VERSION.SDK_INT <= 30) {
    arrayOf(Manifest.permission.ACCESS_FINE_LOCATION)
} else {
    buildList {
        add(Manifest.permission.BLUETOOTH_SCAN)
        add(Manifest.permission.BLUETOOTH_CONNECT)
        add(Manifest.permission.BLUETOOTH_ADVERTISE)
        if (Build.VERSION.SDK_INT >= 33) add(Manifest.permission.POST_NOTIFICATIONS)
    }.toTypedArray()
}

private fun batteryPercent(context: android.content.Context): UByte? {
    val value = context.getSystemService(BatteryManager::class.java)
        ?.getIntProperty(BatteryManager.BATTERY_PROPERTY_CAPACITY) ?: return null
    return value.takeIf { it in 0..100 }?.toUByte()
}

private fun isCommunicationReady(context: android.content.Context): Boolean {
    val manager = context.getSystemService(BluetoothManager::class.java)
    val adapter = manager?.adapter ?: return false
    if (requiredPermissions().any {
            context.checkSelfPermission(it) != PackageManager.PERMISSION_GRANTED
        }
    ) {
        return false
    }
    return runCatching { adapter.isEnabled }.getOrDefault(false)
}
