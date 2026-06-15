package fm.bae.visible.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.unit.dp

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
