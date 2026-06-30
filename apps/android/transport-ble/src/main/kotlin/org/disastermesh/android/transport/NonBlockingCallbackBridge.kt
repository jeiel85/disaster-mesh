package org.disastermesh.android.transport

import java.util.concurrent.ArrayBlockingQueue

sealed interface BleCallbackEvent {
    val linkId: Long

    data class CommandCompleted(
        override val linkId: Long,
        val commandId: Long,
        val status: Int,
        val acceptedBytes: Int?,
    ) : BleCallbackEvent

    data class BytesReceived(
        override val linkId: Long,
        val characteristic: java.util.UUID,
        val bytes: ByteArray,
    ) : BleCallbackEvent

    data class LinkFailed(
        override val linkId: Long,
        val category: TransportFailureCategory,
    ) : BleCallbackEvent
}

sealed interface CallbackOfferResult {
    data object Accepted : CallbackOfferResult
    data class CloseLink(val linkId: Long) : CallbackOfferResult
}

/**
 * GATT callbacks only copy immutable data and call `offer`; no DB, FFI, crypto, or blocking wait.
 * Queue overflow fails the affected link closed so protocol bytes are never silently dropped.
 */
class NonBlockingCallbackBridge(capacity: Int = 256) {
    private val events = ArrayBlockingQueue<BleCallbackEvent>(capacity)

    fun offer(event: BleCallbackEvent): CallbackOfferResult = if (events.offer(event)) {
        CallbackOfferResult.Accepted
    } else {
        CallbackOfferResult.CloseLink(event.linkId)
    }

    fun poll(): BleCallbackEvent? = events.poll()

    fun size(): Int = events.size
}
