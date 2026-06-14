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
    // Open the search screen. Search spans the whole tree, so every level offers
    // it (unlike the root-only settings gear).
    let onOpenSearch: () -> Void
    // Open the sync settings screen. Only the root house passes this, so the gear
    // shows there and nowhere deeper; nil leaves the gear off.
    let onOpenSettings: (() -> Void)?

    @State private var model: BrowseModel
    // Which capture site opened the camera, routing the captured photo to the
    // right model method: the header replaces this node's photo, the + adds a
    // new child carrying the photo. nil while the camera is closed.
    @State private var cameraIntent: CameraIntent?

    init(
        handle: AppHandle,
        nodeId: String,
        onOpenChild: @escaping (String) -> Void,
        onPop: @escaping () -> Void,
        onOpenSearch: @escaping () -> Void,
        onOpenSettings: (() -> Void)? = nil
    ) {
        self.onOpenChild = onOpenChild
        self.onPop = onPop
        self.onOpenSearch = onOpenSearch
        self.onOpenSettings = onOpenSettings
        _model = State(initialValue: BrowseModel(handle: handle, nodeId: nodeId))
    }

    private let columns = [
        GridItem(.flexible(), spacing: 16),
        GridItem(.flexible(), spacing: 16),
    ]

    var body: some View {
        content
            .inlineNavigationTitle(title)
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
                            openCamera(.newChild)
                        } label: {
                            Image(systemName: "plus")
                        }
                    }
                    ToolbarItem(placement: .primaryAction) {
                        Button(action: onOpenSearch) {
                            Image(systemName: "magnifyingglass")
                        }
                    }
                    // The sync gear lives on the root house only.
                    if let onOpenSettings {
                        ToolbarItem(placement: .primaryAction) {
                            Button(action: onOpenSettings) {
                                Image(systemName: "gearshape")
                            }
                        }
                    }
                }
            }
            .task { model.reload() }
            .onReceive(model.deletedSelf) { onPop() }
            .sheet(item: $cameraIntent) { intent in
                CameraView(
                    onCaptured: { bytes in
                        cameraIntent = nil
                        capture(bytes, for: intent)
                    },
                    onCancel: { cameraIntent = nil }
                )
                .ignoresSafeArea()
            }
            .sheet(item: $model.dialog) { dialog in
                dialogContent(dialog)
            }
    }

    private var title: String {
        // Empty while loading/failed (no node to title yet); the loaded node
        // shows its name, or "Untitled" if it has none. The navigation bar renders
        // the title in the system style, so the untitled placeholder reads as a
        // dimmed name on the card and the delete sheet, not here.
        guard case let .loaded(node, _) = model.content else { return "" }
        if let name = node.name { return name }
        return "Untitled"
    }

    /// Presents the camera for `intent`, or logs and does nothing when no camera
    /// is available (the simulator and camera-less devices have none).
    private func openCamera(_ intent: CameraIntent) {
        guard CameraView.isAvailable else {
            logger.warning("no camera available on this device; not presenting the camera")
            return
        }
        cameraIntent = intent
    }

    /// Routes the captured photo to the model method for the site that opened
    /// the camera: the header sets this node's photo, the + adds a new child.
    private func capture(_ bytes: Data, for intent: CameraIntent) {
        switch intent {
        case .nodePhoto: model.setImage(bytes)
        case .newChild: model.addChildWithPhoto(bytes)
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
                    .onTapGesture { openCamera(.nodePhoto) }

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
        case let .rename(target):
            NameSheet(
                // Seed the editable field with the current title, or blank if untitled.
                initial: target.name ?? "",
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

/// Which of the two capture sites opened the camera. The captured photo routes
/// to a different ``BrowseModel`` method per case: the photo header replaces
/// this node's photo, the + adds a new child carrying the photo.
private enum CameraIntent: Identifiable {
    case nodePhoto
    case newChild

    var id: Self { self }
}
