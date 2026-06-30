package org.disastermesh.android.transport

import java.nio.ByteBuffer
import java.util.UUID

object BleClaUuids {
    val service: UUID = UUID.fromString("6f1d0001-8f6b-4d5b-9c61-57c43d4d4d31")
    val controlRx: UUID = UUID.fromString("6f1d0002-8f6b-4d5b-9c61-57c43d4d4d31")
    val controlTx: UUID = UUID.fromString("6f1d0003-8f6b-4d5b-9c61-57c43d4d4d31")
    val dataRx: UUID = UUID.fromString("6f1d0004-8f6b-4d5b-9c61-57c43d4d4d31")
    val dataTx: UUID = UUID.fromString("6f1d0005-8f6b-4d5b-9c61-57c43d4d4d31")
    val clientCharacteristicConfiguration: UUID =
        UUID.fromString("00002902-0000-1000-8000-00805f9b34fb")
}

enum class RelayModeBits(val bits: Int) {
    STANDBY(0),
    EMERGENCY(1),
    FIXED(2),
}

enum class QueueLoadBits(val bits: Int) {
    EMPTY(0),
    LOW(1),
    MEDIUM(2),
    HIGH(3),
}

data class BeaconState(
    val gattServerAvailable: Boolean,
    val relayEnabled: Boolean,
    val mode: RelayModeBits,
    val queueLoad: QueueLoadBits,
) {
    fun encode(): Byte = (
        (if (gattServerAvailable) 1 else 0) or
            ((if (relayEnabled) 1 else 0) shl 1) or
            (mode.bits shl 2) or
            (queueLoad.bits shl 4)
        ).toByte()
}

data class LegacyAdvertisement(
    val beaconId: ByteArray,
    val state: BeaconState,
) {
    init {
        require(beaconId.size == BEACON_BYTES)
    }

    fun serviceData(): ByteArray = byteArrayOf(PROTOCOL_MAJOR, state.encode()) + beaconId

    /** Byte-exact legacy payload: Flags (3) + 128-bit Service Data AD structure (28). */
    fun rawPayload(): ByteArray {
        val uuidLittleEndian = ByteBuffer.allocate(16)
            .putLong(BleClaUuids.service.mostSignificantBits)
            .putLong(BleClaUuids.service.leastSignificantBits)
            .array()
            .reversedArray()
        return byteArrayOf(
            0x02,
            AD_TYPE_FLAGS,
            0x06,
            0x1b,
            AD_TYPE_SERVICE_DATA_128,
        ) + uuidLittleEndian + serviceData()
    }

    companion object {
        const val PROTOCOL_MAJOR: Byte = 1
        const val BEACON_BYTES = 8
        const val LEGACY_PAYLOAD_BYTES = 31
        private const val AD_TYPE_FLAGS: Byte = 0x01
        private const val AD_TYPE_SERVICE_DATA_128: Byte = 0x21
    }
}

enum class AdvertiseShape {
    SERVICE_DATA,
    SERVICE_UUID_ONLY,
}

enum class AdvertiseFailure {
    DATA_TOO_LARGE,
    UNSUPPORTED,
    INTERNAL,
}

fun advertisementFallback(current: AdvertiseShape, failure: AdvertiseFailure): AdvertiseShape? =
    when {
        current == AdvertiseShape.SERVICE_DATA && failure == AdvertiseFailure.DATA_TOO_LARGE ->
            AdvertiseShape.SERVICE_UUID_ONLY
        else -> null
    }

enum class RoleDecision {
    CONNECT_AS_CENTRAL,
    WAIT_AS_PERIPHERAL,
    RANDOM_FALLBACK,
}

fun arbitrateCentralRole(localBeaconId: ByteArray, remoteBeaconId: ByteArray?): RoleDecision {
    require(localBeaconId.size == LegacyAdvertisement.BEACON_BYTES)
    if (remoteBeaconId == null) return RoleDecision.RANDOM_FALLBACK
    require(remoteBeaconId.size == LegacyAdvertisement.BEACON_BYTES)
    for (index in localBeaconId.indices) {
        val local = localBeaconId[index].toInt() and 0xff
        val remote = remoteBeaconId[index].toInt() and 0xff
        if (local < remote) return RoleDecision.CONNECT_AS_CENTRAL
        if (local > remote) return RoleDecision.WAIT_AS_PERIPHERAL
    }
    return RoleDecision.RANDOM_FALLBACK
}

object BeaconRotation {
    const val BASE_MILLIS = 10 * 60 * 1000L
    const val MAX_JITTER_MILLIS = 2 * 60 * 1000L

    fun delayMillis(jitterMillis: Long): Long {
        require(jitterMillis in -MAX_JITTER_MILLIS..MAX_JITTER_MILLIS)
        return BASE_MILLIS + jitterMillis
    }
}

enum class TransportFailureCategory {
    CONNECT_TIMEOUT,
    SERVICE_DISCOVERY_TIMEOUT,
    HELLO_TIMEOUT,
    NOISE_TIMEOUT,
    FRAME_IDLE_TIMEOUT,
    CREDIT_TIMEOUT,
    PERMISSION_LOST,
    BLUETOOTH_OFF,
    GATT_PROTOCOL,
    QUEUE_OVERFLOW,
}

object BleTimeouts {
    const val CONNECT_MILLIS = 12_000L
    const val SERVICE_DISCOVERY_MILLIS = 8_000L
    const val HELLO_MILLIS = 5_000L
    const val NOISE_MILLIS = 10_000L
    const val FRAME_IDLE_MILLIS = 15_000L
    const val CREDIT_IDLE_MILLIS = 10_000L
}
