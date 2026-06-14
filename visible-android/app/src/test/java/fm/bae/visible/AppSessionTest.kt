package fm.bae.visible

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * [shouldRemovePrevious] decides whether switching homes removes the library it
 * replaced. The single-active-home model removes only when a different library
 * takes over; re-joining/restoring the active home reopens the same id and must
 * keep it, or the switch would delete the directory + keyring key the just-opened
 * handle is backed by.
 */
class AppSessionTest {
    @Test
    fun removesPreviousWhenIdDiffers() {
        assertTrue(shouldRemovePrevious(previousId = "home-a", newId = "home-b"))
    }

    @Test
    fun keepsPreviousWhenIdMatches() {
        assertFalse(shouldRemovePrevious(previousId = "home-a", newId = "home-a"))
    }

    @Test
    fun keepsNothingOnFirstRun() {
        assertFalse(shouldRemovePrevious(previousId = null, newId = "home-a"))
    }
}
