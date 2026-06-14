package fm.bae.visible.ui

import android.util.Log
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
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
 * Loads and mutates the cloud-sync settings. Bridge calls touch SQLite, the
 * keyring, and the network, so they run on [Dispatchers.IO]; the read-modify-
 * write of the screen state happens here on the state, not in the composable
 * (observable-mutate-on-the-state-not-the-view). The composable reads these
 * fields and renders them.
 */
class SettingsViewModel(
    private val handle: AppHandle,
) : ViewModel() {
    // The editable S3 form fields, seeded blank (form-seeding exemption). Empty
    // optional boxes (endpoint, key prefix) map back to null on connect.
    var bucket by mutableStateOf("")
    var region by mutableStateOf("")
    var endpoint by mutableStateOf("")
    var keyPrefix by mutableStateOf("")
    var accessKey by mutableStateOf("")
    var secretKey by mutableStateOf("")

    var status: BridgeSyncStatus? by mutableStateOf(null)
        private set

    var outbox: BridgeOutboxSnapshot? by mutableStateOf(null)
        private set

    // A connect/disconnect call is in flight (drives the "Connecting…" state and
    // disables the buttons). Local UI state for the in-flight gesture, not a
    // domain value.
    var working by mutableStateOf(false)
        private set

    var errorMessage: String? by mutableStateOf(null)
        private set

    /** Whether the connect button has the minimum required fields. */
    val canConnect: Boolean
        get() = !working && bucket.isNotEmpty() && region.isNotEmpty() &&
            accessKey.isNotEmpty() && secretKey.isNotEmpty()

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
        errorMessage = null
        working = true
        // Trim optional boxes back to null when blank — the inverse of seeding.
        val config = BridgeS3Config(
            bucket = bucket,
            region = region,
            endpoint = endpoint.ifEmpty { null },
            keyPrefix = keyPrefix.ifEmpty { null },
            accessKey = accessKey,
            secretKey = secretKey,
        )
        viewModelScope.launch {
            errorMessage = runWrite("connecting S3") { handle.saveS3Config(config) }
            working = false
            reload()
        }
    }

    /** Disconnect the cloud provider, then refresh the status. */
    fun disconnect() {
        errorMessage = null
        working = true
        viewModelScope.launch {
            errorMessage = runWrite("disconnecting sync") { handle.disconnectSync() }
            working = false
            reload()
        }
    }

    /** Runs a bridge write off-main; returns null on success or the message. */
    private suspend fun runWrite(description: String, write: () -> Unit): String? =
        withContext(Dispatchers.IO) {
            try {
                write()
                null
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                Log.e(TAG, "$description failed", e)
                e.message ?: e.toString()
            }
        }
}
