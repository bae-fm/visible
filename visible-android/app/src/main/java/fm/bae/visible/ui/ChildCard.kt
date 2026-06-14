package fm.bae.visible.ui

import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import uniffi.visible_bridge.BridgeNode

/**
 * One child node: its thumbnail and name, with a quantity badge over the
 * thumbnail when the node stands for more than one thing. Tapping opens it;
 * long-pressing opens an Edit details / Rename / Delete menu.
 */
@OptIn(ExperimentalFoundationApi::class)
@Composable
fun ChildCard(
    child: BridgeNode,
    path: String?,
    onOpen: () -> Unit,
    onEdit: () -> Unit,
    onRename: () -> Unit,
    onDelete: () -> Unit,
) {
    var menuOpen by remember { mutableStateOf(false) }

    Card {
        Box(
            modifier = Modifier.combinedClickable(
                onClick = onOpen,
                onLongClick = { menuOpen = true },
            ),
        ) {
            Column {
                Box {
                    NodeImage(
                        path = path,
                        cornerRadius = 0.dp,
                        modifier = Modifier.fillMaxWidth().aspectRatio(1f),
                    )
                    QuantityBadge(
                        quantity = child.quantity,
                        modifier = Modifier.align(Alignment.TopEnd).padding(6.dp),
                    )
                }
                NodeName(
                    name = child.name,
                    style = MaterialTheme.typography.bodyMedium,
                    maxLines = 2,
                    modifier = Modifier.fillMaxWidth().padding(8.dp),
                )
            }
            NodeActionsMenu(
                expanded = menuOpen,
                onDismiss = { menuOpen = false },
                onEdit = onEdit,
                onRename = onRename,
                onDelete = onDelete,
                canDelete = true,
            )
        }
    }
}

/**
 * A small "×N" badge for a node that stands for more than one thing. Shown only
 * when the quantity is set and greater than one — a single item (quantity null or
 * 1) carries no badge. The count is an integer, so rendering it as "×N" is the
 * composable's job, not a domain-formatting concern.
 */
@Composable
private fun QuantityBadge(
    quantity: Long?,
    modifier: Modifier = Modifier,
) {
    if (quantity != null && quantity > 1) {
        Text(
            text = "×$quantity",
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onPrimary,
            modifier = modifier
                .background(MaterialTheme.colorScheme.primary, RoundedCornerShape(50))
                .padding(horizontal = 6.dp, vertical = 2.dp),
        )
    }
}
