import SwiftUI

/// Shows one node: its photo header (which scrolls with the contents), a
/// 2-column grid of its children, an empty state, an add button, and a menu to
/// rename or delete the node. Reloads whenever it appears (first show and on
/// return from a child). Deleting the current node pops back to its parent.
struct BrowseView: View {
    let onOpenChild: (String) -> Void
    let onPop: () -> Void

    @State private var model: BrowseModel
    @State private var showCamera = false

    init(
        handle: AppHandle,
        nodeId: String,
        onOpenChild: @escaping (String) -> Void,
        onPop: @escaping () -> Void
    ) {
        self.onOpenChild = onOpenChild
        self.onPop = onPop
        _model = State(initialValue: BrowseModel(handle: handle, nodeId: nodeId))
    }

    private let columns = [
        GridItem(.flexible(), spacing: 16),
        GridItem(.flexible(), spacing: 16),
    ]

    var body: some View {
        content
            .navigationTitle(title)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                if case let .loaded(node, _) = model.content {
                    ToolbarItem(placement: .topBarTrailing) {
                        Menu {
                            Button("Rename") { model.dialog = .rename(target: node) }
                            // The root house has no parent and can't be deleted.
                            if node.parentId != nil {
                                Button("Delete", role: .destructive) {
                                    model.dialog = .confirmDelete(target: node)
                                }
                            }
                        } label: {
                            Image(systemName: "ellipsis.circle")
                        }
                    }
                    ToolbarItem(placement: .topBarTrailing) {
                        Button {
                            model.dialog = .addChild
                        } label: {
                            Image(systemName: "plus")
                        }
                    }
                }
            }
            .onAppear { Task { await model.reload() } }
            .onReceive(model.deletedSelf) { onPop() }
            .sheet(isPresented: $showCamera) {
                CameraView(
                    onCaptured: { bytes in
                        showCamera = false
                        Task { await model.setImage(bytes) }
                    },
                    onCancel: { showCamera = false }
                )
                .ignoresSafeArea()
            }
            .sheet(item: $model.dialog) { dialog in
                dialogContent(dialog)
            }
    }

    private var title: String {
        if case let .loaded(node, _) = model.content { node.name } else { "" }
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
        case let .loaded(node, children):
            loaded(node: node, children: children)
        }
    }

    private func loaded(node: BridgeNode, children: [BridgeNode]) -> some View {
        ScrollView {
            LazyVGrid(columns: columns, spacing: 16) {
                // The photo header is the first scrolling item, spanning both
                // columns — it scrolls with the grid rather than pinning.
                NodeImageView(path: node.imageId.flatMap(model.imagePath), cornerRadius: 16)
                    .aspectRatio(1, contentMode: .fit)
                    .frame(maxWidth: .infinity)
                    .gridCellColumns(2)
                    .onTapGesture { showCamera = true }

                if children.isEmpty {
                    Text("Nothing here yet — add the first thing.")
                        .foregroundStyle(.secondary)
                        .multilineTextAlignment(.center)
                        .frame(maxWidth: .infinity)
                        .padding(.top, 48)
                        .gridCellColumns(2)
                } else {
                    ForEach(children, id: \.id) { child in
                        ChildCard(
                            child: child,
                            path: child.imageId.flatMap(model.imagePath),
                            onOpen: { onOpenChild(child.id) },
                            onRename: { model.dialog = .rename(target: child) },
                            onDelete: { model.dialog = .confirmDelete(target: child) }
                        )
                    }
                }
            }
            .padding(16)
        }
    }

    @ViewBuilder
    private func dialogContent(_ dialog: BrowseDialog) -> some View {
        switch dialog {
        case .addChild:
            NameSheet(
                title: "Add",
                confirmLabel: "Add",
                initial: "",
                onConfirm: { name in Task { await model.addChild(name: name) } },
                onCancel: { model.dialog = nil }
            )
        case let .rename(target):
            NameSheet(
                title: "Rename",
                confirmLabel: "Rename",
                initial: target.name,
                onConfirm: { name in Task { await model.rename(id: target.id, name: name) } },
                onCancel: { model.dialog = nil }
            )
        case let .confirmDelete(target):
            DeleteConfirmSheet(
                name: target.name,
                onConfirm: { Task { await model.delete(id: target.id) } },
                onCancel: { model.dialog = nil }
            )
        }
    }
}
