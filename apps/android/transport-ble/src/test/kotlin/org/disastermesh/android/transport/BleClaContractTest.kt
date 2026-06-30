package org.disastermesh.android.transport

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class BleClaContractTest {
    @Test
    fun legacyAdvertisementIsExactly31BytesWithoutStableIdentity() {
        val advertisement = LegacyAdvertisement(
            beaconId = byteArrayOf(1, 2, 3, 4, 5, 6, 7, 8),
            state = BeaconState(
                gattServerAvailable = true,
                relayEnabled = true,
                mode = RelayModeBits.EMERGENCY,
                queueLoad = QueueLoadBits.MEDIUM,
            ),
        )

        assertEquals(10, advertisement.serviceData().size)
        assertEquals(LegacyAdvertisement.LEGACY_PAYLOAD_BYTES, advertisement.rawPayload().size)
        assertArrayEquals(byteArrayOf(1, 0x27, 1, 2, 3, 4, 5, 6, 7, 8), advertisement.serviceData())
        assertEquals(0x21, advertisement.rawPayload()[4].toInt() and 0xff)
    }

    @Test
    fun dataTooLargeFallsBackOnlyOnceToServiceUuid() {
        assertEquals(
            AdvertiseShape.SERVICE_UUID_ONLY,
            advertisementFallback(AdvertiseShape.SERVICE_DATA, AdvertiseFailure.DATA_TOO_LARGE),
        )
        assertNull(
            advertisementFallback(
                AdvertiseShape.SERVICE_UUID_ONLY,
                AdvertiseFailure.DATA_TOO_LARGE,
            ),
        )
        assertNull(advertisementFallback(AdvertiseShape.SERVICE_DATA, AdvertiseFailure.INTERNAL))
    }

    @Test
    fun beaconArbitrationNeverUsesBluetoothAddress() {
        assertEquals(
            RoleDecision.CONNECT_AS_CENTRAL,
            arbitrateCentralRole(byteArrayOf(0, 0, 0, 0, 0, 0, 0, 1), byteArrayOf(0, 0, 0, 0, 0, 0, 0, 2)),
        )
        assertEquals(
            RoleDecision.WAIT_AS_PERIPHERAL,
            arbitrateCentralRole(byteArrayOf(-1, 0, 0, 0, 0, 0, 0, 1), byteArrayOf(1, 0, 0, 0, 0, 0, 0, 2)),
        )
        assertEquals(
            RoleDecision.RANDOM_FALLBACK,
            arbitrateCentralRole(byteArrayOf(0, 0, 0, 0, 0, 0, 0, 1), null),
        )
    }

    @Test
    fun mtuIsRequestedOnceAndUsesObservedPayload() {
        val negotiation = MtuNegotiation()
        assertEquals(MtuRequestDecision.Request(517), negotiation.begin(7))
        assertEquals(MtuRequestDecision.AlreadyRequested, negotiation.begin(7))
        assertEquals(20, negotiation.applicationPayload(23, 512))
        assertEquals(244, negotiation.applicationPayload(247, 512))
        assertNull(negotiation.applicationPayload(22, 512))
        negotiation.close(7)
        assertEquals(MtuRequestDecision.Request(517), negotiation.begin(7))
    }

    @Test
    fun rotationJitterStaysWithinNormativeWindow() {
        assertEquals(8 * 60 * 1000L, BeaconRotation.delayMillis(-120_000))
        assertEquals(12 * 60 * 1000L, BeaconRotation.delayMillis(120_000))
    }
}
