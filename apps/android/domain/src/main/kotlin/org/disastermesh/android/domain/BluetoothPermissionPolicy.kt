package org.disastermesh.android.domain

enum class CommunicationBlock {
    BLUETOOTH_UNSUPPORTED,
    BLUETOOTH_OFF,
    LEGACY_LOCATION_PERMISSION,
    BLUETOOTH_SCAN_PERMISSION,
    BLUETOOTH_CONNECT_PERMISSION,
    BLUETOOTH_ADVERTISE_PERMISSION,
}

data class BluetoothPermissionSnapshot(
    val apiLevel: Int,
    val bluetoothSupported: Boolean,
    val bluetoothEnabled: Boolean,
    val fineLocationGranted: Boolean,
    val scanGranted: Boolean,
    val connectGranted: Boolean,
    val advertiseGranted: Boolean,
)

data class CommunicationReadiness(val blockers: Set<CommunicationBlock>) {
    val ready: Boolean get() = blockers.isEmpty()
}

fun evaluateCommunicationReadiness(snapshot: BluetoothPermissionSnapshot): CommunicationReadiness {
    val blockers = linkedSetOf<CommunicationBlock>()
    if (!snapshot.bluetoothSupported) blockers += CommunicationBlock.BLUETOOTH_UNSUPPORTED
    if (!snapshot.bluetoothEnabled) blockers += CommunicationBlock.BLUETOOTH_OFF
    if (snapshot.apiLevel <= 30) {
        if (!snapshot.fineLocationGranted) {
            blockers += CommunicationBlock.LEGACY_LOCATION_PERMISSION
        }
    } else {
        if (!snapshot.scanGranted) blockers += CommunicationBlock.BLUETOOTH_SCAN_PERMISSION
        if (!snapshot.connectGranted) blockers += CommunicationBlock.BLUETOOTH_CONNECT_PERMISSION
        if (!snapshot.advertiseGranted) blockers += CommunicationBlock.BLUETOOTH_ADVERTISE_PERMISSION
    }
    return CommunicationReadiness(blockers)
}

/** GPS attachment remains optional on every API level and is not a communication prerequisite. */
fun canAttachGpsLocation(snapshot: BluetoothPermissionSnapshot): Boolean =
    snapshot.fineLocationGranted
