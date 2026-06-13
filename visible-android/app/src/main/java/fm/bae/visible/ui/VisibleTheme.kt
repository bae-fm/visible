package fm.bae.visible.ui

import android.app.Activity
import android.os.Build
import android.view.View
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SideEffect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalView

private val lightColors = lightColorScheme(
    primary = Color(0xFF386A6A),
    onPrimary = Color(0xFFFFFFFF),
    primaryContainer = Color(0xFFBCECEB),
    onPrimaryContainer = Color(0xFF002020),
    secondary = Color(0xFF4A6363),
    onSecondary = Color(0xFFFFFFFF),
    background = Color(0xFFFCFCFF),
    onBackground = Color(0xFF1A1C1E),
    surface = Color(0xFFFCFCFF),
    onSurface = Color(0xFF1A1C1E),
    surfaceVariant = Color(0xFFDADADA),
    onSurfaceVariant = Color(0xFF41484C),
    error = Color(0xFFBA1A1A),
    onError = Color(0xFFFFFFFF),
)

private val darkColors = darkColorScheme(
    primary = Color(0xFFA0CFCE),
    onPrimary = Color(0xFF003737),
    primaryContainer = Color(0xFF1E4E4E),
    onPrimaryContainer = Color(0xFFBCECEB),
    secondary = Color(0xFFB1CCCB),
    onSecondary = Color(0xFF1B3534),
    background = Color(0xFF0F1117),
    onBackground = Color(0xFFE4E8ED),
    surface = Color(0xFF171922),
    onSurface = Color(0xFFE4E8ED),
    surfaceVariant = Color(0xFF40484C),
    onSurfaceVariant = Color(0xFFC0C8CC),
    error = Color(0xFFFFB4AB),
    onError = Color(0xFF690005),
)

@Suppress("DEPRECATION")
@Composable
fun VisibleTheme(content: @Composable () -> Unit) {
    val isDark = isSystemInDarkTheme()
    val colorScheme = if (isDark) darkColors else lightColors
    val view = LocalView.current
    if (!view.isInEditMode) {
        val activity = view.context as Activity
        SideEffect {
            val window = activity.window
            window.statusBarColor = colorScheme.background.toArgb()
            window.navigationBarColor = colorScheme.background.toArgb()
            val lightStatusBar = View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR
            val lightNavBar = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O_MR1) {
                View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR
            } else {
                0
            }
            val lightBars = lightStatusBar or lightNavBar
            window.decorView.systemUiVisibility = if (isDark) {
                window.decorView.systemUiVisibility and lightBars.inv()
            } else {
                window.decorView.systemUiVisibility or lightBars
            }
        }
    }
    MaterialTheme(colorScheme = colorScheme, content = content)
}
