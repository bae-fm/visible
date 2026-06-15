package fm.bae.visible.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp

/**
 * The settings shell for the open home, reached from the browse root's gear: This
 * Home (name + rename), Cloud Sync (the S3 connect form), Sharing & Members (→ the
 * sharing screen), Switch Home (start a fresh home, or join / restore one, each
 * replacing the current home), and About (app version + library id). The
 * composable reads the [SettingsViewModel] fields and renders them; the model owns
 * the mutation and the concurrency.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(
    viewModel: SettingsViewModel,
    onPop: () -> Unit,
    onOpenSharing: () -> Unit,
) {
    LaunchedEffect(viewModel) {
        viewModel.loadHome()
        viewModel.reload()
    }

    var showRename by remember { mutableStateOf(false) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Settings") },
                navigationIcon = {
                    IconButton(onClick = onPop) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp)
                .verticalScroll(rememberScrollState()),
            verticalArrangement = Arrangement.spacedBy(20.dp),
        ) {
            ThisHomeSection(viewModel, onRename = { showRename = true })
            CloudSyncSection(viewModel)
            SharingSection(onOpenSharing)
            SwitchHomeSection(viewModel, onOpenSharing)
            AboutSection(viewModel)

            viewModel.errorMessage?.let { message ->
                Text(text = message, color = MaterialTheme.colorScheme.error)
            }
        }
    }

    if (showRename) {
        val seed = (viewModel.homeName as? Loadable.Loaded)?.value ?: ""
        NameDialog(
            initial = seed,
            onConfirm = { name ->
                viewModel.renameHome(name)
                showRename = false
            },
            onDismiss = { showRename = false },
        )
    }

    viewModel.pendingNewHome?.let {
        AlertDialog(
            onDismissRequest = viewModel::dismissNewHome,
            title = { Text("Replace home?") },
            text = { Text("This replaces your current home on this device.") },
            confirmButton = {
                TextButton(onClick = viewModel::startNewHome) { Text("Replace home") }
            },
            dismissButton = {
                TextButton(onClick = viewModel::dismissNewHome) { Text("Cancel") }
            },
        )
    }
}

@Composable
private fun ThisHomeSection(viewModel: SettingsViewModel, onRename: () -> Unit) {
    SectionColumn("This Home") {
        when (val state = viewModel.homeName) {
            is Loadable.Loading -> Text(
                "Loading…",
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            is Loadable.Failed -> Text(state.message, color = MaterialTheme.colorScheme.error)
            is Loadable.Loaded -> Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                NodeName(
                    name = state.value.ifEmpty { null },
                    modifier = Modifier.weight(1f),
                )
                TextButton(onClick = onRename, enabled = !viewModel.working) { Text("Rename") }
            }
        }
    }
}

@Composable
private fun CloudSyncSection(viewModel: SettingsViewModel) {
    SectionColumn("Cloud Sync") {
        Text(text = viewModel.statusLine, color = MaterialTheme.colorScheme.onSurfaceVariant)

        S3Field(value = viewModel.bucket, onValueChange = { viewModel.bucket = it }, label = "Bucket")
        S3Field(value = viewModel.region, onValueChange = { viewModel.region = it }, label = "Region")
        S3Field(
            value = viewModel.endpoint,
            onValueChange = { viewModel.endpoint = it },
            label = "Endpoint (optional)",
        )
        S3Field(
            value = viewModel.keyPrefix,
            onValueChange = { viewModel.keyPrefix = it },
            label = "Key prefix (optional)",
        )
        S3Field(
            value = viewModel.accessKey,
            onValueChange = { viewModel.accessKey = it },
            label = "Access key",
        )
        S3Field(
            value = viewModel.secretKey,
            onValueChange = { viewModel.secretKey = it },
            label = "Secret key",
            isPassword = true,
        )

        Button(
            onClick = viewModel::connect,
            enabled = viewModel.canConnect,
            modifier = Modifier.fillMaxWidth(),
        ) {
            Text("Connect")
        }

        if (viewModel.isConnected) {
            OutlinedButton(
                onClick = viewModel::triggerSync,
                enabled = !viewModel.working,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text("Sync now")
            }

            OutlinedButton(
                onClick = viewModel::disconnect,
                enabled = !viewModel.working,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text("Disconnect")
            }
        }
    }
}

@Composable
private fun SharingSection(onOpenSharing: () -> Unit) {
    SectionColumn("Sharing & Members") {
        OutlinedButton(onClick = onOpenSharing, modifier = Modifier.fillMaxWidth()) {
            Text("Members & invites")
        }
    }
}

@Composable
private fun SwitchHomeSection(viewModel: SettingsViewModel, onOpenSharing: () -> Unit) {
    SectionColumn("Switch Home") {
        OutlinedTextField(
            value = viewModel.newHomeName,
            onValueChange = { viewModel.newHomeName = it },
            label = { Text("New home name") },
            singleLine = true,
            keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Words),
            modifier = Modifier.fillMaxWidth(),
        )
        Button(
            onClick = viewModel::confirmNewHome,
            enabled = !viewModel.working,
            modifier = Modifier.fillMaxWidth(),
        ) {
            Text("Start a new home")
        }
        OutlinedButton(
            onClick = onOpenSharing,
            enabled = !viewModel.working,
            modifier = Modifier.fillMaxWidth(),
        ) {
            Text("Join or restore a home")
        }
    }
}

@Composable
private fun AboutSection(viewModel: SettingsViewModel) {
    SectionColumn("About") {
        Row(modifier = Modifier.fillMaxWidth()) {
            Text("Version", modifier = Modifier.weight(1f))
            Text(viewModel.appVersion, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        Column {
            Text(
                text = "Library id",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            // The id is shown for support, so it must be copyable — matching the
            // iOS About row's .textSelection(.enabled).
            SelectionContainer {
                Text(
                    text = viewModel.libraryId,
                    style = MaterialTheme.typography.bodySmall,
                    fontFamily = FontFamily.Monospace,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

/** One full-width single-line S3 form field; [isPassword] masks the input. */
@Composable
private fun S3Field(
    value: String,
    onValueChange: (String) -> Unit,
    label: String,
    isPassword: Boolean = false,
) {
    OutlinedTextField(
        value = value,
        onValueChange = onValueChange,
        label = { Text(label) },
        singleLine = true,
        visualTransformation = if (isPassword) PasswordVisualTransformation() else VisualTransformation.None,
        keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.None),
        modifier = Modifier.fillMaxWidth(),
    )
}
