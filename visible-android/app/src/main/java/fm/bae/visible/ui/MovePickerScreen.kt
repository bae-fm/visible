package fm.bae.visible.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.GridItemSpan
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import uniffi.visible_bridge.BridgeNode

/**
 * A self-contained flow to pick a new parent for a node. It browses the tree
 * starting at the root house: tapping a destination card descends into it, the
 * top-bar back arrow walks one level up, and "Move here" re-parents the moving
 * node into the currently-shown location. It is its own navigation destination,
 * separate from the browse stack. On a successful move it pops back; the browse
 * screen it returns to reloads on resume and reflects the move. The composable
 * iterates over [MovePickerViewModel] state and renders it; the model owns the
 * walk, the move, and the concurrency.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MovePickerScreen(
    viewModel: MovePickerViewModel,
    onPop: () -> Unit,
) {
    // A successful move pops back to the browse screen, which reloads on resume.
    LaunchedEffect(viewModel) {
        viewModel.movedEvents.collect { onPop() }
    }

    val content = viewModel.content
    val canGoUp = viewModel.path.size > 1

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Move to…") },
                navigationIcon = {
                    // The arrow walks up one level inside the picker, or cancels
                    // the picker at the root (nowhere above to go).
                    IconButton(onClick = { if (canGoUp) viewModel.goUp() else onPop() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    if (viewModel.current != null) {
                        // A move into the node's current parent is a no-op core
                        // accepts and ignores, so "Move here" is always offered and
                        // the core no-op guard handles that case.
                        TextButton(onClick = { viewModel.moveHere() }) { Text("Move here") }
                    }
                },
            )
        },
    ) { padding ->
        Box(modifier = Modifier.fillMaxSize().padding(padding)) {
            when (content) {
                is MovePickerContent.Loading ->
                    CircularProgressIndicator(Modifier.align(Alignment.Center))
                is MovePickerContent.Failed -> Text(
                    text = content.message,
                    color = MaterialTheme.colorScheme.error,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.align(Alignment.Center).padding(24.dp),
                )
                is MovePickerContent.Loaded -> LoadedContent(
                    viewModel = viewModel,
                    path = viewModel.path,
                    children = content.children,
                )
            }
        }
    }
}

@Composable
private fun LoadedContent(
    viewModel: MovePickerViewModel,
    path: List<BridgeNode>,
    children: List<BridgeNode>,
) {
    Column(modifier = Modifier.fillMaxSize()) {
        Breadcrumb(path = path)

        LazyVerticalGrid(
            columns = GridCells.Fixed(2),
            modifier = Modifier.fillMaxSize(),
            contentPadding = PaddingValues(16.dp),
            horizontalArrangement = Arrangement.spacedBy(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            if (children.isEmpty()) {
                item(span = { GridItemSpan(maxLineSpan) }) {
                    Text(
                        text = "Nothing to open here — use “Move here” to move into this place.",
                        textAlign = TextAlign.Center,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.fillMaxWidth().padding(top = 48.dp),
                    )
                }
            } else {
                items(children, key = { it.id }) { child ->
                    MoveDestinationCard(
                        node = child,
                        path = child.imageId?.let(viewModel::imagePath),
                        onOpen = { viewModel.descend(child) },
                    )
                }
            }
        }
    }
}

/**
 * Where the picker currently is, shown as the path from the root house down to
 * the current location. Display only — the top-bar back arrow walks up; the
 * trail just shows the position.
 */
@Composable
private fun Breadcrumb(path: List<BridgeNode>) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .horizontalScroll(rememberScrollState())
            .padding(horizontal = 16.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        path.forEachIndexed { index, node ->
            if (index > 0) {
                Icon(
                    Icons.AutoMirrored.Filled.KeyboardArrowRight,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            val isCurrent = index == path.size - 1
            NodeName(
                name = node.name,
                style = MaterialTheme.typography.titleSmall.copy(
                    fontWeight = if (isCurrent) FontWeight.SemiBold else FontWeight.Normal,
                ),
                maxLines = 1,
            )
        }
    }
}

/**
 * One destination node in the picker: its thumbnail and name, tappable to descend
 * into it. Reuses the same image and name primitives the browse card does,
 * without the per-node action menu (the picker only descends or moves).
 */
@Composable
private fun MoveDestinationCard(
    node: BridgeNode,
    path: String?,
    onOpen: () -> Unit,
) {
    Card {
        Column(modifier = Modifier.clickable(onClick = onOpen)) {
            NodeImage(
                path = path,
                cornerRadius = 0.dp,
                modifier = Modifier.fillMaxWidth().aspectRatio(1f),
            )
            NodeName(
                name = node.name,
                style = MaterialTheme.typography.bodyMedium,
                maxLines = 2,
                modifier = Modifier.fillMaxWidth().padding(8.dp),
            )
        }
    }
}
