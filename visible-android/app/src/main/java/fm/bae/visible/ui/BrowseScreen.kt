package fm.bae.visible.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.GridItemSpan
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import uniffi.visible_bridge.BridgeNode

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun BrowseScreen(
    viewModel: BrowseViewModel,
    canPop: Boolean,
    onPop: () -> Unit,
    onOpenChild: (String) -> Unit,
) {
    val content = viewModel.content
    val setImage = rememberCameraCapture(onCaptured = viewModel::setImage)

    // Reload whenever this screen becomes current: on first show and on return
    // from a child, so a node's photo or its children reflect changes made
    // while descended (the UX spec reloads each screen when it appears).
    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) { viewModel.reload() }

    // Deleting the current node pops back to its parent.
    LaunchedEffect(viewModel) {
        viewModel.deletedSelfEvents.collect { onPop() }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        text = (content as? BrowseContent.Loaded)?.node?.name ?: "",
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                },
                navigationIcon = {
                    if (canPop) {
                        IconButton(onClick = onPop) {
                            Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                        }
                    }
                },
                actions = {
                    (content as? BrowseContent.Loaded)?.let { loaded ->
                        NodeOverflowMenu(
                            node = loaded.node,
                            onRename = { viewModel.openRename(loaded.node) },
                            onDelete = { viewModel.openDelete(loaded.node) },
                        )
                    }
                },
            )
        },
        floatingActionButton = {
            if (content is BrowseContent.Loaded) {
                FloatingActionButton(onClick = viewModel::openAddChild) {
                    Icon(Icons.Filled.Add, contentDescription = "Add")
                }
            }
        },
    ) { padding ->
        Box(modifier = Modifier.fillMaxSize().padding(padding)) {
            when (content) {
                is BrowseContent.Loading -> CircularProgressIndicator(Modifier.align(Alignment.Center))
                is BrowseContent.Failed -> Text(
                    text = content.message,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.align(Alignment.Center).padding(24.dp),
                )
                is BrowseContent.Loaded -> LoadedContent(
                    viewModel = viewModel,
                    node = content.node,
                    children = content.children,
                    onTakePhoto = setImage::launch,
                    onOpenChild = onOpenChild,
                )
            }
        }
    }

    when (val dialog = viewModel.dialog) {
        null -> {}
        is BrowseDialog.AddChild -> NameDialog(
            title = "Add",
            confirmLabel = "Add",
            initial = "",
            onConfirm = viewModel::addChild,
            onDismiss = viewModel::dismissDialog,
        )
        is BrowseDialog.Rename -> NameDialog(
            title = "Rename",
            confirmLabel = "Rename",
            initial = dialog.target.name,
            onConfirm = { name -> viewModel.rename(dialog.target.id, name) },
            onDismiss = viewModel::dismissDialog,
        )
        is BrowseDialog.ConfirmDelete -> ConfirmDeleteDialog(
            name = dialog.target.name,
            onConfirm = { viewModel.delete(dialog.target.id) },
            onDismiss = viewModel::dismissDialog,
        )
    }
}

@Composable
private fun LoadedContent(
    viewModel: BrowseViewModel,
    node: BridgeNode,
    children: List<BridgeNode>,
    onTakePhoto: () -> Unit,
    onOpenChild: (String) -> Unit,
) {
    LazyVerticalGrid(
        columns = GridCells.Fixed(2),
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(16.dp),
        horizontalArrangement = Arrangement.spacedBy(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        item(span = { GridItemSpan(maxLineSpan) }) {
            NodeImage(
                path = node.imageId?.let(viewModel::imagePath),
                cornerRadius = 16.dp,
                modifier = Modifier
                    .fillMaxWidth()
                    .aspectRatio(1f)
                    .clickable(onClick = onTakePhoto),
            )
        }

        if (children.isEmpty()) {
            item(span = { GridItemSpan(maxLineSpan) }) {
                Text(
                    text = "Nothing here yet — add the first thing.",
                    textAlign = TextAlign.Center,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.fillMaxWidth().padding(top = 48.dp),
                )
            }
        } else {
            items(children, key = { it.id }) { child ->
                ChildCard(
                    child = child,
                    path = child.imageId?.let(viewModel::imagePath),
                    onOpen = { onOpenChild(child.id) },
                    onRename = { viewModel.openRename(child) },
                    onDelete = { viewModel.openDelete(child) },
                )
            }
        }
    }
}

@Composable
private fun NodeOverflowMenu(
    node: BridgeNode,
    onRename: () -> Unit,
    onDelete: () -> Unit,
) {
    var expanded by remember { mutableStateOf(false) }
    IconButton(onClick = { expanded = true }) {
        Icon(Icons.Filled.MoreVert, contentDescription = "More")
    }
    DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
        DropdownMenuItem(
            text = { Text("Rename") },
            onClick = {
                expanded = false
                onRename()
            },
        )
        // The root house has no parent and can't be deleted in v1.
        if (node.parentId != null) {
            DropdownMenuItem(
                text = { Text("Delete") },
                onClick = {
                    expanded = false
                    onDelete()
                },
            )
        }
    }
}
