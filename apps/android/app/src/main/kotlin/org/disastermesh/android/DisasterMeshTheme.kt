package org.disastermesh.android

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

private val DisasterMeshLightScheme = lightColorScheme(
    primary = Color(0xFF235D50),
    onPrimary = Color.White,
    primaryContainer = Color(0xFFA8F2D9),
    onPrimaryContainer = Color(0xFF002019),
    secondary = Color(0xFF4C635B),
    onSecondary = Color.White,
    background = Color(0xFFF5FBF7),
    onBackground = Color(0xFF171D1A),
    surface = Color(0xFFF5FBF7),
    onSurface = Color(0xFF171D1A),
    surfaceVariant = Color(0xFFDBE5DF),
    onSurfaceVariant = Color(0xFF404943),
    error = Color(0xFFBA1A1A),
)

private val DisasterMeshDarkScheme = darkColorScheme(
    primary = Color(0xFF8CD6BE),
    onPrimary = Color(0xFF00382E),
    primaryContainer = Color(0xFF075043),
    onPrimaryContainer = Color(0xFFA8F2D9),
    secondary = Color(0xFFB3CCC2),
    onSecondary = Color(0xFF1E352E),
    background = Color(0xFF0F1512),
    onBackground = Color(0xFFDEE4E0),
    surface = Color(0xFF0F1512),
    onSurface = Color(0xFFDEE4E0),
    surfaceVariant = Color(0xFF404943),
    onSurfaceVariant = Color(0xFFBFC9C3),
    error = Color(0xFFFFB4AB),
)

@Composable
internal fun DisasterMeshTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = if (isSystemInDarkTheme()) DisasterMeshDarkScheme else DisasterMeshLightScheme,
        typography = Typography(),
        content = content,
    )
}
