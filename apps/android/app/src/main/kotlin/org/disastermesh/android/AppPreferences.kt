package org.disastermesh.android

import android.content.Context
import androidx.core.content.edit

internal class AppPreferences(context: Context) {
    private val values = context.getSharedPreferences(PREFERENCES_NAME, Context.MODE_PRIVATE)

    fun onboardingCompleted(): Boolean = values.getBoolean(ONBOARDING_COMPLETED, false)

    fun setOnboardingCompleted(completed: Boolean) {
        values.edit { putBoolean(ONBOARDING_COMPLETED, completed) }
    }

    private companion object {
        const val PREFERENCES_NAME = "disastermesh_app"
        const val ONBOARDING_COMPLETED = "onboarding_completed"
    }
}
