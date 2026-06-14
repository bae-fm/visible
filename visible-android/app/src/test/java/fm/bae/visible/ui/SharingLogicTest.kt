package fm.bae.visible.ui

import org.junit.Assert.assertEquals
import org.junit.Test
import uniffi.visible_bridge.BridgeMemberRole

/**
 * [SharingLogic.roleName] labels a member's role for the members list and the
 * invite picker. Pure logic, no Android framework, so it runs on the JVM under
 * testDebugUnitTest.
 */
class SharingLogicTest {
    @Test
    fun roleNameLabelsEachRole() {
        assertEquals("Owner", SharingLogic.roleName(BridgeMemberRole.OWNER))
        assertEquals("Member", SharingLogic.roleName(BridgeMemberRole.MEMBER))
        assertEquals("Follower", SharingLogic.roleName(BridgeMemberRole.FOLLOWER))
    }
}
