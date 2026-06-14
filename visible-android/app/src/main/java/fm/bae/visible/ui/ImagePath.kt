package fm.bae.visible.ui

import android.util.Log
import uniffi.visible_bridge.AppHandle

private const val TAG = "visible.ImagePath"

/**
 * The local file path for [imageId] if its file exists, else null. The bridge
 * call does no database work (a filesystem existence check), so the image
 * composables call it on the render path. A node whose image file isn't on disk
 * renders the placeholder. Shared by the browse and search view models so the
 * existence check and the missing-image log live in one place.
 */
fun imagePath(handle: AppHandle, imageId: String): String? {
    val path = handle.imagePathIfExists(imageId)
    if (path == null) {
        Log.d(TAG, "no image file for $imageId; showing placeholder")
    }
    return path
}
