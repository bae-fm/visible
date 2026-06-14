import Foundation
import os.log

private let logger = Logger.visible("SettingsModel")

/// Loads and mutates the settings screen for one open home: its name (rename),
/// cloud sync, and starting a fresh home that replaces this one. Bridge calls
/// touch SQLite, the keyring, and the network, so they run off the main actor; the
/// read-modify-write of the screen state happens here on the model, not in the
/// view (observable-mutate-on-the-state-not-the-view). The model owns the
/// concurrency: each method launches its own `Task`, so the view calls them
/// synchronously. The view iterates over the model's fields and renders them.
@MainActor
@Observable
final class SettingsModel {
    private let handle: AppHandle
    private let session: AppSession
    /// The root house node id — the home's name is this node's name, so a rename
    /// renames the root.
    private let rootId: String
    /// This home's library id, shown in About for support.
    let libraryId: String

    /// The home's current name (the root node's name), shown in "This Home" and
    /// used to seed the rename sheet. Loaded on appear.
    private(set) var homeName: Loadable<String> = .loading

    // The editable S3 form fields, seeded blank (form-seeding exemption). A
    // blank or whitespace-only endpoint or key prefix is mapped to nil in
    // connect() — its absence — so core receives None, never "".
    var bucket = ""
    var region = ""
    var endpoint = ""
    var keyPrefix = ""
    var accessKey = ""
    var secretKey = ""

    /// The name for a fresh home, seeded with a suggestion the user can edit
    /// (form-seeding exemption). Trimmed on submit.
    var newHomeName = "Home"
    /// The name the user is confirming a "start a new home" replace for; nil when
    /// no confirm is up. Set when the user taps "Start a new home" with a non-blank
    /// name and cleared on confirm or dismiss.
    private(set) var pendingNewHome: String?

    /// Whether a provider is configured and whether the sync loop is running.
    private(set) var status: BridgeSyncStatus?
    /// Pending cloud-outbox counts.
    private(set) var outbox: BridgeOutboxSnapshot?
    /// A connect/disconnect/rename/switch call is in flight (drives the
    /// "Connecting…" state and disables the buttons). Local UI state for the
    /// in-flight gesture, not a domain value.
    private(set) var working = false
    /// The last failure, cleared on the next attempt.
    private(set) var errorMessage: String?

    init(handle: AppHandle, session: AppSession, rootId: String, libraryId: String) {
        self.handle = handle
        self.session = session
        self.rootId = rootId
        self.libraryId = libraryId
    }

    /// The app version and build for the About section, read from the bundle (a
    /// platform mechanism, so it stays UI-side). "0.1.0 (1)" form. Both keys are
    /// populated from MARKETING_VERSION / CURRENT_PROJECT_VERSION at build, so
    /// their absence is a packaging fault — logged, not silently faked.
    var appVersion: String {
        let info = Bundle.main.infoDictionary
        guard let version = info?["CFBundleShortVersionString"] as? String,
              let build = info?["CFBundleVersion"] as? String else {
            logger.error("bundle is missing CFBundleShortVersionString / CFBundleVersion")
            return "unavailable"
        }
        return "\(version) (\(build))"
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

    /// Load the home's current name from the root node, so "This Home" shows it
    /// and the rename sheet seeds from it.
    func loadHome() {
        let handle = handle
        let rootId = rootId
        Task {
            homeName = await Task.detached { () -> Loadable<String> in
                do {
                    let root = try handle.getNode(id: rootId)
                    guard let root else {
                        logger.error("root node \(rootId, privacy: .public) not found loading the home name")
                        return .failed("This home's root is missing.")
                    }
                    // The root always has a name (create sets it); render an empty
                    // string only if the type's optionality ever surfaces.
                    return .loaded(root.name ?? "")
                } catch {
                    logger.error("loading the home name failed: \(error.localizedDescription, privacy: .public)")
                    return .failed(error.localizedDescription)
                }
            }.value
        }
    }

    /// Rename the home (the root node), then reload its name. The browse root
    /// reflects the new name on its next appear.
    func renameHome(_ name: String) {
        let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        let rootId = rootId
        errorMessage = nil
        working = true
        Task {
            let failure = await BridgeWrite.run("renaming the home", handle: handle) {
                try $0.renameNode(id: rootId, name: trimmed)
            }
            working = false
            errorMessage = failure
            loadHome()
        }
    }

    /// Open the replace confirmation for starting a fresh home with the trimmed
    /// new-home name. Validates the field is non-empty.
    func confirmNewHome() {
        let name = newHomeName.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !name.isEmpty else {
            errorMessage = "Give the new home a name first."
            return
        }
        pendingNewHome = name
    }

    func dismissNewHome() {
        pendingNewHome = nil
    }

    /// Carry out the confirmed "start a new home": ``AppSession/switchToHome(_:)``
    /// creates the fresh local home, opens it, and removes the current one. A
    /// failure there leaves the current home intact.
    func startNewHome() {
        guard let name = pendingNewHome else {
            logger.error("startNewHome called with no pending new home")
            return
        }
        pendingNewHome = nil
        errorMessage = nil
        working = true
        let session = session
        Task {
            errorMessage = await session.switchToHome(.create(name))
            working = false
        }
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
