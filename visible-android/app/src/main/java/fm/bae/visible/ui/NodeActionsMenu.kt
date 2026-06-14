package fm.bae.visible.ui

import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable

/**
 * The Edit details / Change Photo / Remove Photo / Rename / Move / Delete menu
 * shown for a node, both from the current node's overflow button and from a child
 * card's long-press. Move and Delete are omitted when [isRoot] is true — the root
 * house has no parent, so it can be neither re-parented nor deleted. The photo
 * actions are present only where a photo header is on screen to act on: the
 * current node's overflow menu passes [onChangePhoto]/[onRemovePhoto], the child
 * cards leave them null (a child's photo is changed by opening it). Remove Photo
 * shows only when the node has an image ([hasImage]). Each action dismisses the
 * menu before running.
 */
@Composable
fun NodeActionsMenu(
    expanded: Boolean,
    onDismiss: () -> Unit,
    onEdit: () -> Unit,
    onRename: () -> Unit,
    onMove: () -> Unit,
    onDelete: () -> Unit,
    isRoot: Boolean,
    onChangePhoto: (() -> Unit)? = null,
    onRemovePhoto: (() -> Unit)? = null,
    hasImage: Boolean = false,
) {
    DropdownMenu(expanded = expanded, onDismissRequest = onDismiss) {
        DropdownMenuItem(
            text = { Text("Edit details") },
            onClick = {
                onDismiss()
                onEdit()
            },
        )
        if (onChangePhoto != null) {
            DropdownMenuItem(
                text = { Text("Change Photo") },
                onClick = {
                    onDismiss()
                    onChangePhoto()
                },
            )
        }
        if (hasImage && onRemovePhoto != null) {
            DropdownMenuItem(
                text = { Text("Remove Photo") },
                onClick = {
                    onDismiss()
                    onRemovePhoto()
                },
            )
        }
        DropdownMenuItem(
            text = { Text("Rename") },
            onClick = {
                onDismiss()
                onRename()
            },
        )
        if (!isRoot) {
            DropdownMenuItem(
                text = { Text("Move") },
                onClick = {
                    onDismiss()
                    onMove()
                },
            )
            DropdownMenuItem(
                text = { Text("Delete") },
                onClick = {
                    onDismiss()
                    onDelete()
                },
            )
        }
    }
}
