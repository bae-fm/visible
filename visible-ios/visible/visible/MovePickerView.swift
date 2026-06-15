import SwiftUI

/// A self-contained flow to pick a new parent for a node. It browses the tree
/// starting at the root house: tapping a destination card descends into it, a
/// Back action walks one level up, and "Move here" re-parents the moving node
/// into the currently-shown location. It is presented over the browse view (a
/// sheet) rather than pushed onto the browse stack, so the walk is separate from
/// where the user was browsing. On a successful move it dismisses; the browse
/// view it returns to reloads and reflects the move. The view iterates over
/// ``MovePickerModel`` state and renders it; the model owns the walk, the move,
/// and the concurrency.
struct MovePickerView: View {
    let onDismiss: () -> Void

    @State private var model: MovePickerModel

    private let columns = [
        GridItem(.flexible(), spacing: 16),
        GridItem(.flexible(), spacing: 16),
    ]

    init(handle: AppHandle, movingId: String, onDismiss: @escaping () -> Void) {
        self.onDismiss = onDismiss
        _model = State(initialValue: MovePickerModel(handle: handle, movingId: movingId))
    }

    var body: some View {
        NavigationStack {
            content
                .inlineNavigationTitle("Move to…")
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) {
                        Button("Cancel", action: onDismiss)
                    }
                    if model.current != nil {
                        ToolbarItem(placement: .confirmationAction) {
                            // A move into the node's current parent is a no-op core
                            // accepts and ignores, so "Move here" is always offered
                            // and the core no-op guard handles that case.
                            Button("Move here") { model.moveHere() }
                        }
                    }
                }
        }
        .task { model.start() }
        .onReceive(model.moved) { onDismiss() }
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
        case let .loaded(children):
            loaded(children: children)
        }
    }

    private func loaded(children: [BridgeNode]) -> some View {
        VStack(spacing: 0) {
            Breadcrumb(model: model)

            ScrollView {
                LazyVGrid(columns: columns, spacing: 16) {
                    if children.isEmpty {
                        Text("Nothing to open here — use “Move here” to move into this place.")
                            .foregroundStyle(.secondary)
                            .multilineTextAlignment(.center)
                            .frame(maxWidth: .infinity)
                            .padding(.top, 48)
                            .gridCellColumns(2)
                    } else {
                        ForEach(children, id: \.id) { child in
                            MoveDestinationCard(
                                node: child,
                                path: child.imageId.flatMap(model.imagePath),
                                onOpen: { model.descend(into: child) }
                            )
                        }
                    }
                }
                .padding(16)
            }
        }
    }
}

/// Where the picker currently is, shown as the path from the root house down to
/// the current location. A Back chevron leads the trail and walks one level up;
/// it is hidden at the root, where there is nowhere above to go.
private struct Breadcrumb: View {
    let model: MovePickerModel

    var body: some View {
        let path = model.path
        let canGoUp = path.count > 1
        HStack(spacing: 8) {
            if canGoUp {
                Button(action: { model.goUp() }) {
                    Image(systemName: "chevron.left")
                }
                .buttonStyle(.plain)
                .foregroundStyle(Theme.accent)
            }
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 4) {
                    ForEach(Array(path.enumerated()), id: \.element.id) { index, node in
                        if index > 0 {
                            Image(systemName: "chevron.right")
                                .font(.caption2)
                                .foregroundStyle(.secondary)
                        }
                        NodeName(name: node.name)
                            .font(index == path.count - 1 ? .subheadline.weight(.semibold) : .subheadline)
                            .foregroundStyle(index == path.count - 1 ? .primary : .secondary)
                            .lineLimit(1)
                    }
                }
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
    }
}

/// One destination node in the picker: its thumbnail and name, tappable to
/// descend into it. Reuses the same image and name primitives the browse card
/// does, without the per-node action menu (the picker only descends or moves).
private struct MoveDestinationCard: View {
    let node: BridgeNode
    let path: String?
    let onOpen: () -> Void

    var body: some View {
        Button(action: onOpen) {
            VStack(alignment: .leading, spacing: 0) {
                NodeImageView(path: path)
                    .aspectRatio(1, contentMode: .fit)
                    .frame(maxWidth: .infinity)
                NodeName(name: node.name)
                    .font(.body)
                    .lineLimit(2)
                    .multilineTextAlignment(.leading)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(8)
            }
        }
        .buttonStyle(.plain)
        .background(.regularMaterial)
        .clipShape(RoundedRectangle(cornerRadius: 12))
    }
}
