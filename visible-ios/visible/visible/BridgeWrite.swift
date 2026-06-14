import Foundation
import os.log

private let logger = Logger.visible("BridgeWrite")

/// Runs a bridge write off the main actor. Bridge writes touch SQLite, the
/// keyring, and the network, so they run on a detached task; returns nil on
/// success or the failure message, logged under [description], for the model to
/// surface.
enum BridgeWrite {
    static func run(
        _ description: String,
        handle: AppHandle,
        _ write: @escaping @Sendable (AppHandle) throws -> Void
    ) async -> String? {
        await Task.detached {
            do {
                try write(handle)
                return nil
            } catch {
                logger.error("\(description, privacy: .public) failed: \(error.localizedDescription, privacy: .public)")
                return error.localizedDescription
            }
        }.value
    }
}
