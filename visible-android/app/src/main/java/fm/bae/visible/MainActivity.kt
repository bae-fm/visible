package fm.bae.visible

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import fm.bae.visible.ui.AppRoot
import fm.bae.visible.ui.VisibleTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val session = (application as VisibleApp).session
        setContent {
            VisibleTheme {
                AppRoot(session = session)
            }
        }
    }
}
