package org.disastermesh.android.service

import android.app.Service
import android.content.Intent
import android.os.IBinder

/** Goal 0 manifest placeholder. Relay lifecycle behavior is intentionally absent. */
class EmergencyRelayService : Service() {
    override fun onBind(intent: Intent?): IBinder? = null
}
