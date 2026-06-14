import Foundation
import os.log

private let logger = Logger.visible("AppSession")

/// The app-root lifecycle: the session is opening, failed to open, or open.
enum SessionState {
    case loading
    case failed(String)
    /// The open library: its handle and the id of its root house node.
    case open(handle: AppHandle, rootId: String)
}

/// Holds the one ``AppHandle`` for the process and publishes the current
/// ``SessionState`` for the root view to render. On first open it discovers the
/// library under the app's private data dir (creating the default one if none
/// exists), opens it, and reads the root node id. Joining or restoring a home
/// replaces the open library in place — a single active home — via
/// ``switchToHome(_:)``. A failure is published as ``state`` without disturbing
/// the open library, so the user can retry.
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
            state = .open(handle: current.handle, rootId: current.rootId)
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

        let opened = await Task.detached { () -> Result<(AppHandle, String, String), Error> in
            do {
                // Install the keyring before anything reads it — cloud sync stores
                // the identity keypair and the per-library encryption key there.
                initKeyring()
                let library = try discoverLibraries(dataDir: dataDir).first
                    ?? createLibrary(dataDir: dataDir)
                let handle = try initApp(dataDir: dataDir, libraryId: library.id)
                return .success((handle, try handle.rootNode().id, library.id))
            } catch {
                return .failure(error)
            }
        }.value

        switch opened {
        case let .success((handle, rootId, libraryId)):
            current = (handle, rootId, libraryId)
            state = .open(handle: handle, rootId: rootId)
        case let .failure(error):
            logger.error("opening library failed: \(error.localizedDescription, privacy: .public)")
            state = .failed(error.localizedDescription)
        }
    }

    /// Switch the active library to a home the user joined or restored: write the
    /// new library to disk (the joiner-side `join_library_from_invite` /
    /// `restore_library_from_code` call), open it, then remove the previously open
    /// library — a single active home. Returns nil on success or the failure
    /// message; on failure ``state`` and the open library are unchanged.
    ///
    /// Order matters: write and open the new library FIRST, and only remove the
    /// old one once the new handle is in hand, so a failed write or open leaves
    /// the old library intact and nothing is removed.
    func switchToHome(_ source: HomeSwitch) async -> String? {
        guard let previous = current else {
            // open() must have run before any sharing action is reachable.
            logger.error("switchToHome called before the session was open")
            return "The current home isn't open yet."
        }

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
                // The new library is open; dropping the old one can't strand us.
                try removeLibrary(dataDir: dataDir, libraryId: previous.libraryId)
                return .success((handle, rootId, library.id))
            } catch {
                return .failure(error)
            }
        }.value

        switch switched {
        case let .success((handle, rootId, libraryId)):
            current = (handle, rootId, libraryId)
            state = .open(handle: handle, rootId: rootId)
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
