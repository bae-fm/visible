package fm.bae.visible.ui

import android.content.Context
import android.util.Log
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import fm.bae.visible.AppSession
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.visible_bridge.userIdentityCode

private const val TAG = "visible.WelcomeViewModel"

/**
 * The first-run onboarding state: name and create a home, or join / restore one
 * from a code, plus this device's identity code to hand to a home's owner. Bridge
 * calls touch the keyring, disk, and the network, so they run on [Dispatchers.IO];
 * the read-modify-write of the screen state happens here on the state, not in the
 * composable (observable-mutate-on-the-state-not-the-view). The composable reads
 * these fields and renders them.
 */
class WelcomeViewModel(
    private val session: AppSession,
    private val context: Context,
) : ViewModel() {
    /**
     * The name for a new home, seeded with a suggestion the user can edit
     * (form-seeding exemption). Trimmed on submit; the create button is disabled
     * while it is blank.
     */
    var homeName by mutableStateOf("Home")

    /** The codes pasted to join or restore an existing home, each its own value. */
    var joinInviteCode by mutableStateOf("")
    var restoreInputCode by mutableStateOf("")

    /**
     * This device's identity code, sent to a home's owner so they can invite this
     * device before it has a library. Loaded on open; read from the global keyring
     * keypair, so it works with no library on disk.
     */
    var identityCode: Loadable<String> by mutableStateOf(Loadable.Loading)
        private set

    /**
     * An onboarding call is in flight (disables the action buttons). Local UI state
     * for the in-flight gesture, not a domain value.
     */
    var working by mutableStateOf(false)
        private set

    /** The last onboarding failure, cleared on the next attempt. */
    var errorMessage: String? by mutableStateOf(null)
        private set

    /** Whether the create button has a non-blank name. */
    val canCreate: Boolean
        get() = !working && homeName.trim().isNotEmpty()

    /** Load this device's identity code from the global keyring keypair. */
    fun reload() {
        viewModelScope.launch {
            identityCode = withContext(Dispatchers.IO) {
                try {
                    Loadable.Loaded(userIdentityCode())
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    Log.e(TAG, "loading identity code failed", e)
                    Loadable.Failed(e.message ?: e.toString())
                }
            }
        }
    }

    /**
     * Create a fresh local home named after the (trimmed) name field. On success
     * the session opens onto the new home; a failure surfaces in [errorMessage].
     */
    fun createHome() {
        val name = homeName.trim()
        if (name.isEmpty()) {
            errorMessage = "Give your home a name first."
            return
        }
        run { session.createHome(context, name) }
    }

    /** Join an existing home from the pasted invite code. */
    fun joinHome() {
        val code = joinInviteCode.trim()
        if (code.isEmpty()) {
            errorMessage = "Paste an invite code first."
            return
        }
        run { session.joinHome(context, code) }
    }

    /** Restore an existing home from the pasted restore code. */
    fun restoreHome() {
        val code = restoreInputCode.trim()
        if (code.isEmpty()) {
            errorMessage = "Paste a restore code first."
            return
        }
        run { session.restoreHome(context, code) }
    }

    /**
     * Mark the onboarding call in flight, run it, then clear the flag. On success
     * the session publishes [fm.bae.visible.SessionState.Open] and this screen is
     * replaced, so only the failure message lands back here.
     */
    private fun run(complete: suspend () -> String?) {
        errorMessage = null
        working = true
        viewModelScope.launch {
            errorMessage = complete()
            working = false
        }
    }
}
