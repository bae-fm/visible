package fm.bae.visible.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.foundation.text.KeyboardOptions
import uniffi.visible_bridge.BridgeNode
import uniffi.visible_bridge.BridgeSearchResult

/**
 * Search the whole tree by node name. An auto-focused text field drives
 * [SearchViewModel]; the screen renders its tri-state (idle / loading / results /
 * no matches / failed). Each result row shows the node's thumbnail, its name, and
 * the core-built ancestor breadcrumb; tapping a row navigates the browse stack to
 * that node. The composable iterates over the state and renders it; the model
 * owns the search and the concurrency.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SearchScreen(
    viewModel: SearchViewModel,
    onPop: () -> Unit,
    // Hand the tapped result's breadcrumb (root→node ancestor nodes) up to the
    // browse navigation, which resets the stack so the landed node's back button
    // walks up the real ancestor chain.
    onNavigate: (List<BridgeNode>) -> Unit,
) {
    val focusRequester = remember { FocusRequester() }
    LaunchedEffect(Unit) { focusRequester.requestFocus() }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Search") },
                navigationIcon = {
                    IconButton(onClick = onPop) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        Column(modifier = Modifier.fillMaxSize().padding(padding)) {
            OutlinedTextField(
                value = viewModel.query,
                onValueChange = viewModel::onQueryChange,
                placeholder = { Text("Search") },
                singleLine = true,
                keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.None),
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(16.dp)
                    .focusRequester(focusRequester),
            )

            Results(
                state = viewModel.state,
                imagePath = viewModel::imagePath,
                onNavigate = onNavigate,
            )
        }
    }
}

@Composable
private fun Results(
    state: SearchState,
    imagePath: (String) -> String?,
    onNavigate: (List<BridgeNode>) -> Unit,
) {
    when (state) {
        is SearchState.Idle -> Hint("Search for anything in your home by name.")
        is SearchState.Loading -> Box(
            modifier = Modifier.fillMaxSize(),
            contentAlignment = Alignment.Center,
        ) { CircularProgressIndicator() }
        is SearchState.NoMatches -> Hint("No matches.")
        is SearchState.Failed -> Box(
            modifier = Modifier.fillMaxSize().padding(24.dp),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = state.message,
                color = MaterialTheme.colorScheme.error,
                textAlign = TextAlign.Center,
            )
        }
        is SearchState.Results -> LazyColumn(
            modifier = Modifier.fillMaxSize(),
            contentPadding = PaddingValues(horizontal = 16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            items(state.hits, key = { it.node.id }) { hit ->
                SearchResultRow(
                    hit = hit,
                    path = hit.node.imageId?.let(imagePath),
                    onClick = { onNavigate(hit.path) },
                )
            }
        }
    }
}

@Composable
private fun Hint(text: String) {
    Box(
        modifier = Modifier.fillMaxSize().padding(24.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = text,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
        )
    }
}

/**
 * One search match: its thumbnail, name, and the ancestor breadcrumb the core
 * built (rendered as-is; the composable does not join names). An untitled match
 * shows the shared "Untitled" placeholder; the breadcrumb is hidden only when
 * empty.
 */
@Composable
private fun SearchResultRow(
    hit: BridgeSearchResult,
    path: String?,
    onClick: () -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick)
            .padding(vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        NodeImage(
            path = path,
            cornerRadius = 8.dp,
            modifier = Modifier.size(48.dp),
        )
        Column(modifier = Modifier.fillMaxWidth()) {
            NodeName(
                name = hit.node.name,
                style = MaterialTheme.typography.bodyLarge,
                maxLines = 1,
            )
            if (hit.pathLabel.isNotEmpty()) {
                Text(
                    text = hit.pathLabel,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}
