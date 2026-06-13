package fm.bae.visible

import android.content.Context
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.visible_bridge.AppHandle
import uniffi.visible_bridge.createLibrary
import uniffi.visible_bridge.discoverLibraries
import uniffi.visible_bridge.initApp

private const val TAG = "visible.AppSession"

/** The app-root lifecycle: the session is opening, failed to open, or open. */
sealed interface SessionState {
    data object Loading : SessionState

    data class Failed(val message: String) : SessionState

    /** The open library: its handle and the id of its root house node. */
    data class Open(val handle: AppHandle, val rootId: String) : SessionState
}

/**
 * Holds the one [AppHandle] for the process. On first open it discovers the
 * library under the app's private data dir (creating the default one if none
 * exists), opens it, and reads the root node id. The handle stays open for the
 * process lifetime — v1 has a single library and no unlock, so there is nothing
 * to switch to or dispose around.
 */
class AppSession {
    @Volatile
    private var state: SessionState = SessionState.Loading

    /**
     * Open the library and produce the resulting [SessionState]. The bridge
     * calls touch SQLite, so they run on [Dispatchers.IO]; reuse the already
     * open session on a re-entry (e.g. config change recreating the activity).
     */
    suspend fun open(context: Context): SessionState {
        (state as? SessionState.Open)?.let { return it }

        val dataDir = context.filesDir.absolutePath
        val next = withContext(Dispatchers.IO) {
            try {
                val library = discoverLibraries(dataDir).firstOrNull()
                    ?: createLibrary(dataDir)
                val handle = initApp(dataDir, library.id)
                SessionState.Open(handle, handle.rootNode().id)
            } catch (e: Exception) {
                Log.e(TAG, "opening library failed", e)
                SessionState.Failed(e.message ?: "Failed to open library")
            }
        }
        state = next
        return next
    }
}
