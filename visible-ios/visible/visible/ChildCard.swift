import SwiftUI

/// One child node: its thumbnail and name. Tapping opens it; a long-press
/// context menu offers Rename and Delete.
struct ChildCard: View {
    let child: BridgeNode
    let path: String?
    let onOpen: () -> Void
    let onRename: () -> Void
    let onDelete: () -> Void

    var body: some View {
        Button(action: onOpen) {
            VStack(alignment: .leading, spacing: 0) {
                NodeImageView(path: path)
                    .aspectRatio(1, contentMode: .fit)
                    .frame(maxWidth: .infinity)
                Text(child.name)
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
        .contextMenu {
            // A child always has a parent, so it can always be deleted.
            NodeActionsMenu(onRename: onRename, onDelete: onDelete, canDelete: true)
        }
    }
}
