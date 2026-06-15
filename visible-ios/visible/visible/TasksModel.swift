import Foundation
import os.log

private let logger = Logger.visible("TasksModel")

/// Loads and edits the home's shared task list. The list is synced across the
/// home's members, so what loads here reflects the latest merged sync and every
/// add, check-off, rename, or delete shows up on co-householders' devices. Bridge
/// calls touch SQLite so they run off the main actor; the read-modify-write of
/// the screen state happens here on the model, not in the view
/// (observable-mutate-on-the-state-not-the-view). The view iterates over the
/// model and renders it.
@MainActor
@Observable
final class TasksModel {
    private let handle: AppHandle

    private(set) var content: Loadable<[BridgeTask]> = .loading
    /// The new-task field, seeded blank (form-seeding). Trimmed on add; the Add
    /// control is disabled while it's blank.
    var newTitle = ""
    /// A write is in flight. Local UI state for the in-flight gesture.
    private(set) var working = false
    /// The last write failure, cleared on the next attempt.
    private(set) var errorMessage: String?

    init(handle: AppHandle) {
        self.handle = handle
    }

    /// Whether the Add control is enabled: a non-blank title and no write running.
    var canAdd: Bool {
        !newTitle.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty && !working
    }

    /// Load the shared task list.
    func reload() {
        let handle = handle
        Task {
            content = await Task.detached { () -> Loadable<[BridgeTask]> in
                do {
                    return .loaded(try handle.tasks())
                } catch {
                    logger.error("loading tasks failed: \(error.localizedDescription, privacy: .public)")
                    return .failed(error.localizedDescription)
                }
            }.value
        }
    }

    /// Add the typed task and clear the field. Core trims and rejects a blank
    /// title, and the Add control is disabled while blank, so this is the normal
    /// path. Reloads after.
    func add() {
        let title = newTitle
        newTitle = ""
        runWrite("adding a task") { _ = try $0.createTask(title: title) }
    }

    /// Check a task off, or back on.
    func setDone(_ task: BridgeTask, _ done: Bool) {
        runWrite("updating a task") { try $0.setTaskDone(id: task.id, done: done) }
    }

    /// Rename a task.
    func rename(id: String, title: String) {
        runWrite("renaming a task") { try $0.renameTask(id: id, title: title) }
    }

    /// Remove a task from the shared list.
    func delete(id: String) {
        runWrite("deleting a task") { try $0.deleteTask(id: id) }
    }

    /// Run a bridge write off the main actor, then reload the list so it reflects
    /// the stored (and freshly synced) state. The failure, or nil on success,
    /// lands in ``errorMessage``.
    private func runWrite(_ description: String, _ write: @escaping @Sendable (AppHandle) throws -> Void) {
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
