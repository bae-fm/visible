import SwiftUI

/// The Edit details / Rename / Move / Delete actions shown for a node, both from
/// the current node's top-bar overflow `Menu` and from a child card's
/// `.contextMenu`. Move and Delete are omitted when `isRoot` is true — the root
/// house has no parent, so it can be neither re-parented nor deleted.
struct NodeActionsMenu: View {
    let onEdit: () -> Void
    let onRename: () -> Void
    let onMove: () -> Void
    let onDelete: () -> Void
    let isRoot: Bool

    var body: some View {
        Button("Edit details", action: onEdit)
        Button("Rename", action: onRename)
        if !isRoot {
            Button("Move", action: onMove)
            Button("Delete", role: .destructive, action: onDelete)
        }
    }
}
