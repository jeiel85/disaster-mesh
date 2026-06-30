package org.disastermesh.android.domain

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class BluetoothPermissionPolicyTest {
    @Test
    fun api30RequiresLocationForBleButSosCanRemainLocationless() {
        val snapshot = BluetoothPermissionSnapshot(
            apiLevel = 30,
            bluetoothSupported = true,
            bluetoothEnabled = true,
            fineLocationGranted = false,
            scanGranted = false,
            connectGranted = false,
            advertiseGranted = false,
        )
        assertFalse(evaluateCommunicationReadiness(snapshot).ready)
        assertFalse(canAttachGpsLocation(snapshot))
    }

    @Test
    fun api31SeparatesBluetoothFromOptionalGpsPermission() {
        val snapshot = BluetoothPermissionSnapshot(
            apiLevel = 31,
            bluetoothSupported = true,
            bluetoothEnabled = true,
            fineLocationGranted = false,
            scanGranted = true,
            connectGranted = true,
            advertiseGranted = true,
        )
        assertTrue(evaluateCommunicationReadiness(snapshot).ready)
        assertFalse(canAttachGpsLocation(snapshot))
    }

    @Test
    fun permissionRevocationBecomesBlockedStateInsteadOfAnException() {
        val revoked = BluetoothPermissionSnapshot(
            apiLevel = 36,
            bluetoothSupported = true,
            bluetoothEnabled = true,
            fineLocationGranted = false,
            scanGranted = false,
            connectGranted = true,
            advertiseGranted = true,
        )
        assertTrue(
            CommunicationBlock.BLUETOOTH_SCAN_PERMISSION in
                evaluateCommunicationReadiness(revoked).blockers,
        )
    }
}
