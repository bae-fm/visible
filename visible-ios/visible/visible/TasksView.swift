import SwiftUI

/// The home's shared task list, reached from the browse root: add a task, check
/// tasks off, and rename or delete them from a row's context menu. The list is
/// synced across the home's members, so a co-householder's changes appear here on
/// the next sync. The view iterates over ``TasksModel`` and renders; the model
/// owns the state mutation and the concurrency. Shared by iOS and macOS.
struct TasksView: View {
    @State private var model: TasksModel
    // The task whose rename sheet is open, wrapping its id and current title so it
    // drives `.sheet(item:)`; nil while no rename is up.
    @State private var renaming: RenamingTask?

    init(handle: AppHandle) {
        _model = State(initialValue: TasksModel(handle: handle))
    }

    var body: some View {
        content
            .inlineNavigationTitle("Tasks")
            .task { model.reload() }
            .sheet(item: $renaming) { target in
                NameSheet(
                    initial: target.title,
                    onConfirm: { title in
                        model.rename(id: target.id, title: title)
                        renaming = nil
                    },
                    onCancel: { renaming = nil }
                )
            }
    }

    @ViewBuilder
    private var content: some View {
        switch model.content {
        case .loading:
            ProgressView()
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        case let .failed(message):
            Text(message)
                .foregroundStyle(.red)
                .multilineTextAlignment(.center)
                .padding(24)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        case let .loaded(tasks):
            loaded(tasks: tasks)
        }
    }

    private func loaded(tasks: [BridgeTask]) -> some View {
        Form {
            Section {
                HStack {
                    TextField("Add a task", text: $model.newTitle)
                        .submitLabel(.done)
                        .onSubmit { if model.canAdd { model.add() } }
                    Button("Add") { model.add() }
                        .disabled(!model.canAdd)
                }
            }

            if let error = model.errorMessage {
                Section {
                    Text(error).foregroundStyle(.red)
                }
            }

            if tasks.isEmpty {
                Section {
                    Text("No tasks yet — add the first thing to do.")
                        .foregroundStyle(.secondary)
                }
            } else {
                let remaining = tasks.filter { !$0.done }.count
                Section {
                    ForEach(tasks, id: \.id) { task in
                        TaskRow(
                            task: task,
                            onToggle: { model.setDone(task, !task.done) },
                            onRename: { renaming = RenamingTask(id: task.id, title: task.title) },
                            onDelete: { model.delete(id: task.id) }
                        )
                    }
                } header: {
                    Text("\(remaining) to do · \(tasks.count) total")
                } footer: {
                    Text("Shared across everyone in the home — changes sync on the next refresh.")
                }
            }
        }
    }
}

/// One task row: a tap-to-toggle checkmark and the title (struck through when
/// done), with rename and delete in a context menu (cross-platform — no swipe
/// actions, which macOS lacks).
private struct TaskRow: View {
    let task: BridgeTask
    let onToggle: () -> Void
    let onRename: () -> Void
    let onDelete: () -> Void

    var body: some View {
        HStack(spacing: 12) {
            Button(action: onToggle) {
                Image(systemName: task.done ? "checkmark.circle.fill" : "circle")
                    .foregroundStyle(task.done ? Theme.accent : .secondary)
            }
            .buttonStyle(.plain)

            Text(task.title)
                .strikethrough(task.done)
                .foregroundStyle(task.done ? .secondary : .primary)

            Spacer()
        }
        .contentShape(Rectangle())
        .contextMenu {
            Button("Rename", action: onRename)
            Button("Delete", role: .destructive, action: onDelete)
        }
    }
}

/// The task whose rename sheet is open, wrapping its id and current title so it
/// drives `.sheet(item:)` (which needs an `Identifiable`).
private struct RenamingTask: Identifiable {
    let id: String
    let title: String
}
