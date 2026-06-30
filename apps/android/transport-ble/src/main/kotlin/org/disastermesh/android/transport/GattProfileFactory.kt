package org.disastermesh.android.transport

import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothGattDescriptor
import android.bluetooth.BluetoothGattService

object GattProfileFactory {
    fun createService(): BluetoothGattService = BluetoothGattService(
        BleClaUuids.service,
        BluetoothGattService.SERVICE_TYPE_PRIMARY,
    ).apply {
        addCharacteristic(
            BluetoothGattCharacteristic(
                BleClaUuids.controlRx,
                BluetoothGattCharacteristic.PROPERTY_WRITE,
                BluetoothGattCharacteristic.PERMISSION_WRITE,
            ),
        )
        addCharacteristic(
            BluetoothGattCharacteristic(
                BleClaUuids.controlTx,
                BluetoothGattCharacteristic.PROPERTY_INDICATE,
                BluetoothGattCharacteristic.PERMISSION_READ,
            ).withClientConfiguration(),
        )
        addCharacteristic(
            BluetoothGattCharacteristic(
                BleClaUuids.dataRx,
                BluetoothGattCharacteristic.PROPERTY_WRITE or
                    BluetoothGattCharacteristic.PROPERTY_WRITE_NO_RESPONSE,
                BluetoothGattCharacteristic.PERMISSION_WRITE,
            ),
        )
        addCharacteristic(
            BluetoothGattCharacteristic(
                BleClaUuids.dataTx,
                BluetoothGattCharacteristic.PROPERTY_NOTIFY,
                BluetoothGattCharacteristic.PERMISSION_READ,
            ).withClientConfiguration(),
        )
    }

    private fun BluetoothGattCharacteristic.withClientConfiguration() = apply {
        addDescriptor(
            BluetoothGattDescriptor(
                BleClaUuids.clientCharacteristicConfiguration,
                BluetoothGattDescriptor.PERMISSION_READ or BluetoothGattDescriptor.PERMISSION_WRITE,
            ),
        )
    }
}
