package fm.bae.visible.ui

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import androidx.core.content.getSystemService

/**
 * Copy and share affordances for the sharing codes. Copy writes to the system
 * clipboard; share opens the system share sheet ([Intent.ACTION_SEND]). Codes are
 * copy/paste text — no QR in this screen.
 */
object ShareActions {
    /** Write [text] to the system clipboard under [label]. */
    fun copy(context: Context, label: String, text: String) {
        val clipboard = context.getSystemService<ClipboardManager>()
            ?: error("ClipboardManager is unavailable")
        clipboard.setPrimaryClip(ClipData.newPlainText(label, text))
    }

    /** Open the system share sheet for [text]. */
    fun share(context: Context, text: String) {
        val send = Intent(Intent.ACTION_SEND).apply {
            type = "text/plain"
            putExtra(Intent.EXTRA_TEXT, text)
        }
        context.startActivity(Intent.createChooser(send, null))
    }
}
