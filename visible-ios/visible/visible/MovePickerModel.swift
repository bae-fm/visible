import Combine
import Foundation
import os.log

private let logger = Logger.visible("MovePickerModel")

/// What the destination picker is showing at its current location.
enum MovePickerContent {
    case loading
    case failed(String)
    /// The children of the current location that are valid destinations (the
    /// moving node is omitted — see ``MovePickerModel``). The current location
    /// itself is the last element of ``MovePickerModel/path``, which drives the
    /// breadcrumb and "Move here".
    case loaded(children: [BridgeNode])
}

/// Drives the destination picker: a self-contained walk of the tree to choose a
/// new parent for `movingId`. It starts at the root house and descends into a
/// tapped node; the breadcrumb is the path of nodes from the root to the current
/// location, so the back action pops one level. "Move here" re-parents `movingId`
/// into the currently-shown node.
///
/// The moving node is omitted from every children list: you can't move a node
/// into itself, and since it can't be entered its descendants are unreachable, so
/// omitting the one node keeps the whole moving subtree out of the picker. Core
/// still rejects an out-of-band cycle (the omission is the affordance; core is
/// the guard).
///
/// Bridge calls touch SQLite so they run off the main actor; the state mutation
/// happens here on the model, not in the view
/// (observable-mutate-on-the-state-not-the-view). The view iterates over
/// ``content`` and renders it.
@MainActor
@Observable
final class MovePickerModel {
    private let handle: AppHandle
    private let movingId: String

    private(set) var content: MovePickerContent = .loading
    // The nodes from the root down to the current location, inclusive. The first
    // element is the root house; the last is the node whose children are shown.
    // Drives the breadcrumb and the back action.
    private(set) var path: [BridgeNode] = []

    // A one-shot signal that the move succeeded and the picker should dismiss.
    // A successful move is a command, not a state the view reads, so it goes
    // through a subject rather than an observable flag
    // (state-describes-what-is-not-what-should-happen).
    @ObservationIgnored
    let moved = PassthroughSubject<Void, Never>()

    init(handle: AppHandle, movingId: String) {
        self.handle = handle
        self.movingId = movingId
    }

    /// The node whose children are currently shown, or nil before the first load.
    var current: BridgeNode? { path.last }

    /// Load the root house and its children to start the walk. The picker is its
    /// own flow, so it reads the root from the bridge rather than being handed a
    /// root id from the browse stack.
    func start() {
        load(towards: nil, resettingTo: [])
    }

    /// Descend into `node`: show its children with it appended to the breadcrumb.
    func descend(into node: BridgeNode) {
        load(towards: node.id, resettingTo: path)
    }

    /// Go up one level: drop the current node and reload its parent's children.
    /// Does nothing at the root (the root is always the first breadcrumb element).
    func goUp() {
        guard path.count > 1 else { return }
        let parent = path[path.count - 2]
        load(towards: parent.id, resettingTo: Array(path.dropLast(2)))
    }

    /// Move the node into the currently-shown location, then signal dismissal.
    /// Surfaces a move failure (e.g. an unexpected cycle) rather than masking it.
    func moveHere() {
        guard let destination = current else { return }
        let movingId = movingId
        let destinationId = destination.id
        Task {
            let error = await BridgeWrite.run("moving \(movingId) under \(destinationId)", handle: handle) {
                try $0.moveNode(id: movingId, newParentId: destinationId)
            }
            if let error {
                content = .failed(error)
            } else {
                moved.send(())
            }
        }
    }

    /// The local file path for `imageId` if its file exists, else nil; the cards
    /// call it on the render path.
    func imagePath(_ imageId: String) -> String? {
        ImagePath.resolve(handle, imageId)
    }

    /// Load the destination node and its children, landing a breadcrumb of
    /// `prefix` plus the loaded node. A nil `nodeId` loads the root house (the
    /// start of the walk); a non-nil one loads that node. Reads off the main
    /// actor, then mutates the state here. The moving node is dropped from the
    /// children so it can't be chosen or entered.
    private func load(towards nodeId: String?, resettingTo prefix: [BridgeNode]) {
        let handle = handle
        let movingId = movingId
        content = .loading
        Task {
            let outcome = await Task.detached { () -> LoadOutcome in
                do {
                    let node: BridgeNode
                    if let nodeId {
                        guard let loaded = try handle.getNode(id: nodeId) else {
                            return .failed("This place no longer exists.")
                        }
                        node = loaded
                    } else {
                        node = try handle.rootNode()
                    }
                    let children = try handle.children(parentId: node.id)
                        .filter { $0.id != movingId }
                    return .loaded(node: node, children: children)
                } catch {
                    logger.error("loading move destination \(nodeId ?? "root", privacy: .public) failed: \(error.localizedDescription, privacy: .public)")
                    return .failed(error.localizedDescription)
                }
            }.value
            switch outcome {
            case let .failed(message):
                content = .failed(message)
            case let .loaded(node, children):
                path = prefix + [node]
                content = .loaded(children: children)
            }
        }
    }

    /// The result of one off-main destination load: the node and its valid
    /// children, or a failure message already logged. Carried back to the main
    /// actor so the breadcrumb and content transition happens on the model.
    private enum LoadOutcome {
        case loaded(node: BridgeNode, children: [BridgeNode])
        case failed(String)
    }
}
