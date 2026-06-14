import Foundation

/// The pure derivations behind ``SettingsModel``'s status line and connect
/// gate, with no `AppHandle` or `@Observable` state — the model reads its fields
/// and the bridge records, then delegates the decision here. Pulled out so the
/// derivations are exercised directly, without standing up a model.
enum SettingsLogic {
    /// The one-line settings status: the in-flight connect first, then the
    /// configured/ready state, with the pending delete count appended when there
    /// is work queued.
    static func statusLine(
        working: Bool,
        configured: Bool,
        ready: Bool,
        pendingDeletes: UInt64
    ) -> String {
        if working {
            return "Connecting…"
        }
        guard configured else {
            return "Not connected"
        }
        let base = ready ? "Synced" : "Connected (starting…)"
        return pendingDeletes > 0 ? "\(base) · \(pendingDeletes) to delete" : base
    }

    /// Whether the connect button has the minimum required fields. Bucket,
    /// region, and both keys are required; endpoint and prefix are optional. A
    /// connect already in flight (`working`) also disables it.
    static func canConnect(
        bucket: String,
        region: String,
        accessKey: String,
        secretKey: String,
        working: Bool
    ) -> Bool {
        !working && !bucket.isEmpty && !region.isEmpty && !accessKey.isEmpty && !secretKey.isEmpty
    }

    /// Map an optional S3 form box (endpoint or key prefix) to its absence: trim
    /// surrounding whitespace, and treat a blank or whitespace-only box as `nil`
    /// so core receives `None`, never `""`.
    static func optionalField(_ text: String) -> String? {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
    }
}
