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

/// Holds the one ``AppHandle`` for the process. On first open it discovers the
/// library under the app's private data dir (creating the default one if none
/// exists), opens it, and reads the root node id. There is a single local
/// library that stays open for the process lifetime — no unlock and nothing to
/// switch to or dispose around. Only the open result is cached, so a transient
/// failure can be retried by re-invoking ``open()``.
@MainActor
final class AppSession {
    private var opened: (handle: AppHandle, rootId: String)?

    /// Open the library and produce the resulting ``SessionState``. The bridge
    /// calls touch SQLite, so they run off the main actor; reuse the already
    /// open session on a re-entry (e.g. a retry after a transient failure). A
    /// failure is never cached, so a caller can retry by calling this again.
    func open() async -> SessionState {
        if let opened {
            return .open(handle: opened.handle, rootId: opened.rootId)
        }

        let dataDir: String
        do {
            dataDir = try Self.dataDirectory()
        } catch {
            logger.error("locating data directory failed: \(error.localizedDescription, privacy: .public)")
            return .failed(error.localizedDescription)
        }

        let next = await Task.detached {
            do {
                // Install the keyring before anything reads it — cloud sync stores
                // the identity keypair and the per-library encryption key there.
                initKeyring()
                let library = try discoverLibraries(dataDir: dataDir).first
                    ?? createLibrary(dataDir: dataDir)
                let handle = try initApp(dataDir: dataDir, libraryId: library.id)
                return SessionState.open(handle: handle, rootId: try handle.rootNode().id)
            } catch {
                logger.error("opening library failed: \(error.localizedDescription, privacy: .public)")
                return SessionState.failed(error.localizedDescription)
            }
        }.value

        if case let .open(handle, rootId) = next {
            opened = (handle, rootId)
        }
        return next
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
