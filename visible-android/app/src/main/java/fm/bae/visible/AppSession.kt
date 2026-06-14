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

/**
 * Whether [switchToHome] removes the library it replaced. Remove it only when a
 * different library takes its place: there is a previous library and its id
 * differs from the new one. Joining or restoring the home already active produces
 * the same id (coven derives the joined/restored library id from the code, stable
 * per home), so the switch reopens that one directory in place — removing it would
 * delete the directory and keyring key the just-opened handle is backed by. First
 * run has no previous library, so nothing is removed.
 */
internal fun shouldRemovePrevious(previousId: String?, newId: String): Boolean =
    previousId != null && previousId != newId

/**
 * The app-root lifecycle: opening, no library yet (first run), failed to open, or
 * open.
 */
sealed interface SessionState {
    data object Loading : SessionState

    /**
     * No library on this device yet — the onboarding Welcome screen creates or
     * joins the first home. Reached only on first run; once a home exists the
     * session never returns here (switching homes replaces in place).
     */
    data object Onboarding : SessionState

    data class Failed(val message: String) : SessionState

    /**
     * The open library: its handle, the id of its root house node, and its library
     * id (shown in Settings ▸ About for support).
     */
    data class Open(val handle: AppHandle, val rootId: String, val libraryId: String) : SessionState
}

/**
 * A home to make active, replacing the current one if there is one: a fresh local
 * home with a name, a home joined from an invite code, or a home restored from a
 * restore code. Drives both onboarding (the first home) and the settings/sharing
 * switch (replacing the current home on this device).
 */
sealed interface HomeSwitch {
    data class Create(val name: String) : HomeSwitch

    data class Join(val inviteCode: String) : HomeSwitch

    data class Restore(val restoreCode: String) : HomeSwitch

    /**
     * Write the new library to disk via the core call for this source, returning
     * its identity for the session to open. Runs off-main inside
     * [AppSession.switchToHome].
     */
    fun writeLibrary(dataDir: String): BridgeLibrary = when (this) {
        is Create -> createLibrary(dataDir, name)
        is Join -> joinLibraryFromInvite(dataDir, inviteCode)
        is Restore -> restoreLibraryFromCode(dataDir, restoreCode)
    }

    /** A label for the log line on failure, naming the operation without the code. */
    val logLabel: String
        get() = when (this) {
            is Create -> "creating a new home"
            is Join -> "joining from invite code"
            is Restore -> "restoring from restore code"
        }
}

/**
 * Holds the one [AppHandle] for the process and publishes the current
 * [SessionState] for the root view to render. On first open it discovers the
 * library under the app's private data dir; if one exists it opens it and reads
 * the root node id, and if none exists it publishes [SessionState.Onboarding] for
 * the Welcome screen to create or join the first home (it never auto-creates a
 * home). Creating, joining, or restoring a home all go through [switchToHome],
 * which opens the new library before removing the old one — a single active home —
 * so a failure never strands the user. A [SessionState.Failed] is published
 * without disturbing the open library, so the user can retry.
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
     * session on a re-entry (e.g. a retry after a transient failure). On first run
     * (no library on disk) it publishes [SessionState.Onboarding] rather than
     * auto-creating a home. A [SessionState.Failed] is never cached, so the root
     * view's Retry calls this again.
     */
    suspend fun open(context: Context) {
        current?.let {
            _state.value = SessionState.Open(it.handle, it.rootId, it.libraryId)
            return
        }

        val dataDir = context.filesDir.absolutePath
        _state.value = withContext(Dispatchers.IO) {
            try {
                val library = discoverLibraries(dataDir).firstOrNull()
                if (library == null) {
                    SessionState.Onboarding
                } else {
                    val handle = initApp(dataDir, library.id)
                    val rootId = handle.rootNode().id
                    current = Current(handle, rootId, library.id)
                    SessionState.Open(handle, rootId, library.id)
                }
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                Log.e(TAG, "opening library failed", e)
                SessionState.Failed(e.message ?: e.toString())
            }
        }
    }

    /** Create the first home (onboarding "create a home"). No prior library to
     * remove. Returns null on success or the failure message. */
    suspend fun createHome(context: Context, name: String): String? =
        switchToHome(context, HomeSwitch.Create(name))

    /** Join the first home from an invite code (onboarding "join a home"). No
     * prior library to remove. Returns null on success or the failure message. */
    suspend fun joinHome(context: Context, code: String): String? =
        switchToHome(context, HomeSwitch.Join(code))

    /** Restore the first home from a restore code (onboarding "restore a home").
     * No prior library to remove. Returns null on success or the failure message. */
    suspend fun restoreHome(context: Context, code: String): String? =
        switchToHome(context, HomeSwitch.Restore(code))

    /**
     * Make [source]'s home the active one: write the new library to disk (create a
     * fresh local home, or the joiner-side download), open it, then remove the
     * library it replaced if a different one took its place — a single active home.
     * Drives both onboarding (no prior home) and the settings/sharing switch
     * (replacing the current home). Returns null on success or the failure message;
     * on failure [state] and any open library are unchanged.
     *
     * Order matters: write and open the new library FIRST, and only remove the
     * replaced one once the new handle is in hand, so a failed write or open leaves
     * the old library intact and nothing is removed. Re-joining or restoring the
     * home already active reopens the same id ([shouldRemovePrevious] is false), so
     * the switch is a harmless reopen rather than a self-delete.
     */
    suspend fun switchToHome(context: Context, source: HomeSwitch): String? {
        val previous = current

        val dataDir = context.filesDir.absolutePath
        return withContext(Dispatchers.IO) {
            try {
                val library = source.writeLibrary(dataDir)
                val handle = initApp(dataDir, library.id)
                val rootId = handle.rootNode().id
                // The new library is open; dropping the library it replaced (if a
                // different one) can't strand us. Re-joining/restoring the active
                // home reopens the same id, so there is nothing to remove.
                if (shouldRemovePrevious(previous?.libraryId, library.id)) {
                    removeLibrary(dataDir, previous!!.libraryId)
                }
                current = Current(handle, rootId, library.id)
                _state.value = SessionState.Open(handle, rootId, library.id)
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
