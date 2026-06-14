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

    // The editable S3 form fields, seeded blank (form-seeding exemption). Empty
    // optional boxes (endpoint, key prefix) map back to nil on connect.
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
    /// state, with the pending outbox counts appended when there is work queued.
    /// Composed here on the model from the booleans and counts the bridge provides
    /// plus the local in-flight flag, so the view renders it directly.
    var statusLine: String {
        if working {
            return "Connecting…"
        }
        guard let status, status.configured else {
            return "Not connected"
        }
        let base = status.ready ? "Synced" : "Connected (starting…)"
        return base + pendingSuffix
    }

    /// `" · N to upload, M to delete"` when the outbox has pending work, else
    /// empty.
    private var pendingSuffix: String {
        guard let outbox else { return "" }
        var parts: [String] = []
        if outbox.pendingUploads > 0 { parts.append("\(outbox.pendingUploads) to upload") }
        if outbox.pendingDeletes > 0 { parts.append("\(outbox.pendingDeletes) to delete") }
        return parts.isEmpty ? "" : " · " + parts.joined(separator: ", ")
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
        errorMessage = nil
        working = true
        let handle = handle
        // Trim optional boxes back to nil when blank — the inverse of seeding.
        let config = BridgeS3Config(
            bucket: bucket,
            region: region,
            endpoint: endpoint.isEmpty ? nil : endpoint,
            keyPrefix: keyPrefix.isEmpty ? nil : keyPrefix,
            accessKey: accessKey,
            secretKey: secretKey
        )
        Task {
            let failure = await Task.detached { () -> String? in
                do {
                    try handle.saveS3Config(config: config)
                    return nil
                } catch {
                    logger.error("connecting S3 failed: \(error.localizedDescription, privacy: .public)")
                    return error.localizedDescription
                }
            }.value
            working = false
            errorMessage = failure
            reload()
        }
    }

    /// Disconnect the cloud provider, then refresh the status.
    func disconnect() {
        errorMessage = nil
        working = true
        let handle = handle
        Task {
            let failure = await Task.detached { () -> String? in
                do {
                    try handle.disconnectSync()
                    return nil
                } catch {
                    logger.error("disconnecting sync failed: \(error.localizedDescription, privacy: .public)")
                    return error.localizedDescription
                }
            }.value
            working = false
            errorMessage = failure
            reload()
        }
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
}
