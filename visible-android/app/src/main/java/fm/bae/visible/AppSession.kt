package fm.bae.visible

import android.content.Context
import android.util.Log
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.withContext
import uniffi.visible_bridge.AppHandle
import uniffi.visible_bridge.BridgeLibrary
import uniffi.visible_bridge.createLibrary
import uniffi.visible_bridge.discoverLibraries
import uniffi.visible_bridge.initApp
import uniffi.visible_bridge.joinLibraryFromInvite
import uniffi.visible_bridge.removeLibrary
import uniffi.visible_bridge.restoreLibraryFromCode

private const val TAG = "visible.AppSession"

/** The app-root lifecycle: the session is opening, failed to open, or open. */
sealed interface SessionState {
    data object Loading : SessionState

    data class Failed(val message: String) : SessionState

    /** The open library: its handle and the id of its root house node. */
    data class Open(val handle: AppHandle, val rootId: String) : SessionState
}

/**
 * A pending home switch the user is confirming: joining from an invite code, or
 * restoring from a restore code. Both replace the current home on this device.
 */
sealed interface HomeSwitch {
    data class Join(val inviteCode: String) : HomeSwitch

    data class Restore(val restoreCode: String) : HomeSwitch

    /**
     * Write the new library to disk via the joiner-side core call for this
     * source, returning its identity for the session to open. Runs off-main
     * inside [AppSession.switchToHome].
     */
    fun writeLibrary(dataDir: String): BridgeLibrary = when (this) {
        is Join -> joinLibraryFromInvite(dataDir, inviteCode)
        is Restore -> restoreLibraryFromCode(dataDir, restoreCode)
    }

    /** A label for the log line on failure, naming the operation without the code. */
    val logLabel: String
        get() = when (this) {
            is Join -> "joining from invite code"
            is Restore -> "restoring from restore code"
        }
}

/**
 * Holds the one [AppHandle] for the process and publishes the current
 * [SessionState] for the root view to render. On first open it discovers the
 * library under the app's private data dir (creating the default one if none
 * exists), opens it, and reads the root node id. Joining or restoring a home
 * replaces the open library in place — a single active home — via [switchToHome].
 * A [SessionState.Failed] is published without disturbing the open library, so
 * the user can retry.
 */
class AppSession {
    private val _state = MutableStateFlow<SessionState>(SessionState.Loading)
    val state: StateFlow<SessionState> = _state.asStateFlow()

    // The open library's handle, root node id, and library id. The library id is
    // the one switchToHome removes when a new home replaces this one. null until
    // the first successful open.
    private data class Current(val handle: AppHandle, val rootId: String, val libraryId: String)

    @Volatile
    private var current: Current? = null

    /**
     * Open the library and publish the resulting [SessionState]. The bridge calls
     * touch SQLite, so they run on [Dispatchers.IO]; reuse the already open
     * session on a re-entry (e.g. a retry after a transient failure). A
     * [SessionState.Failed] is never cached, so the root view's Retry calls this
     * again.
     */
    suspend fun open(context: Context) {
        current?.let {
            _state.value = SessionState.Open(it.handle, it.rootId)
            return
        }

        val dataDir = context.filesDir.absolutePath
        _state.value = withContext(Dispatchers.IO) {
            try {
                val library = discoverLibraries(dataDir).firstOrNull()
                    ?: createLibrary(dataDir)
                val handle = initApp(dataDir, library.id)
                val rootId = handle.rootNode().id
                current = Current(handle, rootId, library.id)
                SessionState.Open(handle, rootId)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                Log.e(TAG, "opening library failed", e)
                SessionState.Failed(e.message ?: e.toString())
            }
        }
    }

    /**
     * Switch the active library to a home the user joined or restored: write the
     * new library to disk (the joiner-side call), open it, then remove the
     * previously open library — a single active home. Returns null on success or
     * the failure message; on failure [state] and the open library are unchanged.
     *
     * Order matters: write and open the new library FIRST, and only remove the old
     * one once the new handle is in hand, so a failed write or open leaves the old
     * library intact and nothing is removed.
     */
    suspend fun switchToHome(context: Context, source: HomeSwitch): String? {
        val previous = current
        if (previous == null) {
            // open() must have run before any sharing action is reachable.
            Log.e(TAG, "switchToHome called before the session was open")
            return "The current home isn't open yet."
        }

        val dataDir = context.filesDir.absolutePath
        return withContext(Dispatchers.IO) {
            try {
                val library = source.writeLibrary(dataDir)
                val handle = initApp(dataDir, library.id)
                val rootId = handle.rootNode().id
                // The new library is open; dropping the old one can't strand us.
                removeLibrary(dataDir, previous.libraryId)
                current = Current(handle, rootId, library.id)
                _state.value = SessionState.Open(handle, rootId)
                null
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                Log.e(TAG, "${source.logLabel} failed", e)
                e.message ?: e.toString()
            }
        }
    }
}
