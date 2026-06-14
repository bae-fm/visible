package fm.bae.visible.ui

import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable

/**
 * The Edit details / Rename / Move / Delete menu shown for a node, both from the
 * current node's overflow button and from a child card's long-press. Move and
 * Delete are omitted when [isRoot] is true — the root house has no parent, so it
 * can be neither re-parented nor deleted. Each action dismisses the menu before
 * running.
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
) {
    DropdownMenu(expanded = expanded, onDismissRequest = onDismiss) {
        DropdownMenuItem(
            text = { Text("Edit details") },
            onClick = {
                onDismiss()
                onEdit()
            },
        )
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
