import Foundation
import os.log

private let logger = Logger.visible("SettingsModel")

/// Loads and mutates the cloud-sync settings for one library. Bridge calls touch
/// SQLite, the keyring, and the network, so they run off the main actor; the
/// read-modify-write of the screen state happens here on the model, not in the
/// view (observable-mutate-on-the-state-not-the-view). The model owns the
/// concurrency: each method launches its own `Task`, so the view calls them
/// synchronously. The view iterates over the model's fields and renders them.
@MainActor
@Observable
final class SettingsModel {
    private let handle: AppHandle

    // The editable S3 form fields, seeded blank (form-seeding exemption). A
    // blank or whitespace-only endpoint or key prefix is mapped to nil in
    // connect() — its absence — so core receives None, never "".
    var bucket = ""
    var region = ""
    var endpoint = ""
    var keyPrefix = ""
    var accessKey = ""
    var secretKey = ""

    /// Whether a provider is configured and whether the sync loop is running.
    private(set) var status: BridgeSyncStatus?
    /// Pending cloud-outbox counts.
    private(set) var outbox: BridgeOutboxSnapshot?
    /// A connect/disconnect call is in flight (drives the "Connecting…" state and
    /// disables the buttons). Local UI state for the in-flight gesture, not a
    /// domain value.
    private(set) var working = false
    /// The last connect/disconnect failure, cleared on the next attempt.
    private(set) var errorMessage: String?

    init(handle: AppHandle) {
        self.handle = handle
    }

    /// Whether the connect button has the minimum required fields. Bucket,
    /// region, and both keys are required; endpoint and prefix are optional.
    var canConnect: Bool {
        !working && !bucket.isEmpty && !region.isEmpty && !accessKey.isEmpty && !secretKey.isEmpty
    }

    /// Whether a provider is configured (a Disconnect / Sync-now action makes
    /// sense).
    var isConnected: Bool {
        status?.configured ?? false
    }

    /// The one-line status: the in-flight connect, then the configured/ready
    /// state, with the pending delete count appended when there is work queued.
    /// Composed here on the model from the booleans and count the bridge provides
    /// plus the local in-flight flag, so the view renders it directly.
    var statusLine: String {
        if working {
            return "Connecting…"
        }
        guard let status, status.configured else {
            return "Not connected"
        }
        let base = status.ready ? "Synced" : "Connected (starting…)"
        let pending = outbox?.pendingDeletes ?? 0
        return pending > 0 ? "\(base) · \(pending) to delete" : base
    }

    /// Load the current sync status and outbox counts.
    func reload() {
        let handle = handle
        Task {
            let loaded = await Task.detached { () -> (BridgeSyncStatus, BridgeOutboxSnapshot)? in
                do {
                    return try (handle.syncStatus(), handle.outboxSnapshot())
                } catch {
                    logger.error("loading sync status failed: \(error.localizedDescription, privacy: .public)")
                    return nil
                }
            }.value
            if let (status, outbox) = loaded {
                self.status = status
                self.outbox = outbox
            }
        }
    }

    /// Probe and connect the S3 cloud home, then refresh the status. The bridge
    /// call probes the bucket and starts sync — network and a deep stack — so it
    /// runs off the main actor.
    func connect() {
        // Map a blank or whitespace-only optional box to nil at the form (its
        // absence); trim so a real value is sent without surrounding whitespace.
        let ep = endpoint.trimmingCharacters(in: .whitespacesAndNewlines)
        let prefix = keyPrefix.trimmingCharacters(in: .whitespacesAndNewlines)
        let config = BridgeS3Config(
            bucket: bucket,
            region: region,
            endpoint: ep.isEmpty ? nil : ep,
            keyPrefix: prefix.isEmpty ? nil : prefix,
            accessKey: accessKey,
            secretKey: secretKey
        )
        runAction("connecting S3") { try $0.saveS3Config(config: config) }
    }

    /// Disconnect the cloud provider, then refresh the status.
    func disconnect() {
        runAction("disconnecting sync") { try $0.disconnectSync() }
    }

    /// Request an immediate sync cycle, then refresh the status so the outbox
    /// counts reflect the drain. A no-op in the bridge when sync isn't connected.
    func triggerSync() {
        let handle = handle
        Task {
            await Task.detached { handle.triggerSync() }.value
            reload()
        }
    }

    /// Mark a connect/disconnect in flight, run the bridge write off the main
    /// actor, then clear the in-flight flag and reload the status. The error (or
    /// nil on success) lands in ``errorMessage`` for the view to show.
    private func runAction(_ description: String, _ write: @escaping @Sendable (AppHandle) throws -> Void) {
        errorMessage = nil
        working = true
        Task {
            let failure = await BridgeWrite.run(description, handle: handle, write)
            working = false
            errorMessage = failure
            reload()
        }
    }
}
