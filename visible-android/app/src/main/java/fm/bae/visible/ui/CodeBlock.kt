package fm.bae.visible.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp

/**
 * A labelled, monospace code with Copy and Share actions. Renders the
 * sharing/restore/invite/identity codes the same way wherever they appear — the
 * sharing screen and the onboarding Welcome screen.
 */
@Composable
fun CodeBlock(label: String, code: String) {
    val context = LocalContext.current
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text(
            text = label,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(text = code, fontFamily = FontFamily.Monospace)
        Row {
            TextButton(onClick = { ShareActions.copy(context, label, code) }) { Text("Copy") }
            TextButton(onClick = { ShareActions.share(context, code) }) { Text("Share") }
        }
    }
}
