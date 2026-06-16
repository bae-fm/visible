import SwiftUI

/// One child node: its thumbnail and name, with a quantity badge over the
/// thumbnail when the node stands for more than one thing. Tapping opens it; a
/// long-press context menu offers Edit details, Rename, Move, and Delete.
struct ChildCard: View {
    let child: BridgeNode
    let path: String?
    let onOpen: () -> Void
    let onEdit: () -> Void
    let onRename: () -> Void
    let onMove: () -> Void
    let onDelete: () -> Void

    var body: some View {
        Button(action: onOpen) {
            VStack(alignment: .leading, spacing: 0) {
                NodeImageView(path: path)
                    .aspectRatio(1, contentMode: .fit)
                    .frame(maxWidth: .infinity)
                    .overlay(alignment: .topTrailing) {
                        QuantityBadge(badge: child.quantityBadge)
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
        .overlay {
            RoundedRectangle(cornerRadius: 12)
                .strokeBorder(Color.primary.opacity(0.08), lineWidth: 1)
        }
        .shadow(color: .black.opacity(0.06), radius: 6, y: 3)
        .contextMenu {
            // A child always has a parent, so it is never the root: Move and
            // Delete both apply.
            NodeActionsMenu(onEdit: onEdit, onRename: onRename, onMove: onMove, onDelete: onDelete, isRoot: false)
        }
    }
}

/// The count badge for a node that stands for more than one thing, shown over the
/// thumbnail. `badge` is the core-precomputed "×N" string (see
/// `Node::quantity_badge`), `nil` for a single item, so the view renders it
/// directly rather than deciding the threshold or format itself.
private struct QuantityBadge: View {
    let badge: String?

    var body: some View {
        if let badge {
            Text(badge)
                .font(.caption2.weight(.semibold))
                .foregroundStyle(.white)
                .padding(.horizontal, 6)
                .padding(.vertical, 2)
                .background(Theme.accent, in: Capsule())
                .padding(6)
        }
    }
}
