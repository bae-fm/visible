import Foundation
import os.log

private let logger = Logger.visible("NodeDetailModel")

/// What the edit screen is showing while it loads or renders one node's details.
enum NodeDetailContent {
    case loading
    case failed(String)
    case loaded
}

/// Loads and edits one node's attributes and tags. Bridge calls touch SQLite so
/// they run off the main actor; the read-modify-write of the form state happens
/// here on the model, not in the view
/// (observable-mutate-on-the-state-not-the-view). The model owns the concurrency:
/// each method launches its own `Task`, so the view calls them synchronously and
/// renders the form fields.
///
/// The form holds each attribute in its editable representation: quantity and the
/// text fields as strings, value in dollars (seeded from the stored cents), and
/// the acquired date as a `Date?` for the native picker (seeded from the stored
/// ISO `YYYY-MM-DD` string). These conversions are form-seeding — they live here
/// on the model, not in the view body, and invert on save.
@MainActor
@Observable
final class NodeDetailModel {
    private let handle: AppHandle
    private let nodeId: String

    private(set) var content: NodeDetailContent = .loading

    // The editable form fields, seeded from the loaded detail. Blank text fields
    // map to absence on save (the model trims, core also normalizes).
    var quantity = ""
    var valueDollars = ""
    var acquiredDate: Date?
    var notes = ""
    var serial = ""
    var barcode = ""

    private(set) var tags: [String] = []
    var newTag = ""

    /// A save or tag write in flight, so the view can disable the controls while a
    /// write runs. Local UI state for the in-flight gesture, not a domain value.
    private(set) var working = false
    /// The last write failure, cleared on the next attempt.
    private(set) var errorMessage: String?

    init(handle: AppHandle, nodeId: String) {
        self.handle = handle
        self.nodeId = nodeId
    }

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

    /// Load the node's detail and seed the form from it.
    func reload() {
        let handle = handle
        let nodeId = nodeId
        Task {
            // Swift's `Result.Failure` must be an `Error`, and the bridge surfaces
            // a `String` message, so the off-main load returns a small outcome enum.
            let outcome = await Task.detached { () -> LoadOutcome in
                do {
                    return .loaded(try handle.nodeDetail(id: nodeId))
                } catch {
                    logger.error("loading detail for \(nodeId, privacy: .public) failed: \(error.localizedDescription, privacy: .public)")
                    return .failed(error.localizedDescription)
                }
            }.value
            switch outcome {
            case let .loaded(detail):
                seed(from: detail)
                content = .loaded
            case let .failed(message):
                content = .failed(message)
            }
        }
    }

    /// The result of the off-main detail load, handed back to the main actor.
    private enum LoadOutcome: Sendable {
        case loaded(BridgeNodeDetail)
        case failed(String)
    }

    /// Seed the editable form fields from the loaded detail (form-seeding): cents
    /// render as dollars, the ISO date string parses into a `Date?`, and an absent
    /// field seeds blank.
    private func seed(from detail: BridgeNodeDetail) {
        quantity = detail.quantity.map(String.init) ?? ""
        valueDollars = detail.valueCents.map(Self.dollars(fromCents:)) ?? ""
        acquiredDate = detail.acquiredAt.flatMap { Self.isoDate.date(from: $0) }
        notes = detail.notes ?? ""
        serial = detail.serial ?? ""
        barcode = detail.barcode ?? ""
        tags = detail.tags
    }

    /// Save the form's attributes. Quantity and value parse from their editable
    /// strings (value dollars → cents); the acquired date renders back to the ISO
    /// string; blank text fields map to nil (the model trims, core also
    /// normalizes). Reloads after the write so the form reflects the stored state.
    func save() {
        let attributes = formAttributes()
        errorMessage = nil
        working = true
        let handle = handle
        let nodeId = nodeId
        Task {
            let failure = await BridgeWrite.run("saving attributes on \(nodeId)", handle: handle) {
                try $0.updateNodeAttributes(
                    id: nodeId,
                    quantity: attributes.quantity,
                    notes: attributes.notes,
                    valueCents: attributes.valueCents,
                    acquiredAt: attributes.acquiredAt,
                    serial: attributes.serial,
                    barcode: attributes.barcode
                )
            }
            working = false
            errorMessage = failure
            if failure == nil { reload() }
        }
    }

    /// Add the typed tag, clear the field, and reload the tags. The core trims and
    /// ignores a blank tag, so an empty field is a no-op there too.
    func addTag() {
        let tag = newTag
        newTag = ""
        runTagWrite("adding tag to \(nodeId)") { try $0.addNodeTag(id: self.nodeId, tag: tag) }
    }

    /// Remove `tag` and reload the tags.
    func removeTag(_ tag: String) {
        runTagWrite("removing tag from \(nodeId)") { try $0.removeNodeTag(id: self.nodeId, tag: tag) }
    }

    /// Run a tag write off the main actor, then reload the tags so the chip list
    /// reflects the change. Surfaces a failure in `errorMessage`.
    private func runTagWrite(_ description: String, _ write: @escaping @Sendable (AppHandle) throws -> Void) {
        errorMessage = nil
        working = true
        let handle = handle
        let nodeId = nodeId
        Task {
            let failure = await BridgeWrite.run(description, handle: handle, write)
            working = false
            errorMessage = failure
            if failure == nil { reloadTags(handle: handle, nodeId: nodeId) }
        }
    }

    /// Reload just the tags after a tag write (the attributes the user is editing
    /// stay untouched, so the whole form is not re-seeded).
    private func reloadTags(handle: AppHandle, nodeId: String) {
        Task {
            let loaded = await Task.detached { () -> [String]? in
                do {
                    return try handle.nodeDetail(id: nodeId).tags
                } catch {
                    logger.error("reloading tags for \(nodeId, privacy: .public) failed: \(error.localizedDescription, privacy: .public)")
                    return nil
                }
            }.value
            if let loaded { tags = loaded }
        }
    }

    /// The form fields parsed back into the stored attribute shape: quantity and
    /// value from their editable strings, the date back to ISO, blank text to nil.
    private func formAttributes() -> NodeAttributes {
        NodeAttributes(
            quantity: Self.int64(from: quantity),
            notes: Self.blankToNil(notes),
            valueCents: Self.cents(fromDollars: valueDollars),
            acquiredAt: acquiredDate.map { Self.isoDate.string(from: $0) },
            serial: Self.blankToNil(serial),
            barcode: Self.blankToNil(barcode)
        )
    }

    /// A trimmed text field, or nil when blank — the absence a cleared field means.
    private static func blankToNil(_ text: String) -> String? {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
    }

    /// Parse a whole-number string into `Int64?`; blank or non-numeric is nil.
    private static func int64(from text: String) -> Int64? {
        Int64(text.trimmingCharacters(in: .whitespacesAndNewlines))
    }

    /// Cents → a dollars string with two decimal places (form-seeding).
    private static func dollars(fromCents cents: Int64) -> String {
        let value = Decimal(cents) / 100
        return NSDecimalNumber(decimal: value).stringValue
    }

    /// A dollars string → whole cents, rounded to the nearest cent; blank or
    /// unparseable is nil. Uses `Decimal` so the cent rounding doesn't inherit
    /// binary-float drift.
    private static func cents(fromDollars text: String) -> Int64? {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty { return nil }
        guard let dollars = Decimal(string: trimmed, locale: Locale(identifier: "en_US_POSIX")) else {
            return nil
        }
        var cents = dollars * 100
        var rounded = Decimal()
        NSDecimalRound(&rounded, &cents, 0, .plain)
        return NSDecimalNumber(decimal: rounded).int64Value
    }
}

/// The node attributes parsed from the edit form, in their stored shape (value in
/// cents, date as the ISO string). The bridge call takes them as separate
/// arguments; this groups them so the parse stays in one place on the model.
private struct NodeAttributes {
    let quantity: Int64?
    let notes: String?
    let valueCents: Int64?
    let acquiredAt: String?
    let serial: String?
    let barcode: String?
}
