package fm.bae.visible.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * [NodeDetailLogic] converts the node edit form between its stored shape (cents,
 * ISO date) and its editable shape (dollars string, UTC epoch millis). Pure
 * logic, no Android framework beyond the no-op [android.util.Log] the unparseable
 * paths log through (unit tests return default values), so it runs on the JVM
 * under testDebugUnitTest.
 */
class NodeDetailLogicTest {
    @Test
    fun centsAndDollarsRoundTrip() {
        // A whole-dollar and a cents amount both survive the cents → dollars
        // string → cents round trip.
        assertEquals("12.99", NodeDetailLogic.dollarsFromCents(1299))
        assertEquals(1299L, NodeDetailLogic.centsFromDollars("12.99"))
        assertEquals(500L, NodeDetailLogic.centsFromDollars(NodeDetailLogic.dollarsFromCents(500)))
    }

    @Test
    fun dollarsToCentsRoundsToTheNearestCent() {
        // Half-up rounding so a third-of-a-cent input lands on a whole cent.
        assertEquals(1299L, NodeDetailLogic.centsFromDollars("12.985"))
    }

    @Test
    fun centsFromBlankIsNull() {
        assertNull(NodeDetailLogic.centsFromDollars(""))
        assertNull(NodeDetailLogic.centsFromDollars("   "))
    }

    @Test
    fun centsFromUnparseableIsNull() {
        assertNull(NodeDetailLogic.centsFromDollars("not a number"))
    }

    @Test
    fun quantityFromTextParsesWholeNumbers() {
        assertEquals(7L, NodeDetailLogic.quantityFromText(" 7 "))
    }

    @Test
    fun quantityFromBlankIsNull() {
        assertNull(NodeDetailLogic.quantityFromText(""))
    }

    @Test
    fun quantityFromUnparseableIsNull() {
        assertNull(NodeDetailLogic.quantityFromText("1.5"))
        assertNull(NodeDetailLogic.quantityFromText("abc"))
    }

    @Test
    fun isoAndMillisRoundTrip() {
        val iso = "2024-01-02"
        val millis = NodeDetailLogic.millisFromIso(iso)!!
        assertEquals(iso, NodeDetailLogic.isoFromMillis(millis))
    }

    @Test
    fun millisFromIsoIsUtcStartOfDay() {
        // 2024-01-02 UTC start of day is 1704153600000 ms since the epoch.
        assertEquals(1704153600000L, NodeDetailLogic.millisFromIso("2024-01-02"))
    }

    @Test
    fun millisFromUnparseableIsoIsNull() {
        assertNull(NodeDetailLogic.millisFromIso("not a date"))
    }
}
