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
    // Open the detail edit screen for a node id, pushed onto the browse stack.
    let onOpenDetail: (String) -> Void
    // Open the search screen. Search spans the whole tree, so every level offers
    // it (unlike the root-only settings gear).
    let onOpenSearch: () -> Void
    // Open the sync settings screen. Only the root house passes this, so the gear
    // shows there and nowhere deeper; nil leaves the gear off.
    let onOpenSettings: (() -> Void)?

    private let handle: AppHandle

    @State private var model: BrowseModel
    // Which capture site opened the camera, routing the captured photo to the
    // right model method: the header replaces this node's photo, the + adds a
    // new child carrying the photo. nil while the camera is closed.
    @State private var cameraIntent: CameraIntent?
    // Presents the Take Photo / Choose from Library action sheet for setting this
    // node's photo (from a placeholder-header tap or the Change Photo menu item).
    @State private var choosingPhotoSource = false
    // Presents the photo-library picker over this view; both Choose from Library
    // and the resulting import feed this node's photo through `setNodeImage`.
    @State private var importingPhoto = false
    // The on-disk path of the node photo shown full-screen, or nil while no
    // viewer is open. Set by tapping the header when a photo is present.
    @State private var viewingPhotoPath: String?
    // The node whose move-destination picker is open, presented over this view;
    // nil while no picker is shown. Holds the node id (an Identifiable wrapper so
    // it drives `.sheet(item:)`) because the picker moves a specific node.
    @State private var movingNode: MovingNode?

    init(
        handle: AppHandle,
        nodeId: String,
        onOpenChild: @escaping (String) -> Void,
        onPop: @escaping () -> Void,
        onOpenDetail: @escaping (String) -> Void,
        onOpenSearch: @escaping () -> Void,
        onOpenSettings: (() -> Void)? = nil
    ) {
        self.handle = handle
        self.onOpenChild = onOpenChild
        self.onPop = onPop
        self.onOpenDetail = onOpenDetail
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
                                onEdit: { onOpenDetail(node.id) },
                                onChangePhoto: { choosingPhotoSource = true },
                                onRemovePhoto: { model.openRemovePhoto() },
                                hasImage: node.imageId != nil,
                                onRename: { model.openRename(node) },
                                onMove: { movingNode = MovingNode(id: node.id) },
                                onDelete: { model.openDelete(node) },
                                // The root house has no parent: it can be neither
                                // moved nor deleted.
                                isRoot: node.parentId == nil
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
            .sheet(item: $movingNode) { moving in
                MovePickerView(
                    handle: handle,
                    movingId: moving.id,
                    onDismiss: {
                        movingNode = nil
                        // The moved node may have left this level, so reload to
                        // reflect the new tree.
                        model.reload()
                    }
                )
            }
            .confirmationDialog(
                "Photo",
                isPresented: $choosingPhotoSource,
                titleVisibility: .hidden
            ) {
                // Take Photo is only offered where a camera exists; Choose from
                // Library is always available. Both feed this node's photo.
                if CameraView.isAvailable {
                    Button("Take Photo") { cameraIntent = .nodePhoto }
                }
                Button("Choose from Library") { importingPhoto = true }
            }
            .photoLibraryImport(
                isPresented: $importingPhoto,
                onPicked: { bytes in
                    importingPhoto = false
                    model.setImage(bytes)
                },
                onCancel: { importingPhoto = false }
            )
            .fullScreenImageCover(path: $viewingPhotoPath)
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
                let headerPath = node.imageId.flatMap(model.imagePath)
                NodeImageView(path: headerPath, cornerRadius: 16)
                    .aspectRatio(1, contentMode: .fit)
                    .frame(maxWidth: .infinity)
                    .gridCellColumns(2)
                    .onTapGesture {
                        // A set photo opens full-screen; a placeholder offers the
                        // Take/Choose source sheet.
                        if let headerPath {
                            viewingPhotoPath = headerPath
                        } else {
                            choosingPhotoSource = true
                        }
                    }

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
                            onEdit: { onOpenDetail(child.id) },
                            onRename: { model.openRename(child) },
                            onMove: { movingNode = MovingNode(id: child.id) },
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
        case .confirmRemovePhoto:
            RemovePhotoConfirmSheet(
                onConfirm: { model.removePhoto() },
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

/// The node whose move-destination picker is open, wrapping its id so it drives
/// `.sheet(item:)` (which needs an `Identifiable`). The id is both the node id
/// and the sheet's identity.
private struct MovingNode: Identifiable {
    let id: String
}
