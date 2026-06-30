package org.disastermesh.android.transport

import org.junit.Assert.assertEquals
import org.junit.Test

class NonBlockingCallbackBridgeTest {
    @Test
    fun byteEventsAreCopiedAndOverflowClosesTheAffectedLink() {
        val bridge = NonBlockingCallbackBridge(capacity = 1)
        val source = byteArrayOf(1, 2, 3)
        val first = BleCallbackEvent.BytesReceived(7, BleClaUuids.controlRx, source.copyOf())
        assertEquals(CallbackOfferResult.Accepted, bridge.offer(first))
        source[0] = 9
        assertEquals(
            CallbackOfferResult.CloseLink(8),
            bridge.offer(
                BleCallbackEvent.LinkFailed(
                    8,
                    TransportFailureCategory.QUEUE_OVERFLOW,
                ),
            ),
        )
        val accepted = bridge.poll() as BleCallbackEvent.BytesReceived
        assertEquals(1, accepted.bytes[0].toInt())
    }
}
