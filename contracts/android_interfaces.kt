package org.disastermesh.android.domain

import kotlinx.coroutines.flow.Flow

interface BlePlatformAdapter {
    val events: Flow<TransportEvent>
    suspend fun execute(command: PlatformCommand): CommandResult
    suspend fun close()
}

interface MeshRuntime {
    val homeState: Flow<HomeUiState>
    val serviceState: Flow<RelayServiceState>

    suspend fun start()
    suspend fun setMode(mode: RelayMode)
    suspend fun sendDirectText(contactId: ByteArray, text: String): Result<ByteArray>
    suspend fun sendCheckIn(draft: CheckInDraft): Result<MessageBatchRef>
    suspend fun sendPrivateSos(draft: PrivateSosDraft): Result<MessageBatchRef>
    suspend fun cancelMessage(messageId: ByteArray, reason: CancelReason): Result<Unit>
    suspend fun shutdown()
}

/**
 * The only class allowed to call the generated Rust MeshEngine.
 * All calls run on a dedicated limitedParallelism(1) dispatcher.
 */
interface MeshCoordinator {
    suspend fun accept(event: TransportEvent)
    suspend fun accept(event: SystemEvent)
    suspend fun refreshSnapshots()
}
