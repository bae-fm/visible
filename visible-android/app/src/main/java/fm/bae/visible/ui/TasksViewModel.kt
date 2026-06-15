package fm.bae.visible.ui

import android.util.Log
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.visible_bridge.AppHandle
import uniffi.visible_bridge.BridgeTask

private const val TAG = "visible.TasksViewModel"

/**
 * Loads and edits the home's shared task list. The list is synced across the
 * home's members, so what loads here reflects the latest merged sync and every
 * add, check-off, rename, or delete shows up on co-householders' devices. Bridge
 * calls touch SQLite so they run on [Dispatchers.IO]; the state mutation happens
 * here on the view model (observable-mutate-on-the-state-not-the-view). The
 * composable iterates over [content] and renders it.
 */
class TasksViewModel(private val handle: AppHandle) : ViewModel() {
    var content: Loadable<List<BridgeTask>> by mutableStateOf(Loadable.Loading)
        private set

    /** The new-task field, blank initially (form-seeding). Trimmed on add. */
    var newTitle by mutableStateOf("")

    /** A write is in flight, disabling the add control. Local UI state. */
    var working by mutableStateOf(false)
        private set

    /** The last write failure, cleared on the next attempt. */
    var errorMessage: String? by mutableStateOf(null)
        private set

    init {
        reload()
    }

    /** Whether the add control is enabled: a non-blank title and no write running. */
    val canAdd: Boolean
        get() = newTitle.trim().isNotEmpty() && !working

    /** Load the shared task list. */
    fun reload() {
        viewModelScope.launch {
            content = withContext(Dispatchers.IO) {
                try {
                    Loadable.Loaded(handle.tasks())
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    Log.e(TAG, "loading tasks failed", e)
                    Loadable.Failed(e.message ?: e.toString())
                }
            }
        }
    }

    /** Add the typed task and clear the field. Core trims and rejects a blank
     * title, and the add control is disabled while blank, so this is the normal
     * path. Reloads after. */
    fun add() {
        val title = newTitle
        newTitle = ""
        runWrite("adding a task") { handle.createTask(title) }
    }

    /** Check a task off, or back on. */
    fun setDone(task: BridgeTask, done: Boolean) {
        runWrite("updating a task") { handle.setTaskDone(task.id, done) }
    }

    /** Rename a task. */
    fun rename(id: String, title: String) {
        runWrite("renaming a task") { handle.renameTask(id, title) }
    }

    /** Remove a task from the shared list. */
    fun delete(id: String) {
        runWrite("deleting a task") { handle.deleteTask(id) }
    }

    private fun runWrite(description: String, write: () -> Unit) {
        errorMessage = null
        working = true
        viewModelScope.launch {
            val failure = runBridgeWrite(TAG, description, write)
            working = false
            errorMessage = failure
            reload()
        }
    }
}
