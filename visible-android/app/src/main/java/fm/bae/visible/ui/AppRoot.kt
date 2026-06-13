package fm.bae.visible.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import fm.bae.visible.AppSession
import fm.bae.visible.SessionState
import uniffi.visible_bridge.AppHandle

private const val ARG_NODE_ID = "nodeId"
private const val ROUTE_BROWSE = "browse/{$ARG_NODE_ID}"

private fun browseRoute(nodeId: String) = "browse/$nodeId"

/** Opens the session, then hosts the browse navigation stack once it is open. */
@Composable
fun AppRoot(session: AppSession) {
    val context = LocalContext.current
    // Bumping this re-keys produceState, which re-runs session.open. A failed
    // open is not cached, so the retry re-attempts it.
    var attempt by remember { mutableIntStateOf(0) }
    val state by produceState<SessionState>(SessionState.Loading, session, attempt) {
        value = SessionState.Loading
        value = session.open(context)
    }

    when (val s = state) {
        is SessionState.Loading -> CenteredMessage { CircularProgressIndicator() }
        is SessionState.Failed -> CenteredMessage {
            Column(
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                Text(s.message, color = MaterialTheme.colorScheme.error)
                Button(onClick = { attempt++ }) { Text("Retry") }
            }
        }
        is SessionState.Open -> BrowseNavigation(handle = s.handle, rootId = s.rootId)
    }
}

/**
 * The browse navigation stack, one destination per node id. Each entry gets its
 * own [BrowseViewModel] scoped to the back-stack entry, so it loads its node and
 * is cleared when the entry pops. Tapping a child navigates to its id; system
 * back pops; the start destination (the root house) can't pop.
 */
@Composable
private fun BrowseNavigation(handle: AppHandle, rootId: String) {
    val navController = rememberNavController()
    NavHost(navController = navController, startDestination = browseRoute(rootId)) {
        composable(
            route = ROUTE_BROWSE,
            arguments = listOf(navArgument(ARG_NODE_ID) { type = NavType.StringType }),
        ) { entry ->
            val viewModel: BrowseViewModel = viewModel(
                factory = viewModelFactory {
                    initializer {
                        val nodeId = entry.arguments?.getString(ARG_NODE_ID)
                            ?: error("browse destination is missing its $ARG_NODE_ID argument")
                        BrowseViewModel(handle, nodeId)
                    }
                },
            )
            BrowseScreen(
                viewModel = viewModel,
                canPop = navController.previousBackStackEntry != null,
                onPop = { navController.popBackStack() },
                onOpenChild = { childId -> navController.navigate(browseRoute(childId)) },
            )
        }
    }
}

@Composable
private fun CenteredMessage(content: @Composable () -> Unit) {
    Box(
        modifier = Modifier.fillMaxSize().padding(24.dp),
        contentAlignment = Alignment.Center,
    ) {
        content()
    }
}
