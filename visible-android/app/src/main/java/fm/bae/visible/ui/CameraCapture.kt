package fm.bae.visible.ui

import android.Manifest
import android.content.pm.PackageManager
import android.net.Uri
import android.util.Log
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import androidx.core.content.ContextCompat
import androidx.core.content.FileProvider
import java.io.File

private const val TAG = "visible.CameraCapture"

/** Opens the camera and delivers the captured photo's JPEG bytes. */
fun interface CameraCapture {
    fun launch()
}

/**
 * A [CameraCapture] backed by [ActivityResultContracts.TakePicture]: it writes
 * the photo to a temp file in the cache via a [FileProvider] content uri, and on
 * success reads the JPEG bytes back and hands them to [onCaptured], then deletes
 * the temp file. Requests the camera permission first if it isn't granted.
 */
@Composable
fun rememberCameraCapture(onCaptured: (ByteArray) -> Unit): CameraCapture {
    val context = LocalContext.current

    // The temp file's path, held across the launcher round-trip. Saved (not
    // plain remembered) so a captured photo survives the process being
    // reclaimed while the camera is foregrounded.
    var pendingPath: String? by rememberSaveable { mutableStateOf(null) }

    fun readAndClear() {
        val file = pendingPath?.let(::File) ?: return
        pendingPath = null
        onCaptured(file.readBytes())
        file.delete()
    }

    val takePicture = rememberLauncherForActivityResult(
        ActivityResultContracts.TakePicture(),
    ) { saved ->
        if (saved) {
            readAndClear()
        } else {
            pendingPath?.let(::File)?.delete()
            pendingPath = null
        }
    }

    fun startCapture() {
        val dir = File(context.cacheDir, "camera-capture").apply { mkdirs() }
        val file = File.createTempFile("photo", ".jpg", dir)
        pendingPath = file.absolutePath
        val uri: Uri = FileProvider.getUriForFile(
            context,
            "${context.packageName}.fileprovider",
            file,
        )
        takePicture.launch(uri)
    }

    val requestPermission = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { granted ->
        if (granted) {
            startCapture()
        } else {
            Log.w(TAG, "camera permission denied; cannot capture a photo")
        }
    }

    return CameraCapture {
        val granted = ContextCompat.checkSelfPermission(
            context,
            Manifest.permission.CAMERA,
        ) == PackageManager.PERMISSION_GRANTED
        if (granted) {
            startCapture()
        } else {
            requestPermission.launch(Manifest.permission.CAMERA)
        }
    }
}
