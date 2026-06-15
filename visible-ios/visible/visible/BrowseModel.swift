import Combine
import Foundation
import os.log

private let logger = Logger.visible("BrowseModel")

/// What the screen is showing while it loads or renders one node.
enum BrowseContent {
    case loading
    case failed(String)
    case loaded(node: BridgeNode, children: [BridgeNode])
}

/// The dialog currently open over the screen, if any.
enum BrowseDialog: Identifiable {
    case rename(target: BridgeNode)
    case confirmDelete(target: BridgeNode)
    case confirmRemovePhoto

    var id: String {
        switch self {
        case let .rename(target): "rename-\(target.id)"
        case let .confirmDelete(target): "confirmDelete-\(target.id)"
        case .confirmRemovePhoto: "confirmRemovePhoto"
        }
    }
}

/// Loads and mutates one node's browse state. Bridge calls touch SQLite so they
/// run off the main actor; the read-modify-write of the screen state happens
/// here on the model, not in the view
/// (observable-mutate-on-the-state-not-the-view). The model also owns the
/// concurrency: every method that does async work launches its own `Task`, so
/// the view calls them synchronously and never wraps a model call in an ad-hoc
/// `Task`. The view iterates over ``content`` and renders it.
@MainActor
@Observable
final class BrowseModel {
    private let handle: AppHandle
    private let nodeId: String

    private(set) var content: BrowseContent = .loading
    // Writable so `.sheet(item:)` can clear it when the user swipes the sheet
    // down; the OPENING transitions go through `openRename`/`openDelete` so the
    // view never assigns a dialog state directly.
    var dialog: BrowseDialog?

    // A one-shot signal that this node was deleted and the screen showing it
    // should pop back to its parent. A delete is a command, not a state the
    // screen reads, so it goes through a subject rather than an observable flag
    // (state-describes-what-is-not-what-should-happen).
    @ObservationIgnored
    let deletedSelf = PassthroughSubject<Void, Never>()

    init(handle: AppHandle, nodeId: String) {
        self.handle = handle
        self.nodeId = nodeId
    }

    func reload() {
        let handle = handle
        let nodeId = nodeId
        Task {
            content = await Task.detached {
                do {
                    guard let node = try handle.getNode(id: nodeId) else {
                        return BrowseContent.failed("This item no longer exists.")
                    }
                    return try BrowseContent.loaded(node: node, children: handle.children(parentId: nodeId))
                } catch {
                    logger.error("loading node \(nodeId, privacy: .public) failed: \(error.localizedDescription, privacy: .public)")
                    return BrowseContent.failed(error.localizedDescription)
                }
            }.value
        }
    }

    func openRename(_ node: BridgeNode) {
        dialog = .rename(target: node)
    }

    func openDelete(_ node: BridgeNode) {
        dialog = .confirmDelete(target: node)
    }

    func openRemovePhoto() {
        dialog = .confirmRemovePhoto
    }

    func dismissDialog() {
        dialog = nil
    }

    /// Create a new child under this node carrying `bytes` as its photo. The
    /// child starts untitled (name = nil) — the photo is the thing's identity
    /// until it is renamed. The node and its image are written in one atomic
    /// core call, so the child never appears without its photo.
    func addChildWithPhoto(_ bytes: Data) {
        mutate("creating child of \(nodeId) with photo") {
            _ = try $0.createNodeWithImage(parentId: self.nodeId, bytes: bytes)
        }
    }

    func rename(id: String, name: String) {
        dialog = nil
        mutate("renaming \(id)") { try $0.renameNode(id: id, name: name) }
    }

    /// Delete `id`. Deleting a child reloads this screen; deleting this node
    /// itself signals the screen to pop to the parent (reloading a deleted node
    /// would only show a dead screen).
    func delete(id: String) {
        dialog = nil
        if id == nodeId {
            Task {
                let error = await BridgeWrite.run("deleting \(nodeId)", handle: handle) {
                    try $0.deleteNode(id: self.nodeId)
                }
                if let error {
                    content = .failed(error)
                } else {
                    deletedSelf.send(())
                }
            }
        } else {
            mutate("deleting \(id)") { try $0.deleteNode(id: id) }
        }
    }

    func setImage(_ bytes: Data) {
        mutate("setting image on \(nodeId)") { try $0.setNodeImage(id: self.nodeId, bytes: bytes) }
    }

    func removePhoto() {
        dialog = nil
        mutate("clearing image on \(nodeId)") { try $0.clearNodeImage(id: self.nodeId) }
    }

    /// The local file path for `imageId` if its file exists, else nil; the image
    /// views call it on the render path.
    func imagePath(_ imageId: String) -> String? {
        ImagePath.resolve(handle, imageId)
    }

    /// Runs a bridge write off the main actor, then reloads to reflect the new
    /// state, or surfaces the failure. Launches its own task so callers stay
    /// synchronous.
    private func mutate(_ description: String, _ write: @escaping @Sendable (AppHandle) throws -> Void) {
        Task {
            if let error = await BridgeWrite.run(description, handle: handle, write) {
                content = .failed(error)
            } else {
                reload()
            }
        }
    }
}
