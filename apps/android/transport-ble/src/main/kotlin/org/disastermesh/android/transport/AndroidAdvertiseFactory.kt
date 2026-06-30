package org.disastermesh.android.transport

import android.bluetooth.le.AdvertiseData
import android.bluetooth.le.AdvertiseSettings
import android.bluetooth.le.ScanFilter
import android.os.ParcelUuid

object AndroidAdvertiseFactory {
    fun settings(): AdvertiseSettings = AdvertiseSettings.Builder()
        .setAdvertiseMode(AdvertiseSettings.ADVERTISE_MODE_BALANCED)
        .setConnectable(true)
        .setTimeout(0)
        .setTxPowerLevel(AdvertiseSettings.ADVERTISE_TX_POWER_MEDIUM)
        .build()

    fun primary(advertisement: LegacyAdvertisement): AdvertiseData = AdvertiseData.Builder()
        .setIncludeDeviceName(false)
        .setIncludeTxPowerLevel(false)
        .addServiceData(ParcelUuid(BleClaUuids.service), advertisement.serviceData())
        .build()

    fun serviceUuidOnly(): AdvertiseData = AdvertiseData.Builder()
        .setIncludeDeviceName(false)
        .setIncludeTxPowerLevel(false)
        .addServiceUuid(ParcelUuid(BleClaUuids.service))
        .build()

    fun scanFilter(): ScanFilter = ScanFilter.Builder()
        .setServiceUuid(ParcelUuid(BleClaUuids.service))
        .build()
}
