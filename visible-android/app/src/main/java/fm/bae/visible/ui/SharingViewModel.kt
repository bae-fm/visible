package fm.bae.visible.ui

import android.content.Context
import android.util.Log
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import fm.bae.visible.AppSession
import fm.bae.visible.HomeSwitch
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.visible_bridge.AppHandle
import uniffi.visible_bridge.BridgeMember
import uniffi.visible_bridge.BridgeMemberRole

private const val TAG = "visible.SharingViewModel"

/**
 * A value loaded off-main: in flight, failed with a message, or the loaded value.
 * Used for the identity code and the member list so a failure is rendered, never
 * silently stuck on "Loading…".
 */
sealed interface Loadable<out Value> {
    data object Loading : Loadable<Nothing>

    data class Failed(val message: String) : Loadable<Nothing>

    data class Loaded<Value>(val value: Value) : Loadable<Value>
}

/**
 * Loads and mutates the sharing state for one library: this device's identity
 * code, the member list, inviting a member, and joining or restoring a home.
 * Bridge calls touch SQLite, the keyring, and the network, so they run on
 * [Dispatchers.IO]; the read-modify-write of the screen state happens here on the
 * state, not in the composable (observable-mutate-on-the-state-not-the-view). The
 * composable reads these fields and renders them.
 */
class SharingViewModel(
    private val handle: AppHandle,
    private val session: AppSession,
    private val context: Context,
) : ViewModel() {
    /**
     * Whether the sync loop is running. The members list, inviting, and the
     * restore code require a connected library; the identity code and joining /
     * restoring a home do not.
     */
    var connected by mutableStateOf(false)
        private set

    /**
     * This device's identity code, sent to a home's owner so they can invite this
     * device. Loaded on open.
     */
    var identityCode: Loadable<String> by mutableStateOf(Loadable.Loading)
        private set

    /** The member list. Reloaded after a remove. */
    var members: Loadable<List<BridgeMember>> by mutableStateOf(Loadable.Loading)
        private set

    /** The invitee's identity code the owner pastes, and the role to grant. */
    var inviteIdentityCode by mutableStateOf("")
    var inviteRole by mutableStateOf(BridgeMemberRole.MEMBER)

    /**
     * The invite code produced by the last successful invite, to send back to the
     * invitee; null until one is minted.
     */
    var inviteCode: String? by mutableStateOf(null)
        private set

    /** The codes pasted to join or restore a home, each holding its own value. */
    var joinInviteCode by mutableStateOf("")
    var restoreInputCode by mutableStateOf("")

    /**
     * This owner device's restore code, rendered for the user to save; null until
     * "Show my restore code" mints it.
     */
    var restoreCode: String? by mutableStateOf(null)
        private set

    /**
     * The member the owner is confirming a remove of; null when no confirm is up.
     */
    var pendingRemoval: BridgeMember? by mutableStateOf(null)
        private set

    /**
     * The join or restore the user is confirming (it replaces the current home);
     * null when no confirm is up.
     */
    var pendingSwitch: HomeSwitch? by mutableStateOf(null)
        private set

    /**
     * A bridge call is in flight (disables the action buttons). Local UI state for
     * the in-flight gesture, not a domain value.
     */
    var working by mutableStateOf(false)
        private set

    /** The last action failure, cleared on the next attempt. */
    var errorMessage: String? by mutableStateOf(null)
        private set

    /**
     * A member's role as a row label. The role-to-label decision is domain, so it
     * lives on the model, not the composable.
     */
    fun roleLabel(member: BridgeMember): String = roleName(member.role)

    /** The label for a grantable role in the invite picker. */
    fun roleName(role: BridgeMemberRole): String = SharingLogic.roleName(role)

    /**
     * Load the identity code, the connected flag, and the member list together so
     * the section visibility and its rows reflect the same point in time.
     */
    fun reload() {
        viewModelScope.launch {
            val loaded = withContext(Dispatchers.IO) {
                val identity: Loadable<String> = try {
                    Loadable.Loaded(handle.userIdentityCode())
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    Log.e(TAG, "loading identity code failed", e)
                    Loadable.Failed(e.message ?: e.toString())
                }
                val connected = handle.syncStatus().ready
                val members: Loadable<List<BridgeMember>> = try {
                    Loadable.Loaded(handle.members())
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    Log.e(TAG, "loading members failed", e)
                    Loadable.Failed(e.message ?: e.toString())
                }
                Triple(identity, connected, members)
            }
            identityCode = loaded.first
            connected = loaded.second
            members = loaded.third
        }
    }

    /**
     * Open the remove confirmation for [member] (the owner removing another
     * device). Removing re-keys the library.
     */
    fun confirmRemove(member: BridgeMember) {
        pendingRemoval = member
    }

    fun dismissRemove() {
        pendingRemoval = null
    }

    /** Remove the pending member, then reload the list. Re-keys the library. */
    fun removePending() {
        val member = pendingRemoval
        if (member == null) {
            Log.e(TAG, "removePending called with no pending member")
            return
        }
        pendingRemoval = null
        errorMessage = null
        working = true
        viewModelScope.launch {
            val failure = runBridgeWrite(TAG, "removing member ${member.pubkey}") {
                handle.removeMember(member.pubkey)
            }
            working = false
            if (failure != null) {
                errorMessage = failure
            } else {
                members = Loadable.Loading
                reload()
            }
        }
    }

    /**
     * Invite the device whose identity code is in the field, granting the picked
     * role, and show the returned invite code to send back. Validates the field is
     * non-empty.
     */
    fun invite() {
        val code = inviteIdentityCode.trim()
        if (code.isEmpty()) {
            errorMessage = "Paste the invitee's identity code first."
            return
        }
        val role = inviteRole
        errorMessage = null
        working = true
        viewModelScope.launch {
            val result = withContext(Dispatchers.IO) {
                try {
                    Result.success(handle.inviteMember(code, role))
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    Log.e(TAG, "inviting member failed", e)
                    Result.failure(e)
                }
            }
            working = false
            result.fold(
                onSuccess = { inviteCode = it },
                onFailure = { errorMessage = it.message ?: it.toString() },
            )
        }
    }

    /** Mint and show this owner device's restore code for the user to save. */
    fun showRestoreCode() {
        errorMessage = null
        working = true
        viewModelScope.launch {
            val result = withContext(Dispatchers.IO) {
                try {
                    Result.success(handle.restoreCode())
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    Log.e(TAG, "loading restore code failed", e)
                    Result.failure(e)
                }
            }
            working = false
            result.fold(
                onSuccess = { restoreCode = it },
                onFailure = { errorMessage = it.message ?: it.toString() },
            )
        }
    }

    /** Open the replace confirmation for joining from the pasted invite code. */
    fun confirmJoin() {
        val code = joinInviteCode.trim()
        if (code.isEmpty()) {
            errorMessage = "Paste an invite code first."
            return
        }
        pendingSwitch = HomeSwitch.Join(code)
    }

    /** Open the replace confirmation for restoring from the pasted restore code. */
    fun confirmRestore() {
        val code = restoreInputCode.trim()
        if (code.isEmpty()) {
            errorMessage = "Paste a restore code first."
            return
        }
        pendingSwitch = HomeSwitch.Restore(code)
    }

    fun dismissSwitch() {
        pendingSwitch = null
    }

    /**
     * Carry out the confirmed join or restore: [AppSession.switchToHome] writes
     * the new library to disk, opens it, and removes the current home. A failure
     * there leaves the current home intact.
     */
    fun switchPending() {
        val pending = pendingSwitch
        if (pending == null) {
            Log.e(TAG, "switchPending called with no pending switch")
            return
        }
        pendingSwitch = null
        errorMessage = null
        working = true
        viewModelScope.launch {
            errorMessage = session.switchToHome(context, pending)
            working = false
        }
    }
}
