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
import uniffi.visible_bridge.BridgeNodeDetail

private const val TAG = "visible.NodeDetailViewModel"

/** What the edit screen is showing while it loads or renders one node's details. */
sealed interface NodeDetailContent {
    data object Loading : NodeDetailContent

    data class Failed(val message: String) : NodeDetailContent

    data object Loaded : NodeDetailContent
}

/**
 * Loads and edits one node's attributes and tags. Bridge calls touch SQLite so
 * they run on [Dispatchers.IO]; the read-modify-write of the form state happens
 * here on the state, not in the composable
 * (observable-mutate-on-the-state-not-the-view). The composable reads these
 * fields and renders the form.
 *
 * The form holds each attribute in its editable representation: quantity and the
 * text fields as strings, value in dollars (seeded from the stored cents), and
 * the acquired date as a UTC epoch-millis [Long] for the Material date picker
 * (seeded from the stored ISO `YYYY-MM-DD` string). These conversions are
 * form-seeding — they live here on the view model, not in the composable, and
 * invert on save.
 */
class NodeDetailViewModel(
    private val handle: AppHandle,
    private val nodeId: String,
) : ViewModel() {
    var content: NodeDetailContent by mutableStateOf(NodeDetailContent.Loading)
        private set

    // The editable form fields, seeded from the loaded detail. Blank text fields
    // map to absence on save (the view model trims, core also normalizes).
    var quantity by mutableStateOf("")
    var valueDollars by mutableStateOf("")

    // The acquired date as UTC start-of-day epoch millis, the unit the Material
    // date picker speaks; null is "no date" (cleared, saved as absence).
    var acquiredDateMillis: Long? by mutableStateOf(null)
        private set

    var notes by mutableStateOf("")
    var serial by mutableStateOf("")
    var barcode by mutableStateOf("")

    var tags: List<String> by mutableStateOf(emptyList())
        private set
    var newTag by mutableStateOf("")

    // A save or tag write in flight, so the composable can disable the controls
    // while a write runs. Local UI state for the in-flight gesture.
    var working by mutableStateOf(false)
        private set

    var errorMessage: String? by mutableStateOf(null)
        private set

    /** Load the node's detail and seed the form from it. */
    fun reload() {
        viewModelScope.launch {
            val outcome = withContext(Dispatchers.IO) {
                try {
                    LoadOutcome.Loaded(handle.nodeDetail(nodeId))
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    Log.e(TAG, "loading detail for $nodeId failed", e)
                    LoadOutcome.Failed(e.message ?: e.toString())
                }
            }
            content = when (outcome) {
                is LoadOutcome.Loaded -> {
                    seed(outcome.detail)
                    NodeDetailContent.Loaded
                }
                is LoadOutcome.Failed -> NodeDetailContent.Failed(outcome.message)
            }
        }
    }

    /** The result of the off-main detail load, handed back to the main dispatcher. */
    private sealed interface LoadOutcome {
        data class Loaded(val detail: BridgeNodeDetail) : LoadOutcome

        data class Failed(val message: String) : LoadOutcome
    }

    /**
     * Seed the editable form fields from the loaded detail (form-seeding): cents
     * render as dollars, the ISO date string parses into UTC epoch millis, and an
     * absent field seeds blank.
     */
    private fun seed(detail: BridgeNodeDetail) {
        quantity = detail.quantity?.toString() ?: ""
        valueDollars = detail.valueCents?.let(NodeDetailLogic::dollarsFromCents) ?: ""
        acquiredDateMillis = detail.acquiredAt?.let(NodeDetailLogic::millisFromIso)
        notes = detail.notes ?: ""
        serial = detail.serial ?: ""
        barcode = detail.barcode ?: ""
        tags = detail.tags
    }

    /** Set the acquired date from the picker's UTC epoch millis. */
    fun setAcquiredDate(millis: Long?) {
        acquiredDateMillis = millis
    }

    /** Clear the acquired date (saved as absence). */
    fun clearAcquiredDate() {
        acquiredDateMillis = null
    }

    /**
     * Save the form's attributes. Quantity and value parse from their editable
     * strings (value dollars → cents); the acquired date renders back to the ISO
     * string; blank text fields map to null (the view model trims, core also
     * normalizes). Reloads after the write so the form reflects the stored state.
     */
    fun save() {
        val quantity = NodeDetailLogic.quantityFromText(quantity)
        val valueCents = NodeDetailLogic.centsFromDollars(valueDollars)
        val acquiredAt = acquiredDateMillis?.let(NodeDetailLogic::isoFromMillis)
        val notes = NodeDetailLogic.blankToNull(notes)
        val serial = NodeDetailLogic.blankToNull(serial)
        val barcode = NodeDetailLogic.blankToNull(barcode)

        errorMessage = null
        working = true
        viewModelScope.launch {
            val failure = runBridgeWrite(TAG, "saving attributes on $nodeId") {
                handle.updateNodeAttributes(nodeId, quantity, notes, valueCents, acquiredAt, serial, barcode)
            }
            working = false
            errorMessage = failure
            if (failure == null) reload()
        }
    }

    /**
     * Add the typed tag, clear the field, and reload the tags. Core trims and
     * ignores a blank tag, so an empty field is a no-op there too.
     */
    fun addTag() {
        val tag = newTag
        newTag = ""
        runTagWrite("adding tag to $nodeId") { handle.addNodeTag(nodeId, tag) }
    }

    /** Remove [tag] and reload the tags. */
    fun removeTag(tag: String) {
        runTagWrite("removing tag from $nodeId") { handle.removeNodeTag(nodeId, tag) }
    }

    /**
     * Run a tag write off-main, then reload just the tags so the chip list
     * reflects the change (the attributes the user is editing stay untouched).
     */
    private fun runTagWrite(description: String, write: () -> Unit) {
        errorMessage = null
        working = true
        viewModelScope.launch {
            val failure = runBridgeWrite(TAG, description, write)
            working = false
            errorMessage = failure
            if (failure == null) reloadTags()
        }
    }

    private fun reloadTags() {
        viewModelScope.launch {
            val loaded = withContext(Dispatchers.IO) {
                try {
                    handle.nodeDetail(nodeId).tags
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    Log.e(TAG, "reloading tags for $nodeId failed", e)
                    null
                }
            }
            if (loaded != null) tags = loaded
        }
    }
}
