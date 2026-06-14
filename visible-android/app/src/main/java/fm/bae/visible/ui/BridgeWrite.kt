package fm.bae.visible.ui

import android.util.Log
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * Run a bridge write off the main thread. Returns null on success or the failure
 * message. Bridge writes touch SQLite, the keyring, and the network, so they run
 * on [Dispatchers.IO]; a [CancellationException] is rethrown so coroutine
 * cancellation propagates, any other failure is logged under [tag] and its
 * message returned for the model to surface.
 */
suspend fun runBridgeWrite(tag: String, description: String, write: () -> Unit): String? =
    withContext(Dispatchers.IO) {
        try {
            write()
            null
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            Log.e(tag, "$description failed", e)
            e.message ?: e.toString()
        }
    }
