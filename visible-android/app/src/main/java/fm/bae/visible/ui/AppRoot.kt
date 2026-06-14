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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
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
import kotlinx.coroutines.launch
import uniffi.visible_bridge.AppHandle

private const val ARG_NODE_ID = "nodeId"
private const val ROUTE_BROWSE = "browse/{$ARG_NODE_ID}"
private const val ROUTE_SETTINGS = "settings"
private const val ROUTE_SHARING = "sharing"

private fun browseRoute(nodeId: String) = "browse/$nodeId"

/**
 * Opens the session, then hosts the browse navigation stack once it is open.
 * Collects the session's published [SessionState] so a library switch (join /
 * restore) re-renders the stack onto the new home.
 */
@Composable
fun AppRoot(session: AppSession) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val state by session.state.collectAsStateWithLifecycle()

    // Open on first composition; a failed open is not cached, so Retry re-runs it.
    LaunchedEffect(session) { session.open(context) }

    when (val s = state) {
        is SessionState.Loading -> CenteredMessage { CircularProgressIndicator() }
        is SessionState.Failed -> CenteredMessage {
            Column(
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                Text(s.message, color = MaterialTheme.colorScheme.error)
                Button(onClick = { scope.launch { session.open(context) } }) { Text("Retry") }
            }
        }
        is SessionState.Open -> BrowseNavigation(
            session = session,
            handle = s.handle,
            rootId = s.rootId,
        )
    }
}

/**
 * The browse navigation stack, one destination per node id. Each entry gets its
 * own [BrowseViewModel] scoped to the back-stack entry, so it loads its node and
 * is cleared when the entry pops. Tapping a child navigates to its id; system
 * back pops; the start destination (the root house) can't pop.
 */
@Composable
private fun BrowseNavigation(session: AppSession, handle: AppHandle, rootId: String) {
    // Re-key the controller + stack on the open library's root so a switch to a
    // joined home starts a fresh stack rooted at the new house, with no entries
    // carried over from the replaced home.
    key(rootId) {
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
                val nodeId = entry.arguments?.getString(ARG_NODE_ID)
                BrowseScreen(
                    viewModel = viewModel,
                    canPop = navController.previousBackStackEntry != null,
                    onPop = { navController.popBackStack() },
                    onOpenChild = { childId -> navController.navigate(browseRoute(childId)) },
                    // The sync gear lives on the root house only.
                    onOpenSettings = if (nodeId == rootId) {
                        { navController.navigate(ROUTE_SETTINGS) }
                    } else {
                        null
                    },
                )
            }
            composable(route = ROUTE_SETTINGS) {
                val viewModel: SettingsViewModel = viewModel(
                    factory = viewModelFactory {
                        initializer { SettingsViewModel(handle) }
                    },
                )
                SettingsScreen(
                    viewModel = viewModel,
                    onPop = { navController.popBackStack() },
                    onOpenSharing = { navController.navigate(ROUTE_SHARING) },
                )
            }
            composable(route = ROUTE_SHARING) {
                val appContext = LocalContext.current.applicationContext
                val viewModel: SharingViewModel = viewModel(
                    factory = viewModelFactory {
                        initializer { SharingViewModel(handle, session, appContext) }
                    },
                )
                SharingScreen(
                    viewModel = viewModel,
                    onPop = { navController.popBackStack() },
                )
            }
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
