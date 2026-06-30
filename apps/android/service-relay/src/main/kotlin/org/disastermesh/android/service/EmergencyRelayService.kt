package org.disastermesh.android.service

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.BatteryManager
import android.os.Build
import android.os.IBinder
import android.os.PowerManager
import org.disastermesh.android.domain.RelayDeviceState
import org.disastermesh.android.domain.RelayDutyPlan
import org.disastermesh.android.domain.RelayDutyPolicy
import org.disastermesh.android.domain.RelayMode

class EmergencyRelayService : Service() {
    private lateinit var preferences: RelayPreferences
    private var plan: RelayDutyPlan? = null

    override fun onCreate() {
        super.onCreate()
        preferences = RelayPreferences(this)
        notificationManager().createNotificationChannel(
            NotificationChannel(CHANNEL_ID, "DisasterMesh 릴레이", NotificationManager.IMPORTANCE_LOW),
        )
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_STOP -> {
                preferences.setUserEnabled(false)
                stopForeground(STOP_FOREGROUND_REMOVE)
                stopSelf()
                return START_NOT_STICKY
            }
            ACTION_START -> {
                val mode = parseMode(intent.getStringExtra(EXTRA_MODE))
                preferences.setMode(mode)
                preferences.setUserEnabled(true)
            }
            ACTION_RECOVER, ACTION_REEVALUATE, null -> if (!preferences.userEnabled()) {
                stopSelf()
                return START_NOT_STICKY
            }
        }
        plan = evaluatePlan(preferences.mode(), intent?.getBooleanExtra(EXTRA_STORAGE_LOW, false) == true)
        val notification = notification(preferences.mode(), requireNotNull(plan))
        if (Build.VERSION.SDK_INT >= 29) {
            startForeground(NOTIFICATION_ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE)
        } else {
            startForeground(NOTIFICATION_ID, notification)
        }
        return START_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private fun evaluatePlan(mode: RelayMode, storageLow: Boolean): RelayDutyPlan {
        val battery = getSystemService(BatteryManager::class.java)
            ?.getIntProperty(BatteryManager.BATTERY_PROPERTY_CAPACITY)
            ?.takeIf { it in 0..100 } ?: 50
        val charging = getSystemService(BatteryManager::class.java)?.isCharging == true
        val thermalSevere = if (Build.VERSION.SDK_INT >= 29) {
            (getSystemService(PowerManager::class.java)?.currentThermalStatus ?: 0) >=
                PowerManager.THERMAL_STATUS_SEVERE
        } else {
            false
        }
        return RelayDutyPolicy.evaluate(RelayDeviceState(mode, battery, charging, thermalSevere, storageLow))
    }

    private fun notification(mode: RelayMode, plan: RelayDutyPlan): Notification {
        val launch = packageManager.getLaunchIntentForPackage(packageName)
        val contentIntent = launch?.let {
            PendingIntent.getActivity(this, 1, it, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
        }
        val stopIntent = Intent(this, EmergencyRelayService::class.java).setAction(ACTION_STOP)
        val stop = PendingIntent.getService(this, 2, stopIntent, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
        return Notification.Builder(this, CHANNEL_ID)
            .setSmallIcon(android.R.drawable.stat_sys_data_bluetooth)
            .setContentTitle("DisasterMesh ${mode.name.lowercase()} 릴레이")
            .setContentText("${plan.reason} · 전달은 보장되지 않습니다")
            .setContentIntent(contentIntent)
            .setOngoing(true)
            .addAction(Notification.Action.Builder(null, "릴레이 중지", stop).build())
            .build()
    }

    private fun notificationManager() = getSystemService(NotificationManager::class.java)

    companion object {
        const val ACTION_START = "org.disastermesh.action.RELAY_START"
        const val ACTION_STOP = "org.disastermesh.action.RELAY_STOP"
        const val ACTION_RECOVER = "org.disastermesh.action.RELAY_RECOVER"
        const val ACTION_REEVALUATE = "org.disastermesh.action.RELAY_REEVALUATE"
        const val EXTRA_MODE = "relay_mode"
        const val EXTRA_STORAGE_LOW = "storage_low"
        private const val CHANNEL_ID = "disastermesh_relay"
        private const val NOTIFICATION_ID = 41

        fun start(context: Context, mode: String) {
            val intent = Intent(context, EmergencyRelayService::class.java)
                .setAction(ACTION_START)
                .putExtra(EXTRA_MODE, mode)
            context.startForegroundService(intent)
        }

        fun stop(context: Context) {
            context.startService(Intent(context, EmergencyRelayService::class.java).setAction(ACTION_STOP))
        }

        private fun parseMode(value: String?): RelayMode =
            runCatching { RelayMode.valueOf(value ?: "") }.getOrDefault(RelayMode.STANDBY)
    }
}

internal class RelayPreferences(context: Context) {
    private val values = context.getSharedPreferences("relay_lifecycle", Context.MODE_PRIVATE)

    fun userEnabled(): Boolean = values.getBoolean("user_enabled", false)
    fun setUserEnabled(enabled: Boolean) { values.edit().putBoolean("user_enabled", enabled).apply() }
    fun mode(): RelayMode = runCatching {
        RelayMode.valueOf(values.getString("mode", null) ?: "")
    }.getOrDefault(RelayMode.STANDBY)
    fun setMode(mode: RelayMode) { values.edit().putString("mode", mode.name).apply() }
}
