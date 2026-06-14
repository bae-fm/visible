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

private const val TAG = "visible.BrowseViewModel"

/** What the screen is showing while it loads or renders one node. */
sealed interface BrowseContent {
    data object Loading : BrowseContent

    data class Failed(val message: String) : BrowseContent

    data class Loaded(val node: BridgeNode, val children: List<BridgeNode>) : BrowseContent
}

/** The dialog currently open over the screen, if any. */
sealed interface BrowseDialog {
    data class Rename(val target: BridgeNode) : BrowseDialog

    data class ConfirmDelete(val target: BridgeNode) : BrowseDialog
}

/**
 * Loads and mutates one node's browse state. Bridge calls touch SQLite so they
 * run on [Dispatchers.IO]; the read-modify-write of the screen state happens
 * here on the state, not in the composable
 * (observable-mutate-on-the-state-not-the-view). The composable iterates over
 * [content] and renders it.
 */
class BrowseViewModel(
    private val handle: AppHandle,
    private val nodeId: String,
) : ViewModel() {
    var content: BrowseContent by mutableStateOf(BrowseContent.Loading)
        private set

    var dialog: BrowseDialog? by mutableStateOf(null)
        private set

    // A one-shot signal that this node was deleted and the screen showing it
    // should pop back to its parent. A delete is a command, not a state the
    // screen reads, so it goes through a channel rather than an observable flag
    // (state-describes-what-is-not-what-should-happen).
    private val deletedSelf = Channel<Unit>(Channel.CONFLATED)
    val deletedSelfEvents: Flow<Unit> = deletedSelf.receiveAsFlow()

    fun reload() {
        viewModelScope.launch {
            content = withContext(Dispatchers.IO) {
                try {
                    val node = handle.getNode(nodeId)
                        ?: return@withContext BrowseContent.Failed("This item no longer exists.")
                    BrowseContent.Loaded(node, handle.children(nodeId))
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    Log.e(TAG, "loading node $nodeId failed", e)
                    BrowseContent.Failed(e.message ?: e.toString())
                }
            }
        }
    }

    fun openRename(target: BridgeNode) {
        dialog = BrowseDialog.Rename(target)
    }

    fun openDelete(target: BridgeNode) {
        dialog = BrowseDialog.ConfirmDelete(target)
    }

    fun dismissDialog() {
        dialog = null
    }

    /**
     * Create a new child under this node carrying [bytes] as its photo and an
     * empty name — the photo is the thing's identity until it is titled (by
     * rename, or later by on-device vision). The node and its image are written
     * in one atomic core call, so the child never appears without its photo.
     */
    fun addChildWithPhoto(bytes: ByteArray) {
        mutate("creating child of $nodeId with photo") {
            handle.createNodeWithImage(nodeId, "", bytes)
        }
    }

    fun rename(id: String, name: String) {
        dialog = null
        mutate("renaming $id") { handle.renameNode(id, name) }
    }

    /**
     * Delete [id]. Deleting a child reloads this screen; deleting this node
     * itself signals the screen to pop to the parent (reloading a deleted node
     * would only show a dead screen).
     */
    fun delete(id: String) {
        dialog = null
        if (id == nodeId) {
            viewModelScope.launch {
                val error = runWrite("deleting $nodeId") { handle.deleteNode(nodeId) }
                if (error == null) deletedSelf.send(Unit) else content = BrowseContent.Failed(error)
            }
        } else {
            mutate("deleting $id") { handle.deleteNode(id) }
        }
    }

    fun setImage(bytes: ByteArray) {
        mutate("setting image on $nodeId") { handle.setNodeImage(nodeId, bytes) }
    }

    /**
     * The local file path for [imageId] if its file exists, else null. The
     * bridge call does no database work (it is a filesystem existence check), so
     * the image composables call it directly on the render path.
     */
    fun imagePath(imageId: String): String? {
        val path = handle.imagePathIfExists(imageId)
        if (path == null) {
            // A node whose image file isn't on disk renders the placeholder.
            Log.d(TAG, "no image file for $imageId; showing placeholder")
        }
        return path
    }

    /** Runs a bridge write off-main, then reloads to reflect the new state. */
    private fun mutate(description: String, write: () -> Unit) {
        viewModelScope.launch {
            val error = runWrite(description, write)
            if (error == null) {
                reload()
            } else {
                content = BrowseContent.Failed(error)
            }
        }
    }

    /** Runs a bridge write off-main; returns null on success or the message. */
    private suspend fun runWrite(description: String, write: () -> Unit): String? =
        withContext(Dispatchers.IO) {
            try {
                write()
                null
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                Log.e(TAG, "$description failed", e)
                e.message ?: e.toString()
            }
        }
}
