package org.disastermesh.android.service

import android.Manifest
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build

class RelayBootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val enabled = RelayPreferences(context).userEnabled()
        if (!enabled || !hasBluetoothPermission(context)) return
        val action = when (intent.action) {
            Intent.ACTION_BOOT_COMPLETED -> EmergencyRelayService.ACTION_RECOVER
            ACTION_STORAGE_LOW, ACTION_STORAGE_OK ->
                EmergencyRelayService.ACTION_REEVALUATE
            else -> return
        }
        val service = Intent(context, EmergencyRelayService::class.java)
            .setAction(action)
            .putExtra(
                EmergencyRelayService.EXTRA_STORAGE_LOW,
                intent.action == ACTION_STORAGE_LOW,
            )
        context.startForegroundService(service)
    }

    private fun hasBluetoothPermission(context: Context): Boolean =
        Build.VERSION.SDK_INT < 31 ||
            context.checkSelfPermission(Manifest.permission.BLUETOOTH_CONNECT) == PackageManager.PERMISSION_GRANTED

    companion object {
        private const val ACTION_STORAGE_LOW = "android.intent.action.DEVICE_STORAGE_LOW"
        private const val ACTION_STORAGE_OK = "android.intent.action.DEVICE_STORAGE_OK"
    }
}
