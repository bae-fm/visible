package fm.bae.visible.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * [SettingsLogic] derives the settings status line, the connect gate, and the
 * optional-field absence mapping the view model delegates to. Pure logic, no
 * Android framework, so it runs on the JVM under testDebugUnitTest.
 */
class SettingsLogicTest {
    @Test
    fun statusLineShowsConnectingWhileWorking() {
        // The in-flight flag wins over everything else.
        assertEquals(
            "Connecting…",
            SettingsLogic.statusLine(working = true, configured = true, ready = true, pendingDeletes = 3uL),
        )
    }

    @Test
    fun statusLineNotConnectedWhenNoProvider() {
        assertEquals(
            "Not connected",
            SettingsLogic.statusLine(working = false, configured = false, ready = false, pendingDeletes = 0uL),
        )
    }

    @Test
    fun statusLineStartingWhenConfiguredButNotReady() {
        assertEquals(
            "Connected (starting…)",
            SettingsLogic.statusLine(working = false, configured = true, ready = false, pendingDeletes = 0uL),
        )
    }

    @Test
    fun statusLineSyncedWithNoPending() {
        assertEquals(
            "Synced",
            SettingsLogic.statusLine(working = false, configured = true, ready = true, pendingDeletes = 0uL),
        )
    }

    @Test
    fun statusLineAppendsPendingDeleteCount() {
        assertEquals(
            "Synced · 2 to delete",
            SettingsLogic.statusLine(working = false, configured = true, ready = true, pendingDeletes = 2uL),
        )
    }

    @Test
    fun canConnectRequiresBucketRegionAndBothKeys() {
        assertTrue(SettingsLogic.canConnect("b", "r", "ak", "sk", working = false))
    }

    @Test
    fun canConnectFalseWhenAnyRequiredFieldIsBlank() {
        assertFalse(SettingsLogic.canConnect("", "r", "ak", "sk", working = false))
        assertFalse(SettingsLogic.canConnect("b", "", "ak", "sk", working = false))
        assertFalse(SettingsLogic.canConnect("b", "r", "", "sk", working = false))
        assertFalse(SettingsLogic.canConnect("b", "r", "ak", "", working = false))
    }

    @Test
    fun canConnectFalseWhileWorking() {
        // The endpoint and prefix are optional, so a full set still connects, but
        // an in-flight connect disables it regardless.
        assertFalse(SettingsLogic.canConnect("b", "r", "ak", "sk", working = true))
    }

    @Test
    fun optionalFieldTrimsAndKeepsAValue() {
        assertEquals("https://s3.example.com", SettingsLogic.optionalField("  https://s3.example.com  "))
    }

    @Test
    fun optionalFieldMapsBlankToNull() {
        assertNull(SettingsLogic.optionalField(""))
        assertNull(SettingsLogic.optionalField("   "))
    }
}
