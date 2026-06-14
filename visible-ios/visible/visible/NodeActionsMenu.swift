import SwiftUI

/// The Edit details / Rename / Delete actions shown for a node, both from the
/// current node's top-bar overflow `Menu` and from a child card's `.contextMenu`.
/// Delete is omitted when `canDelete` is false (the root house has no parent and
/// can't be deleted).
struct NodeActionsMenu: View {
    let onEdit: () -> Void
    let onRename: () -> Void
    let onDelete: () -> Void
    let canDelete: Bool

    var body: some View {
        Button("Edit details", action: onEdit)
        Button("Rename", action: onRename)
        if canDelete {
            Button("Delete", role: .destructive, action: onDelete)
        }
    }
}
