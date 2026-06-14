package fm.bae.visible.ui

import android.content.Context
import android.util.Log
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import fm.bae.visible.AppSession
import fm.bae.visible.BuildConfig
import fm.bae.visible.HomeSwitch
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.visible_bridge.AppHandle
import uniffi.visible_bridge.BridgeOutboxSnapshot
import uniffi.visible_bridge.BridgeS3Config
import uniffi.visible_bridge.BridgeSyncStatus

private const val TAG = "visible.SettingsViewModel"

/**
 * Loads and mutates the settings screen for one open home: its name (rename),
 * cloud sync, and starting a fresh home that replaces this one. Bridge calls touch
 * SQLite, the keyring, and the network, so they run on [Dispatchers.IO]; the read-
 * modify-write of the screen state happens here on the state, not in the
 * composable (observable-mutate-on-the-state-not-the-view). The composable reads
 * these fields and renders them.
 */
class SettingsViewModel(
    private val handle: AppHandle,
    private val session: AppSession,
    private val context: Context,
    private val rootId: String,
    /** This home's library id, shown in About for support. */
    val libraryId: String,
) : ViewModel() {
    /**
     * The home's current name (the root node's name), shown in "This Home" and used
     * to seed the rename dialog. Loaded on open.
     */
    var homeName: Loadable<String> by mutableStateOf(Loadable.Loading)
        private set

    // The editable S3 form fields, seeded blank (form-seeding exemption). A
    // blank or whitespace-only endpoint or key prefix is mapped to null in
    // connect() — its absence — so core receives None, never "".
    var bucket by mutableStateOf("")
    var region by mutableStateOf("")
    var endpoint by mutableStateOf("")
    var keyPrefix by mutableStateOf("")
    var accessKey by mutableStateOf("")
    var secretKey by mutableStateOf("")

    /**
     * The name for a fresh home, seeded with a suggestion the user can edit
     * (form-seeding exemption). Trimmed on submit.
     */
    var newHomeName by mutableStateOf("Home")

    /**
     * The name the user is confirming a "start a new home" replace for; null when
     * no confirm is up. Set when the user taps "Start a new home" with a non-blank
     * name and cleared on confirm or dismiss.
     */
    var pendingNewHome: String? by mutableStateOf(null)
        private set

    var status: BridgeSyncStatus? by mutableStateOf(null)
        private set

    var outbox: BridgeOutboxSnapshot? by mutableStateOf(null)
        private set

    // A connect/disconnect/rename/switch call is in flight (drives the
    // "Connecting…" state and disables the buttons). Local UI state for the
    // in-flight gesture, not a domain value.
    var working by mutableStateOf(false)
        private set

    var errorMessage: String? by mutableStateOf(null)
        private set

    /** The app version and build for the About section, from BuildConfig. */
    val appVersion: String
        get() = "${BuildConfig.VERSION_NAME} (${BuildConfig.VERSION_CODE})"

    /** Whether the connect button has the minimum required fields. */
    val canConnect: Boolean
        get() = SettingsLogic.canConnect(bucket, region, accessKey, secretKey, working)

    /** Whether a provider is configured (a Disconnect / Sync-now action makes sense). */
    val isConnected: Boolean
        get() = status?.configured == true

    /**
     * The one-line status: the in-flight connect, then the configured/ready
     * state, with the pending delete count appended when there is work queued.
     * Composed here on the model from the booleans and count the bridge provides
     * plus the local in-flight flag, so the composable renders it directly.
     */
    val statusLine: String
        get() = SettingsLogic.statusLine(
            working = working,
            configured = status?.configured == true,
            ready = status?.ready == true,
            pendingDeletes = outbox?.pendingDeletes ?: 0uL,
        )

    /**
     * Load the home's current name from the root node, so "This Home" shows it and
     * the rename dialog seeds from it.
     */
    fun loadHome() {
        viewModelScope.launch {
            homeName = withContext(Dispatchers.IO) {
                try {
                    val root = handle.getNode(rootId)
                    if (root == null) {
                        Log.e(TAG, "root node $rootId not found loading the home name")
                        Loadable.Failed("This home's root is missing.")
                    } else {
                        // The root always has a name (create sets it); fall back to
                        // empty only if the type's optionality ever surfaces.
                        Loadable.Loaded(root.name ?: "")
                    }
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    Log.e(TAG, "loading the home name failed", e)
                    Loadable.Failed(e.message ?: e.toString())
                }
            }
        }
    }

    /**
     * Rename the home (the root node), then reload its name. The browse root
     * reflects the new name on its next resume.
     */
    fun renameHome(name: String) {
        val trimmed = name.trim()
        if (trimmed.isEmpty()) return
        errorMessage = null
        working = true
        viewModelScope.launch {
            errorMessage = runBridgeWrite(TAG, "renaming the home") {
                handle.renameNode(rootId, trimmed)
            }
            working = false
            loadHome()
        }
    }

    /**
     * Open the replace confirmation for starting a fresh home with the trimmed
     * new-home name. Validates the field is non-empty.
     */
    fun confirmNewHome() {
        val name = newHomeName.trim()
        if (name.isEmpty()) {
            errorMessage = "Give the new home a name first."
            return
        }
        pendingNewHome = name
    }

    fun dismissNewHome() {
        pendingNewHome = null
    }

    /**
     * Carry out the confirmed "start a new home": [AppSession.switchToHome] creates
     * the fresh local home, opens it, and removes the current one. A failure there
     * leaves the current home intact.
     */
    fun startNewHome() {
        val name = pendingNewHome
        if (name == null) {
            Log.e(TAG, "startNewHome called with no pending new home")
            return
        }
        pendingNewHome = null
        errorMessage = null
        working = true
        viewModelScope.launch {
            errorMessage = session.switchToHome(context, HomeSwitch.Create(name))
            working = false
        }
    }

    /** Load the current sync status and outbox counts. */
    fun reload() {
        viewModelScope.launch {
            val loaded = withContext(Dispatchers.IO) {
                try {
                    handle.syncStatus() to handle.outboxSnapshot()
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    Log.e(TAG, "loading sync status failed", e)
                    null
                }
            }
            if (loaded != null) {
                status = loaded.first
                outbox = loaded.second
            }
        }
    }

    /** Probe and connect the S3 cloud home, then refresh the status. */
    fun connect() {
        // Map a blank or whitespace-only optional box to null at the form (its
        // absence); trim so a real value is sent without surrounding whitespace.
        val config = BridgeS3Config(
            bucket = bucket,
            region = region,
            endpoint = SettingsLogic.optionalField(endpoint),
            keyPrefix = SettingsLogic.optionalField(keyPrefix),
            accessKey = accessKey,
            secretKey = secretKey,
        )
        runAction("connecting S3") { handle.saveS3Config(config) }
    }

    /** Disconnect the cloud provider, then refresh the status. */
    fun disconnect() {
        runAction("disconnecting sync") { handle.disconnectSync() }
    }

    /**
     * Request an immediate sync cycle, then refresh the status so the outbox
     * counts reflect the drain. A no-op in the bridge when sync isn't connected.
     */
    fun triggerSync() {
        viewModelScope.launch {
            withContext(Dispatchers.IO) { handle.triggerSync() }
            reload()
        }
    }

    /**
     * Mark a connect/disconnect in flight, run the bridge write off-main, then
     * clear the in-flight flag and reload the status. The error (or null on
     * success) lands in [errorMessage] for the screen to show.
     */
    private fun runAction(description: String, write: () -> Unit) {
        errorMessage = null
        working = true
        viewModelScope.launch {
            errorMessage = runBridgeWrite(TAG, description, write)
            working = false
            reload()
        }
    }
}
