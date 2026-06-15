package fm.bae.visible.ui

import android.util.Log
import java.math.BigDecimal
import java.math.RoundingMode
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneOffset
import java.time.format.DateTimeFormatter
import java.time.format.DateTimeParseException

private const val TAG = "visible.NodeDetailLogic"

/**
 * The pure form conversions behind [NodeDetailViewModel]: the value cents↔dollars
 * pair, the acquired-date ISO↔UTC-epoch-millis pair (the unit the Material date
 * picker speaks), and the blank/quantity parsing the form seeds from and saves
 * back through. No [uniffi.visible_bridge.AppHandle] or observable state, so the
 * round trips and the unparseable→null cases are exercised directly. The view
 * model delegates each conversion here.
 */
object NodeDetailLogic {
    private val ISO_DATE: DateTimeFormatter = DateTimeFormatter.ofPattern("yyyy-MM-dd")

    /**
     * A quantity string → [Long], or null. Blank is null (a cleared field, the
     * form-seeding exemption). A non-blank string that isn't a whole number is
     * also null, but that's a dropped value on the save path, so it's logged.
     */
    fun quantityFromText(text: String): Long? {
        val trimmed = text.trim()
        if (trimmed.isEmpty()) return null
        val value = trimmed.toLongOrNull()
        if (value == null) {
            Log.d(TAG, "quantity '$trimmed' is not a whole number; saving no quantity")
        }
        return value
    }

    /** Cents → a dollars string with two decimal places (form-seeding). */
    fun dollarsFromCents(cents: Long): String =
        BigDecimal(cents).movePointLeft(2).toPlainString()

    /**
     * A dollars string → whole cents, rounded to the nearest cent; blank or
     * unparseable is null. Uses [BigDecimal] so the cent rounding doesn't inherit
     * binary-float drift.
     */
    fun centsFromDollars(text: String): Long? {
        val trimmed = text.trim()
        if (trimmed.isEmpty()) return null
        return try {
            BigDecimal(trimmed).movePointRight(2).setScale(0, RoundingMode.HALF_UP).longValueExact()
        } catch (e: ArithmeticException) {
            Log.d(TAG, "value '$trimmed' is not a parseable amount; saving no value", e)
            null
        } catch (e: NumberFormatException) {
            Log.d(TAG, "value '$trimmed' is not a parseable amount; saving no value", e)
            null
        }
    }

    /** An ISO `YYYY-MM-DD` date → UTC start-of-day epoch millis (form-seeding). */
    fun millisFromIso(iso: String): Long? =
        try {
            LocalDate.parse(iso, ISO_DATE).atStartOfDay(ZoneOffset.UTC).toInstant().toEpochMilli()
        } catch (e: DateTimeParseException) {
            Log.d(TAG, "acquired date '$iso' is not an ISO date; showing no date", e)
            null
        }

    /** UTC epoch millis → an ISO `YYYY-MM-DD` date string. */
    fun isoFromMillis(millis: Long): String =
        Instant.ofEpochMilli(millis).atZone(ZoneOffset.UTC).toLocalDate().format(ISO_DATE)
}
