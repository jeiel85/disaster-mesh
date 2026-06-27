package org.disastermesh.android.domain

import kotlinx.coroutines.flow.Flow

sealed interface CommandEnqueueResult {
    data object Accepted : CommandEnqueueResult
    data class Rejected(val category: TransportFailure) : CommandEnqueueResult
}

interface BlePlatformAdapter {
    val events: Flow<TransportEvent>
    /** Returns after durable/in-memory queue acceptance, never after the GATT callback. */
    suspend fun enqueue(command: PlatformCommand): CommandEnqueueResult
    suspend fun close()
}

interface GattOperationQueue {
    /** Exactly one in-flight GATT operation per link. */
    suspend fun enqueue(command: PlatformCommand): CommandEnqueueResult
    fun onCallback(linkId: Long, callbackKind: GattCallbackKind, status: Int, acceptedBytes: Int? = null)
    fun failAll(linkId: Long, reason: TransportFailure)
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
    suspend fun deleteLocalData(scope: DeleteScope): Result<Unit>
    suspend fun resetIdentity(confirmation: ResetIdentityConfirmation): Result<Unit>
    suspend fun shutdown()
}

/** The only class allowed to call the generated Rust MeshEngine. */
interface MeshCoordinator {
    suspend fun accept(event: TransportEvent)
    suspend fun accept(event: SystemEvent)
    suspend fun refreshSnapshots()
}
