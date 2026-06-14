package fm.bae.visible.ui

import android.util.Log
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.visible_bridge.AppHandle
import uniffi.visible_bridge.BridgeSearchResult

private const val TAG = "visible.SearchViewModel"

/**
 * What the search screen is showing for the current query. Idle is the
 * empty-query state (nothing to search yet); Loading is a search in flight;
 * Results carries a non-empty hit list; NoMatches is a non-empty query that
 * matched nothing; Failed carries a bridge error message. The states are
 * distinct so the screen never shows a perpetual spinner, and a failed search
 * reads as a failure rather than as "no matches" (never-mask).
 */
sealed interface SearchState {
    data object Idle : SearchState

    data object Loading : SearchState

    data class Results(val hits: List<BridgeSearchResult>) : SearchState

    data object NoMatches : SearchState

    data class Failed(val message: String) : SearchState
}

/**
 * Loads search results for one query. The composable binds its text field to
 * [query]; each change runs the search and lands a [SearchState]. Bridge calls
 * touch SQLite so they run on [Dispatchers.IO]; the state mutation happens here
 * on the state, not in the composable
 * (observable-mutate-on-the-state-not-the-view). The composable iterates over
 * [state] and renders it.
 */
class SearchViewModel(
    private val handle: AppHandle,
) : ViewModel() {
    var query: String by mutableStateOf("")
        private set

    var state: SearchState by mutableStateOf(SearchState.Idle)
        private set

    // The in-flight search, cancelled when a newer keystroke supersedes it so a
    // slow earlier query can't land its results over a newer one.
    private var searchJob: Job? = null

    /** Update the query and run the search for it. */
    fun onQueryChange(newQuery: String) {
        query = newQuery
        search()
    }

    /**
     * Run the search for the current query. An empty or whitespace-only query
     * has nothing to match, so it resets to Idle without calling the bridge (core
     * would return no results anyway). Otherwise show Loading, then the results
     * or the no-matches state, or the failure.
     */
    private fun search() {
        searchJob?.cancel()

        val query = query
        if (query.isBlank()) {
            state = SearchState.Idle
            return
        }

        state = SearchState.Loading
        searchJob = viewModelScope.launch {
            state = withContext(Dispatchers.IO) {
                try {
                    val hits = handle.search(query)
                    if (hits.isEmpty()) SearchState.NoMatches else SearchState.Results(hits)
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    Log.e(TAG, "searching for $query failed", e)
                    SearchState.Failed(e.message ?: e.toString())
                }
            }
        }
    }

    /**
     * The local file path for [imageId] if its file exists, else null. The bridge
     * call does no database work (a filesystem existence check), so the row's
     * thumbnail calls it directly on the render path.
     */
    fun imagePath(imageId: String): String? {
        val path = handle.imagePathIfExists(imageId)
        if (path == null) {
            Log.d(TAG, "no image file for $imageId; showing placeholder")
        }
        return path
    }
}
