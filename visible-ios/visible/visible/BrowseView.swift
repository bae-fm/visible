import SwiftUI
import os.log

private let logger = Logger.visible("BrowseView")

/// Shows one node: its photo header (which scrolls with the contents), a
/// 2-column grid of its children, an empty state, an add button, and a menu to
/// rename or delete the node. Reloads whenever it appears (first show and on
/// return from a child). Deleting the current node pops back to its parent. The
/// view only calls ``BrowseModel`` methods and renders; the model owns the
/// state mutation and the concurrency.
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
            #if os(iOS)
            .navigationBarTitleDisplayMode(.inline)
            #endif
            .toolbar {
                if case let .loaded(node, _) = model.content {
                    ToolbarItem(placement: .primaryAction) {
                        Menu {
                            NodeActionsMenu(
                                onRename: { model.openRename(node) },
                                onDelete: { model.openDelete(node) },
                                // The root house has no parent and can't be deleted.
                                canDelete: node.parentId != nil
                            )
                        } label: {
                            Image(systemName: "ellipsis.circle")
                        }
                    }
                    ToolbarItem(placement: .primaryAction) {
                        Button {
                            model.openAddChild()
                        } label: {
                            Image(systemName: "plus")
                        }
                    }
                }
            }
            .task { model.reload() }
            .onReceive(model.deletedSelf) { onPop() }
            .sheet(isPresented: $showCamera) {
                CameraView(
                    onCaptured: { bytes in
                        showCamera = false
                        model.setImage(bytes)
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

    /// Presents the camera, or logs and does nothing when no camera is
    /// available (the simulator and camera-less devices have none).
    private func openCamera() {
        guard CameraView.isAvailable else {
            logger.warning("no camera available on this device; not presenting the camera")
            return
        }
        showCamera = true
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
                    .onTapGesture { openCamera() }

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
                            onRename: { model.openRename(child) },
                            onDelete: { model.openDelete(child) }
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
                initial: "",
                onConfirm: { name in model.addChild(name: name) },
                onCancel: { model.dismissDialog() }
            )
        case let .rename(target):
            NameSheet(
                title: "Rename",
                initial: target.name,
                onConfirm: { name in model.rename(id: target.id, name: name) },
                onCancel: { model.dismissDialog() }
            )
        case let .confirmDelete(target):
            DeleteConfirmSheet(
                name: target.name,
                onConfirm: { model.delete(id: target.id) },
                onCancel: { model.dismissDialog() }
            )
        }
    }
}
