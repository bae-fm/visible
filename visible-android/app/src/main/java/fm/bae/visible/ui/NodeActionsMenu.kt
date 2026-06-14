package fm.bae.visible.ui

import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable

/**
 * The Edit details / Rename / Delete menu shown for a node, both from the current
 * node's overflow button and from a child card's long-press. Delete is omitted
 * when [canDelete] is false (the root house has no parent and can't be deleted).
 * Each action dismisses the menu before running.
 */
@Composable
fun NodeActionsMenu(
    expanded: Boolean,
    onDismiss: () -> Unit,
    onEdit: () -> Unit,
    onRename: () -> Unit,
    onDelete: () -> Unit,
    canDelete: Boolean,
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
        if (canDelete) {
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
