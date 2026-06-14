package fm.bae.visible.ui

import android.util.Log
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalContext

private const val TAG = "visible.LibraryPick"

/**
 * Opens the photo library for a single image, backed by
 * [ActivityResultContracts.PickVisualMedia] restricted to images. On a pick it
 * reads the chosen content uri's bytes and hands them to [onPicked]; a cancelled
 * pick or a read failure is logged and drops the import. Returns the launch
 * function to invoke (e.g. from an onClick). PickVisualMedia needs no storage
 * permission — it returns a one-shot read grant for the chosen item.
 */
@Composable
fun rememberLibraryPick(onPicked: (ByteArray) -> Unit): () -> Unit {
    val context = LocalContext.current

    val pickMedia = rememberLauncherForActivityResult(
        ActivityResultContracts.PickVisualMedia(),
    ) { uri ->
        if (uri == null) {
            // No selection (the user dismissed the picker).
            return@rememberLauncherForActivityResult
        }
        val bytes = context.contentResolver.openInputStream(uri)?.use { it.readBytes() }
        if (bytes == null) {
            Log.w(TAG, "could not open the picked image uri $uri; dropping import")
            return@rememberLauncherForActivityResult
        }
        onPicked(bytes)
    }

    return {
        pickMedia.launch(
            PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly),
        )
    }
}
