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
import androidx.compose.material.icons.filled.Checklist
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.style.TextAlign
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
    // Open the detail edit screen for a node id, pushed onto the browse stack.
    onOpenDetail: (String) -> Unit,
    // Open the destination picker to move a node id under a new parent.
    onOpenMove: (String) -> Unit,
    // Open the search screen. Search spans the whole tree, so every level offers
    // it (unlike the root-only settings gear).
    onOpenSearch: () -> Unit,
    // Open the sync settings screen. Only the root house passes this, so the gear
    // shows there and nowhere deeper; null leaves the gear off.
    onOpenSettings: (() -> Unit)? = null,
    // Open the shared task list. Like settings, only the root house passes this —
    // tasks are a home-level list, not per-node; null leaves the button off.
    onOpenTasks: (() -> Unit)? = null,
) {
    val content = viewModel.content
    // Photo capture sites. The + adds a new child carrying a captured photo. This
    // node's own photo comes from either the camera or the library, chosen via the
    // source sheet — each has its own launcher so the bytes route to setImage.
    val addChildWithPhoto = rememberCameraCapture(onCaptured = viewModel::addChildWithPhoto)
    val takeNodePhoto = rememberCameraCapture(onCaptured = viewModel::setImage)
    val pickNodePhoto = rememberLibraryPick(onPicked = viewModel::setImage)

    // The Take Photo / Choose from Library source sheet for setting this node's
    // photo, opened from a placeholder-header tap or the Change Photo menu item.
    var choosingPhotoSource by remember { mutableStateOf(false) }
    // The on-disk path of the node photo shown full-screen, or null while no
    // viewer is open. Set by tapping the header when a photo is present.
    var fullScreenPath: String? by remember { mutableStateOf(null) }

    // Reload whenever this screen becomes current: on first show and on return
    // from a child, so a node's photo or its children reflect changes made while
    // descended.
    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) { viewModel.reload() }

    // Deleting the current node pops back to its parent.
    LaunchedEffect(viewModel) {
        viewModel.deletedSelfEvents.collect { onPop() }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    // Empty while loading/failed (no node to title yet); the loaded
                    // node shows its name, or a dimmed "Untitled" if it has none.
                    val loaded = content as? BrowseContent.Loaded
                    if (loaded != null) {
                        NodeName(name = loaded.node.name, maxLines = 1)
                    } else {
                        Text(text = "", maxLines = 1)
                    }
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
                        IconButton(onClick = onOpenSearch) {
                            Icon(Icons.Filled.Search, contentDescription = "Search")
                        }
                        NodeOverflowMenu(
                            node = loaded.node,
                            onEdit = { onOpenDetail(loaded.node.id) },
                            onChangePhoto = { choosingPhotoSource = true },
                            onRemovePhoto = { viewModel.openRemovePhoto() },
                            onRename = { viewModel.openRename(loaded.node) },
                            onMove = { onOpenMove(loaded.node.id) },
                            onDelete = { viewModel.openDelete(loaded.node) },
                        )
                        // The shared task list lives on the root house only (tasks
                        // are a home-level list, not per-node).
                        if (onOpenTasks != null) {
                            IconButton(onClick = onOpenTasks) {
                                Icon(Icons.Filled.Checklist, contentDescription = "Tasks")
                            }
                        }
                        // The sync gear lives on the root house only.
                        if (onOpenSettings != null) {
                            IconButton(onClick = onOpenSettings) {
                                Icon(Icons.Filled.Settings, contentDescription = "Sync settings")
                            }
                        }
                    }
                },
            )
        },
        floatingActionButton = {
            if (content is BrowseContent.Loaded) {
                FloatingActionButton(onClick = addChildWithPhoto) {
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
                    onViewPhoto = { path -> fullScreenPath = path },
                    onChoosePhotoSource = { choosingPhotoSource = true },
                    onOpenChild = onOpenChild,
                    onOpenDetail = onOpenDetail,
                    onOpenMove = onOpenMove,
                )
            }
        }
    }

    when (val dialog = viewModel.dialog) {
        null -> {}
        is BrowseDialog.Rename -> NameDialog(
            // Seed the editable field with the current title, or blank if untitled.
            initial = dialog.target.name ?: "",
            onConfirm = { name -> viewModel.rename(dialog.target.id, name) },
            onDismiss = viewModel::dismissDialog,
        )
        is BrowseDialog.ConfirmDelete -> ConfirmDeleteDialog(
            name = dialog.target.name,
            onConfirm = { viewModel.delete(dialog.target.id) },
            onDismiss = viewModel::dismissDialog,
        )
        is BrowseDialog.ConfirmRemovePhoto -> ConfirmRemovePhotoDialog(
            onConfirm = { viewModel.removePhoto() },
            onDismiss = viewModel::dismissDialog,
        )
    }

    if (choosingPhotoSource) {
        PhotoSourceSheet(
            onTakePhoto = {
                choosingPhotoSource = false
                takeNodePhoto()
            },
            onChooseFromLibrary = {
                choosingPhotoSource = false
                pickNodePhoto()
            },
            onDismiss = { choosingPhotoSource = false },
        )
    }

    fullScreenPath?.let { path ->
        FullScreenImage(path = path, onDismiss = { fullScreenPath = null })
    }
}

/**
 * The Take Photo / Choose from Library chooser for setting a node's photo, shown
 * as a bottom sheet — the Android equivalent of the iOS source action sheet. Take
 * Photo is only offered where a camera exists; Choose from Library is always
 * available.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun PhotoSourceSheet(
    onTakePhoto: () -> Unit,
    onChooseFromLibrary: () -> Unit,
    onDismiss: () -> Unit,
) {
    val context = LocalContext.current
    ModalBottomSheet(onDismissRequest = onDismiss) {
        if (CameraCapture.isAvailable(context)) {
            ListItem(
                headlineContent = { Text("Take Photo") },
                modifier = Modifier.clickable(onClick = onTakePhoto),
            )
        }
        ListItem(
            headlineContent = { Text("Choose from Library") },
            modifier = Modifier.clickable(onClick = onChooseFromLibrary),
        )
    }
}

@Composable
private fun LoadedContent(
    viewModel: BrowseViewModel,
    node: BridgeNode,
    children: List<BridgeNode>,
    onViewPhoto: (String) -> Unit,
    onChoosePhotoSource: () -> Unit,
    onOpenChild: (String) -> Unit,
    onOpenDetail: (String) -> Unit,
    onOpenMove: (String) -> Unit,
) {
    LazyVerticalGrid(
        columns = GridCells.Fixed(2),
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(16.dp),
        horizontalArrangement = Arrangement.spacedBy(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        item(span = { GridItemSpan(maxLineSpan) }) {
            val headerPath = node.imageId?.let(viewModel::imagePath)
            NodeImage(
                path = headerPath,
                cornerRadius = 16.dp,
                modifier = Modifier
                    .fillMaxWidth()
                    .aspectRatio(1f)
                    .clickable {
                        // A set photo opens full-screen; a placeholder offers the
                        // Take/Choose source sheet.
                        if (headerPath != null) onViewPhoto(headerPath) else onChoosePhotoSource()
                    },
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
                    onEdit = { onOpenDetail(child.id) },
                    onRename = { viewModel.openRename(child) },
                    onMove = { onOpenMove(child.id) },
                    onDelete = { viewModel.openDelete(child) },
                )
            }
        }
    }
}

@Composable
private fun NodeOverflowMenu(
    node: BridgeNode,
    onEdit: () -> Unit,
    onChangePhoto: () -> Unit,
    onRemovePhoto: () -> Unit,
    onRename: () -> Unit,
    onMove: () -> Unit,
    onDelete: () -> Unit,
) {
    var expanded by remember { mutableStateOf(false) }
    IconButton(onClick = { expanded = true }) {
        Icon(Icons.Filled.MoreVert, contentDescription = "More")
    }
    NodeActionsMenu(
        expanded = expanded,
        onDismiss = { expanded = false },
        onEdit = onEdit,
        onRename = onRename,
        onMove = onMove,
        onDelete = onDelete,
        // The root house has no parent: it can be neither moved nor deleted.
        isRoot = node.parentId == null,
        onChangePhoto = onChangePhoto,
        onRemovePhoto = onRemovePhoto,
        hasImage = node.imageId != null,
    )
}
