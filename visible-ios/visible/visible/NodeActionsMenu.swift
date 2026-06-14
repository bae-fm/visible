import SwiftUI

/// The Edit details / Change Photo / Remove Photo / Rename / Move / Delete
/// actions shown for a node, both from the current node's top-bar overflow `Menu`
/// and from a child card's `.contextMenu`. Move and Delete are omitted when
/// `isRoot` is true — the root house has no parent, so it can be neither
/// re-parented nor deleted.
///
/// The photo actions are present only where a photo header is on screen to act
/// on: the current node's overflow menu passes `onChangePhoto`/`onRemovePhoto`,
/// the child cards leave them nil (a child's photo is changed by opening it).
/// Remove Photo shows only when the node has an image (`hasImage`).
struct NodeActionsMenu: View {
    let onEdit: () -> Void
    var onChangePhoto: (() -> Void)?
    var onRemovePhoto: (() -> Void)?
    var hasImage: Bool = false
    let onRename: () -> Void
    let onMove: () -> Void
    let onDelete: () -> Void
    let isRoot: Bool

    var body: some View {
        Button("Edit details", action: onEdit)
        if let onChangePhoto {
            Button("Change Photo", action: onChangePhoto)
        }
        if hasImage, let onRemovePhoto {
            Button("Remove Photo", role: .destructive, action: onRemovePhoto)
        }
        Button("Rename", action: onRename)
        if !isRoot {
            Button("Move", action: onMove)
            Button("Delete", role: .destructive, action: onDelete)
        }
    }
}
