package fm.bae.visible.ui

import uniffi.visible_bridge.BridgeMemberRole

/**
 * The pure role-to-label derivation behind [SharingViewModel], with no
 * [uniffi.visible_bridge.AppHandle] or observable state. The view model
 * delegates here so the labels are exercised directly.
 */
object SharingLogic {
    /**
     * The label for a role: the row label in the members list and the option
     * label in the invite picker.
     */
    fun roleName(role: BridgeMemberRole): String = when (role) {
        BridgeMemberRole.OWNER -> "Owner"
        BridgeMemberRole.MEMBER -> "Member"
        BridgeMemberRole.FOLLOWER -> "Follower"
    }
}
