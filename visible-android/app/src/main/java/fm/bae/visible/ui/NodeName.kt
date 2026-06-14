package fm.bae.visible.ui

import androidx.compose.material3.LocalTextStyle
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.TextOverflow

/**
 * Renders a node's title. A named node shows its name; an untitled node
 * (name = null, e.g. a photo-first child not yet renamed) shows a dimmed
 * "Untitled" placeholder so the absence reads as a placeholder, not a real name.
 */
@Composable
fun NodeName(
    name: String?,
    modifier: Modifier = Modifier,
    style: TextStyle = LocalTextStyle.current,
    maxLines: Int = Int.MAX_VALUE,
) {
    if (name != null) {
        Text(
            text = name,
            style = style,
            maxLines = maxLines,
            overflow = TextOverflow.Ellipsis,
            modifier = modifier,
        )
    } else {
        Text(
            text = "Untitled",
            style = style,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            maxLines = maxLines,
            overflow = TextOverflow.Ellipsis,
            modifier = modifier,
        )
    }
}
