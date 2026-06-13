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
    case addChild
    case rename(target: BridgeNode)
    case confirmDelete(target: BridgeNode)

    var id: String {
        switch self {
        case .addChild: "addChild"
        case let .rename(target): "rename-\(target.id)"
        case let .confirmDelete(target): "confirmDelete-\(target.id)"
        }
    }
}

/// Loads and mutates one node's browse state. Bridge calls touch SQLite so they
/// run off the main actor; the read-modify-write of the screen state happens
/// here on the model, not in the view
/// (observable-mutate-on-the-state-not-the-view). The view iterates over
/// ``content`` and renders it.
@MainActor
@Observable
final class BrowseModel {
    private let handle: AppHandle
    private let nodeId: String

    private(set) var content: BrowseContent = .loading
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

    func reload() async {
        let handle = handle
        let nodeId = nodeId
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

    func addChild(name: String) async {
        dialog = nil
        await mutate("creating child of \(nodeId)") { _ = try $0.createNode(parentId: self.nodeId, name: name) }
    }

    func rename(id: String, name: String) async {
        dialog = nil
        await mutate("renaming \(id)") { try $0.renameNode(id: id, name: name) }
    }

    /// Delete `id`. Deleting a child reloads this screen; deleting this node
    /// itself signals the screen to pop to the parent (reloading a deleted node
    /// would only show a dead screen).
    func delete(id: String) async {
        dialog = nil
        if id == nodeId {
            if let error = await runWrite("deleting \(nodeId)", { try $0.deleteNode(id: self.nodeId) }) {
                content = .failed(error)
            } else {
                deletedSelf.send(())
            }
        } else {
            await mutate("deleting \(id)") { try $0.deleteNode(id: id) }
        }
    }

    func setImage(_ bytes: Data) async {
        await mutate("setting image on \(nodeId)") { try $0.setNodeImage(id: self.nodeId, bytes: bytes) }
    }

    /// The local file path for `imageId` if its file exists, else nil. The
    /// bridge call does no database work (it is a filesystem existence check), so
    /// the image views call it directly on the render path.
    func imagePath(_ imageId: String) -> String? {
        let path = handle.imagePathIfExists(imageId: imageId)
        if path == nil {
            // The node references an image whose file isn't on disk. Today this
            // can't happen (set_image writes the file before recording the id);
            // once libraries sync it is the normal "row arrived, blob not pulled
            // yet" case. Either way the caller renders the placeholder.
            logger.debug("no image file for \(imageId, privacy: .public); showing placeholder")
        }
        return path
    }

    /// Runs a bridge write off the main actor, then reloads to reflect the new
    /// state, or surfaces the failure.
    private func mutate(_ description: String, _ write: @escaping @Sendable (AppHandle) throws -> Void) async {
        if let error = await runWrite(description, write) {
            content = .failed(error)
        } else {
            await reload()
        }
    }

    /// Runs a bridge write off the main actor; returns nil on success or the
    /// message on failure.
    private func runWrite(_ description: String, _ write: @escaping @Sendable (AppHandle) throws -> Void) async -> String? {
        let handle = handle
        return await Task.detached {
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
