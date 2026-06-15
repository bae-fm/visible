package fm.bae.visible.ui

import android.util.Log
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.visible_bridge.AppHandle
import uniffi.visible_bridge.BridgeNode

private const val TAG = "visible.MovePickerViewModel"

/** What the destination picker is showing at its current location. */
sealed interface MovePickerContent {
    data object Loading : MovePickerContent

    data class Failed(val message: String) : MovePickerContent

    /**
     * The children of the current location that are valid destinations (the
     * moving node is omitted — see [MovePickerViewModel]). The current location
     * itself is the last element of [MovePickerViewModel.path], which drives the
     * breadcrumb and "Move here".
     */
    data class Loaded(val children: List<BridgeNode>) : MovePickerContent
}

/**
 * Drives the destination picker: a self-contained walk of the tree to choose a
 * new parent for [movingId]. It starts at the root house and descends into a
 * tapped node; [path] is the nodes from the root to the current location, so the
 * back action pops one level. "Move here" re-parents [movingId] into the
 * currently-shown node.
 *
 * The moving node is omitted from every children list: you can't move a node into
 * itself, and since it can't be entered its descendants are unreachable, so
 * omitting the one node keeps the whole moving subtree out of the picker. Core
 * still rejects an out-of-band cycle (the omission is the affordance; core is the
 * guard).
 *
 * Bridge calls touch SQLite so they run on [Dispatchers.IO]; the state mutation
 * happens here on the state, not in the composable
 * (observable-mutate-on-the-state-not-the-view). The composable iterates over
 * [content] and renders it.
 */
class MovePickerViewModel(
    private val handle: AppHandle,
    private val movingId: String,
) : ViewModel() {
    var content: MovePickerContent by mutableStateOf(MovePickerContent.Loading)
        private set

    // The nodes from the root down to the current location, inclusive. The first
    // element is the root house; the last is the node whose children are shown.
    // Drives the breadcrumb and the back action.
    var path: List<BridgeNode> by mutableStateOf(emptyList())
        private set

    // A one-shot signal that the move succeeded and the picker should pop back.
    // A successful move is a command, not a state the screen reads, so it goes
    // through a channel rather than an observable flag
    // (state-describes-what-is-not-what-should-happen).
    private val moved = Channel<Unit>(Channel.CONFLATED)
    val movedEvents: Flow<Unit> = moved.receiveAsFlow()

    init {
        // Load the root house and its children to start the walk. The picker is
        // its own flow, so it reads the root from the bridge rather than being
        // handed a root id from the browse stack.
        load(nodeId = null, prefix = emptyList())
    }

    /** The node whose children are currently shown, or null before the first load. */
    val current: BridgeNode?
        get() = path.lastOrNull()

    /** Descend into [node]: show its children with it appended to the breadcrumb. */
    fun descend(node: BridgeNode) {
        load(nodeId = node.id, prefix = path)
    }

    /**
     * Go up one level: drop the current node and reload its parent's children.
     * Does nothing at the root (the root is always the first breadcrumb element).
     */
    fun goUp() {
        if (path.size <= 1) return
        val parent = path[path.size - 2]
        load(nodeId = parent.id, prefix = path.dropLast(2))
    }

    /**
     * Move the node into the currently-shown location, then signal that the
     * picker should pop back. Surfaces a move failure (e.g. an unexpected cycle)
     * rather than masking it.
     */
    fun moveHere() {
        val destination = current ?: return
        viewModelScope.launch {
            val error = runBridgeWrite(TAG, "moving $movingId under ${destination.id}") {
                handle.moveNode(movingId, destination.id)
            }
            if (error == null) moved.send(Unit) else content = MovePickerContent.Failed(error)
        }
    }

    /**
     * The local file path for [imageId] if its file exists, else null; the cards
     * call it on the render path.
     */
    fun imagePath(imageId: String): String? = imagePath(handle, imageId)

    /**
     * Load the destination node and its children, landing a breadcrumb of
     * [prefix] plus the loaded node. A null [nodeId] loads the root house (the
     * start of the walk); a non-null one loads that node. Reads off-main, then
     * mutates the state here. The moving node is dropped from the children so it
     * can't be chosen or entered.
     */
    private fun load(nodeId: String?, prefix: List<BridgeNode>) {
        content = MovePickerContent.Loading
        viewModelScope.launch {
            val outcome = withContext(Dispatchers.IO) {
                try {
                    val node = if (nodeId != null) {
                        handle.getNode(nodeId)
                            ?: return@withContext LoadOutcome.Failed("This place no longer exists.")
                    } else {
                        handle.rootNode()
                    }
                    val children = handle.children(node.id).filter { it.id != movingId }
                    LoadOutcome.Loaded(node, children)
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    Log.e(TAG, "loading move destination ${nodeId ?: "root"} failed", e)
                    LoadOutcome.Failed(e.message ?: e.toString())
                }
            }
            when (outcome) {
                is LoadOutcome.Failed -> content = MovePickerContent.Failed(outcome.message)
                is LoadOutcome.Loaded -> {
                    path = prefix + outcome.node
                    content = MovePickerContent.Loaded(outcome.children)
                }
            }
        }
    }

    /**
     * The result of one off-main destination load: the node and its valid
     * children, or a failure message already logged. Carried back so the
     * breadcrumb and content transition happens together on the state.
     */
    private sealed interface LoadOutcome {
        data class Loaded(val node: BridgeNode, val children: List<BridgeNode>) : LoadOutcome

        data class Failed(val message: String) : LoadOutcome
    }
}
