package fm.bae.visible.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.dp

/**
 * A titled section: the header label, a divider, then the section content. Shared
 * by the sharing, settings, and onboarding Welcome screens so a section reads the
 * same wherever it appears.
 */
@Composable
fun SectionColumn(title: String, content: @Composable () -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(text = title, style = MaterialTheme.typography.titleMedium)
        HorizontalDivider()
        content()
    }
}
