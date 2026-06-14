import SwiftUI

/// One child node: its thumbnail and name, with a quantity badge over the
/// thumbnail when the node stands for more than one thing. Tapping opens it; a
/// long-press context menu offers Edit details, Rename, and Delete.
struct ChildCard: View {
    let child: BridgeNode
    let path: String?
    let onOpen: () -> Void
    let onEdit: () -> Void
    let onRename: () -> Void
    let onDelete: () -> Void

    var body: some View {
        Button(action: onOpen) {
            VStack(alignment: .leading, spacing: 0) {
                NodeImageView(path: path)
                    .aspectRatio(1, contentMode: .fit)
                    .frame(maxWidth: .infinity)
                    .overlay(alignment: .topTrailing) {
                        QuantityBadge(quantity: child.quantity)
                    }
                NodeName(name: child.name)
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
            NodeActionsMenu(onEdit: onEdit, onRename: onRename, onDelete: onDelete, canDelete: true)
        }
    }
}

/// A small "×N" badge for a node that stands for more than one thing. Shown only
/// when the quantity is set and greater than one — a single item (quantity nil or
/// 1) carries no badge. The count is an integer, so rendering it as "×N" is the
/// view's job, not a domain-formatting concern.
private struct QuantityBadge: View {
    let quantity: Int64?

    var body: some View {
        if let quantity, quantity > 1 {
            Text("×\(quantity)")
                .font(.caption2.weight(.semibold))
                .foregroundStyle(.white)
                .padding(.horizontal, 6)
                .padding(.vertical, 2)
                .background(Theme.accent, in: Capsule())
                .padding(6)
        }
    }
}
