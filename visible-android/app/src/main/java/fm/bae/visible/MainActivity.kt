package fm.bae.visible

import android.Manifest
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import fm.bae.visible.ui.AppRoot
import fm.bae.visible.ui.VisibleTheme

class MainActivity : ComponentActivity() {
    // Taking a node's photo needs the camera. Request once at launch; the
    // capture flow checks the grant and re-requests if the user declined.
    private val requestCameraPermission =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) {}

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        requestCameraPermission.launch(Manifest.permission.CAMERA)
        val session = (application as VisibleApp).session
        setContent {
            VisibleTheme {
                AppRoot(session = session)
            }
        }
    }
}
