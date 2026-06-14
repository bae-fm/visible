import Foundation
import os.log

private let logger = Logger.visible("AppSession")

/// The app-root lifecycle: opening, no library yet (first run), failed to open,
/// or open.
enum SessionState {
    case loading
    /// No library on this device yet — the onboarding Welcome screen creates or
    /// joins the first home. Reached only on first run; once a home exists the
    /// session never returns here (switching homes replaces in place).
    case onboarding
    case failed(String)
    /// The open library: its handle, the id of its root house node, and its
    /// library id (shown in Settings ▸ About for support).
    case open(handle: AppHandle, rootId: String, libraryId: String)
}

/// Holds the one ``AppHandle`` for the process and publishes the current
/// ``SessionState`` for the root view to render. On first open it discovers the
/// library under the app's private data dir; if one exists it opens it and reads
/// the root node id, and if none exists it publishes ``SessionState/onboarding``
/// for the Welcome screen to create or join the first home (it never auto-creates
/// a home). Creating, joining, or restoring a home all go through
/// ``switchToHome(_:)``, which opens the new library before removing the old one —
/// a single active home — so a failure never strands the user. A failure is
/// published as ``state`` without disturbing the open library, so the user can
/// retry.
@MainActor
@Observable
final class AppSession {
    private(set) var state: SessionState = .loading

    /// The open library's handle, root node id, and library id. The library id is
    /// the one ``switchToHome(_:)`` removes when a new home replaces this one. nil
    /// until the first successful open.
    private var current: (handle: AppHandle, rootId: String, libraryId: String)?

    /// Open the library and publish the resulting ``state``. The bridge calls
    /// touch SQLite, so they run off the main actor; reuse the already open
    /// session on a re-entry (e.g. a retry after a transient failure). A failure
    /// is never cached, so the root view's Retry calls this again.
    func open() async {
        if let current {
            state = .open(handle: current.handle, rootId: current.rootId, libraryId: current.libraryId)
            return
        }

        let dataDir: String
        do {
            dataDir = try Self.dataDirectory()
        } catch {
            logger.error("locating data directory failed: \(error.localizedDescription, privacy: .public)")
            state = .failed(error.localizedDescription)
            return
        }

        let opened = await Task.detached { () -> Result<(AppHandle, String, String)?, Error> in
            do {
                // Install the keyring before anything reads it — cloud sync stores
                // the identity keypair and the per-library encryption key there.
                initKeyring()
                // First run has no library: nil tells the caller to onboard rather
                // than auto-creating a home.
                guard let library = try discoverLibraries(dataDir: dataDir).first else {
                    return .success(nil)
                }
                let handle = try initApp(dataDir: dataDir, libraryId: library.id)
                return .success((handle, try handle.rootNode().id, library.id))
            } catch {
                return .failure(error)
            }
        }.value

        switch opened {
        case let .success(.some((handle, rootId, libraryId))):
            current = (handle, rootId, libraryId)
            state = .open(handle: handle, rootId: rootId, libraryId: libraryId)
        case .success(.none):
            state = .onboarding
        case let .failure(error):
            logger.error("opening library failed: \(error.localizedDescription, privacy: .public)")
            state = .failed(error.localizedDescription)
        }
    }

    /// Create the first home (onboarding "create a home"). No prior library to
    /// remove. Returns nil on success or the failure message.
    func createHome(name: String) async -> String? {
        await switchToHome(.create(name))
    }

    /// Join the first home from an invite code (onboarding "join a home"). No
    /// prior library to remove. Returns nil on success or the failure message.
    func joinHome(code: String) async -> String? {
        await switchToHome(.join(code))
    }

    /// Restore the first home from a restore code (onboarding "restore a home").
    /// No prior library to remove. Returns nil on success or the failure message.
    func restoreHome(code: String) async -> String? {
        await switchToHome(.restore(code))
    }

    /// Make `source`'s home the active one: write the new library to disk (create
    /// a fresh local home, or the joiner-side `join_library_from_invite` /
    /// `restore_library_from_code` download), open it, then remove the previously
    /// open library if there was one — a single active home. Drives both
    /// onboarding (no prior home) and the settings/sharing switch (replacing the
    /// current home). Returns nil on success or the failure message; on failure
    /// ``state`` and any open library are unchanged.
    ///
    /// Order matters: write and open the new library FIRST, and only remove the
    /// old one once the new handle is in hand, so a failed write or open leaves
    /// the old library intact and nothing is removed.
    func switchToHome(_ source: HomeSwitch) async -> String? {
        let previous = current

        let dataDir: String
        do {
            dataDir = try Self.dataDirectory()
        } catch {
            logger.error("locating data directory failed: \(error.localizedDescription, privacy: .public)")
            return error.localizedDescription
        }

        let switched = await Task.detached { () -> Result<(AppHandle, String, String), Error> in
            do {
                let library = try source.writeLibrary(dataDir: dataDir)
                let handle = try initApp(dataDir: dataDir, libraryId: library.id)
                let rootId = try handle.rootNode().id
                // The new library is open; dropping the old one (if any) can't
                // strand us.
                if let previous {
                    try removeLibrary(dataDir: dataDir, libraryId: previous.libraryId)
                }
                return .success((handle, rootId, library.id))
            } catch {
                return .failure(error)
            }
        }.value

        switch switched {
        case let .success((handle, rootId, libraryId)):
            current = (handle, rootId, libraryId)
            state = .open(handle: handle, rootId: rootId, libraryId: libraryId)
            return nil
        case let .failure(error):
            logger.error("\(source.logLabel, privacy: .public) failed: \(error.localizedDescription, privacy: .public)")
            return error.localizedDescription
        }
    }

    /// Absolute path to the app's Application Support directory, created if
    /// absent. visible-core writes its library tree and config under here.
    private static func dataDirectory() throws -> String {
        let fileManager = FileManager.default
        let base = fileManager.urls(
            for: .applicationSupportDirectory,
            in: .userDomainMask
        )[0]
        try fileManager.createDirectory(at: base, withIntermediateDirectories: true)
        return base.path
    }
}
