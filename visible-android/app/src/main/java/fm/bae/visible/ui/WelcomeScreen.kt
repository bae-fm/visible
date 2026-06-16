package fm.bae.visible.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Visibility
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/**
 * First-run onboarding shown when no library exists yet: create a home, or join /
 * restore one from a code, plus this device's identity code to send to a home's
 * owner. On completion the session opens onto the home and [AppRoot] replaces this
 * screen with the browse stack. The composable reads the [WelcomeViewModel] fields
 * and renders them; the model owns the mutation and the concurrency.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun WelcomeScreen(viewModel: WelcomeViewModel) {
    LaunchedEffect(viewModel) { viewModel.reload() }

    Scaffold(
        topBar = { TopAppBar(title = { Text("Welcome") }) },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp)
                .verticalScroll(rememberScrollState()),
            verticalArrangement = Arrangement.spacedBy(20.dp),
        ) {
            Brand(modifier = Modifier.fillMaxWidth().padding(top = 8.dp, bottom = 4.dp))

            Text(
                text = "Set up the home you want to keep track of, or join one a " +
                    "co-householder already shares with you.",
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            CreateSection(viewModel)
            JoinSection(viewModel)
            ThisDeviceSection(viewModel)

            viewModel.errorMessage?.let { message ->
                Text(text = message, color = MaterialTheme.colorScheme.error)
            }
        }
    }
}

@Composable
private fun CreateSection(viewModel: WelcomeViewModel) {
    SectionColumn("Create a home") {
        OutlinedTextField(
            value = viewModel.homeName,
            onValueChange = { viewModel.homeName = it },
            label = { Text("Home name") },
            singleLine = true,
            keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Words),
            modifier = Modifier.fillMaxWidth(),
        )
        Button(
            onClick = viewModel::createHome,
            enabled = viewModel.canCreate,
            modifier = Modifier.fillMaxWidth(),
        ) {
            Text("Create home")
        }
        Text(
            text = "Your house sits at the top; rooms, shelves and things branch below it.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

/**
 * The visible brand mark: an eye glyph in an accent rounded square beside the
 * "visible" wordmark (the "visi" stem tinted in the accent), with the
 * "See what you own." tagline.
 */
@Composable
private fun Brand(modifier: Modifier = Modifier) {
    Column(
        modifier = modifier,
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Box(
                modifier = Modifier
                    .size(44.dp)
                    .clip(RoundedCornerShape(11.dp))
                    .background(MaterialTheme.colorScheme.primary),
                contentAlignment = Alignment.Center,
            ) {
                Icon(
                    Icons.Filled.Visibility,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.onPrimary,
                    modifier = Modifier.size(24.dp),
                )
            }
            Text(
                text = buildAnnotatedString {
                    withStyle(SpanStyle(color = MaterialTheme.colorScheme.primary)) { append("visi") }
                    append("ble")
                },
                fontSize = 34.sp,
                fontWeight = FontWeight.Bold,
                letterSpacing = (-1).sp,
            )
        }
        Text(
            text = "See what you own.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun JoinSection(viewModel: WelcomeViewModel) {
    SectionColumn("Join a home") {
        CodeEntryField(
            value = viewModel.joinInviteCode,
            onValueChange = { viewModel.joinInviteCode = it },
            label = "Paste an invite code",
            buttonLabel = "Join home",
            isWorking = viewModel.working,
            onSubmit = viewModel::joinHome,
        )
        CodeEntryField(
            value = viewModel.restoreInputCode,
            onValueChange = { viewModel.restoreInputCode = it },
            label = "Paste a restore code",
            buttonLabel = "Restore home",
            isWorking = viewModel.working,
            onSubmit = viewModel::restoreHome,
        )
    }
}

/**
 * A paste-a-code field and its submit button. The invite and restore rows are the
 * same shape, differing only in their label, bound field, button label, and action.
 */
@Composable
private fun CodeEntryField(
    value: String,
    onValueChange: (String) -> Unit,
    label: String,
    buttonLabel: String,
    isWorking: Boolean,
    onSubmit: () -> Unit,
) {
    OutlinedTextField(
        value = value,
        onValueChange = onValueChange,
        label = { Text(label) },
        singleLine = true,
        keyboardOptions = KeyboardOptions(
            capitalization = KeyboardCapitalization.None,
            autoCorrectEnabled = false,
        ),
        modifier = Modifier.fillMaxWidth(),
    )
    OutlinedButton(
        onClick = onSubmit,
        enabled = !isWorking,
        modifier = Modifier.fillMaxWidth(),
    ) {
        Text(buttonLabel)
    }
}

@Composable
private fun ThisDeviceSection(viewModel: WelcomeViewModel) {
    SectionColumn("This device") {
        when (val state = viewModel.identityCode) {
            is Loadable.Loading -> Text(
                "Loading…",
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            is Loadable.Failed -> Text(
                state.message,
                color = MaterialTheme.colorScheme.error,
            )
            is Loadable.Loaded -> CodeBlock(label = "Your identity code", code = state.value)
        }
        Text(
            text = "Send this to whoever owns the home you want to join, so they can " +
                "invite this device.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}
