import Foundation
import os.log

private let logger = Logger.visible("NodeDetailLogic")

/// The pure form conversions behind ``NodeDetailModel``: the value cents↔dollars
/// pair, the acquired-date ISO↔`Date` pair, and the blank/quantity parsing the
/// form seeds from and saves back through. No `AppHandle` or `@Observable`
/// state, so the round trips and the unparseable→nil cases are exercised
/// directly. The model delegates each conversion here.
enum NodeDetailLogic {
    /// The ISO `YYYY-MM-DD` formatter the acquired date seeds from and saves back
    /// to. Fixed locale and UTC so a bare calendar date round-trips without a
    /// time-zone shift.
    private static let isoDate: DateFormatter = {
        let f = DateFormatter()
        f.locale = Locale(identifier: "en_US_POSIX")
        f.timeZone = TimeZone(identifier: "UTC")
        f.dateFormat = "yyyy-MM-dd"
        return f
    }()

    /// Parse a stored ISO `YYYY-MM-DD` string into a `Date?` for the native
    /// picker (form-seeding); a string that doesn't parse seeds no date — logged,
    /// like the Android counterpart and the value/quantity parses here.
    static func dateFromIso(_ iso: String) -> Date? {
        if let date = isoDate.date(from: iso) { return date }
        logger.debug("acquired date \(iso, privacy: .public) is not an ISO date; showing no date")
        return nil
    }

    /// Render a picked `Date` back to the stored ISO `YYYY-MM-DD` string.
    static func isoFromDate(_ date: Date) -> String {
        isoDate.string(from: date)
    }

    /// A trimmed text field, or nil when blank — the absence a cleared field means.
    static func blankToNil(_ text: String) -> String? {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
    }

    /// Parse a whole-number string into `Int64?`. Blank is nil (a cleared field,
    /// the form-seeding exemption). A non-blank string that doesn't parse is also
    /// nil, but that's a dropped value on the save path, so it's logged.
    static func int64(from text: String) -> Int64? {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty { return nil }
        guard let value = Int64(trimmed) else {
            logger.warning("quantity \(trimmed, privacy: .public) is not a whole number; saving no quantity")
            return nil
        }
        return value
    }

    /// Cents → a dollars string with two decimal places (form-seeding).
    static func dollars(fromCents cents: Int64) -> String {
        let value = Decimal(cents) / 100
        return NSDecimalNumber(decimal: value).stringValue
    }

    /// A dollars string → whole cents, rounded to the nearest cent; blank or
    /// unparseable is nil. Uses `Decimal` so the cent rounding doesn't inherit
    /// binary-float drift.
    static func cents(fromDollars text: String) -> Int64? {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty { return nil }
        guard let dollars = Decimal(string: trimmed, locale: Locale(identifier: "en_US_POSIX")) else {
            logger.warning("value \(trimmed, privacy: .public) is not a parseable amount; saving no value")
            return nil
        }
        var cents = dollars * 100
        var rounded = Decimal()
        NSDecimalRound(&rounded, &cents, 0, .plain)
        return NSDecimalNumber(decimal: rounded).int64Value
    }
}
