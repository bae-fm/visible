package fm.bae.visible.ui

import androidx.navigation.NavController
import androidx.navigation.NavType
import androidx.navigation.compose.ComposeNavigator
import androidx.navigation.compose.composable
import androidx.navigation.createGraph
import androidx.navigation.navArgument
import androidx.navigation.testing.TestNavHostController
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import uniffi.visible_bridge.BridgeNode

/**
 * Proves the search-result jump synthesizes a browse back stack that walks up the
 * matched node's real ancestors. Drives [navigateToBreadcrumb] — the production
 * replay [AppRoot] uses — against a real [TestNavHostController] whose graph has
 * the same browse/search route shapes, so it exercises the real [NavController]
 * behavior, not a reconstruction of it.
 *
 * The graph is built programmatically (not via a `NavHost` composable) and every
 * navigation call runs on the main thread via `runOnMainSync`, so the test needs
 * neither a Compose host nor Espresso idle-sync — both of which the Compose test
 * rule routes through and which the API-36 emulator's Espresso build can't load
 * (`InputManager.getInstance` reflection failure).
 */
@RunWith(AndroidJUnit4::class)
class SearchNavigationTest {
    private lateinit var nav: TestNavHostController

    @Before
    fun setUp() {
        InstrumentationRegistry.getInstrumentation().runOnMainSync {
            nav = TestNavHostController(ApplicationProvider.getApplicationContext())
            nav.navigatorProvider.addNavigator(ComposeNavigator())
            nav.graph = nav.createGraph(startDestination = "browse/root") {
                composable(
                    route = "browse/{nodeId}",
                    arguments = listOf(navArgument("nodeId") { type = NavType.StringType }),
                ) {}
                composable("search") {}
            }
        }
    }

    private fun onMain(block: () -> Unit) =
        InstrumentationRegistry.getInstrumentation().runOnMainSync(block)

    private fun node(id: String) = BridgeNode(id = id, parentId = null, name = id, imageId = null)

    /** The current destination's browse node id (the `{nodeId}` argument), or null on `search`. */
    private fun NavController.currentNodeId(): String? =
        currentBackStackEntry?.arguments?.getString("nodeId")

    /** The node ids on the back stack, root first (search entries contribute none). */
    private fun NavController.stackNodeIds(): List<String> =
        currentBackStack.value.mapNotNull { it.arguments?.getString("nodeId") }

    @Test
    fun tappingADepth3ResultLandsTheNodeAndBackWalksUpToRoot() {
        // Open search from the root house, the way the search icon does.
        onMain {
            nav.navigate("search")
            assertNull("search has no node id", nav.currentNodeId())
        }

        // Tap a depth-3 result: Home -> room -> shelf -> vase. The breadcrumb is
        // root→node inclusive, exactly what BridgeSearchResult.path carries.
        val breadcrumb = listOf(node("root"), node("room"), node("shelf"), node("vase"))
        onMain {
            navigateToBreadcrumb(nav, "root", breadcrumb)

            // It lands on the matched node, search popped, with the full ancestor
            // chain on the stack: root, room, shelf, vase.
            assertEquals("vase", nav.currentNodeId())
            assertEquals(listOf("root", "room", "shelf", "vase"), nav.stackNodeIds())
        }

        // Press back three times: vase -> shelf -> room -> root, through the real
        // ancestors, ending at the root house.
        onMain {
            nav.popBackStack()
            assertEquals("shelf", nav.currentNodeId())
            nav.popBackStack()
            assertEquals("room", nav.currentNodeId())
            nav.popBackStack()
            assertEquals("root", nav.currentNodeId())
        }
    }

    @Test
    fun jumpingFromADeepBrowseLocationClearsTheStaleStack() {
        // Descend into an unrelated branch, then open search from there.
        onMain {
            nav.navigate("browse/garage")
            nav.navigate("browse/toolbox")
            nav.navigate("search")
        }

        // Jump to a node in a different branch. The replay pops the stale garage/
        // toolbox entries back to root before replaying the new chain.
        val breadcrumb = listOf(node("root"), node("kitchen"), node("drawer"))
        onMain {
            navigateToBreadcrumb(nav, "root", breadcrumb)
            assertEquals(listOf("root", "kitchen", "drawer"), nav.stackNodeIds())
        }
    }
}
