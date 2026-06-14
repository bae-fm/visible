package fm.bae.visible.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
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
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import uniffi.visible_bridge.BridgeOutboxSnapshot
import uniffi.visible_bridge.BridgeSyncStatus

/**
 * The cloud-sync settings screen: an S3 connection form, a Connect/Disconnect
 * action, and a status line. Reached from the browse root's gear. The composable
 * reads the [SettingsViewModel] fields and renders them; the model owns the
 * mutation and the concurrency.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(
    viewModel: SettingsViewModel,
    onPop: () -> Unit,
) {
    LaunchedEffect(viewModel) { viewModel.reload() }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Sync") },
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
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text(
                text = statusLine(viewModel.working, viewModel.status, viewModel.outbox),
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            val noCaps = KeyboardOptions(capitalization = KeyboardCapitalization.None)
            OutlinedTextField(
                value = viewModel.bucket,
                onValueChange = { viewModel.bucket = it },
                label = { Text("Bucket") },
                singleLine = true,
                keyboardOptions = noCaps,
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = viewModel.region,
                onValueChange = { viewModel.region = it },
                label = { Text("Region") },
                singleLine = true,
                keyboardOptions = noCaps,
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = viewModel.endpoint,
                onValueChange = { viewModel.endpoint = it },
                label = { Text("Endpoint (optional)") },
                singleLine = true,
                keyboardOptions = noCaps,
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = viewModel.keyPrefix,
                onValueChange = { viewModel.keyPrefix = it },
                label = { Text("Key prefix (optional)") },
                singleLine = true,
                keyboardOptions = noCaps,
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = viewModel.accessKey,
                onValueChange = { viewModel.accessKey = it },
                label = { Text("Access key") },
                singleLine = true,
                keyboardOptions = noCaps,
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = viewModel.secretKey,
                onValueChange = { viewModel.secretKey = it },
                label = { Text("Secret key") },
                singleLine = true,
                visualTransformation = PasswordVisualTransformation(),
                keyboardOptions = noCaps,
                modifier = Modifier.fillMaxWidth(),
            )

            viewModel.errorMessage?.let { message ->
                Text(text = message, color = MaterialTheme.colorScheme.error)
            }

            Button(
                onClick = viewModel::connect,
                enabled = viewModel.canConnect,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text("Connect")
            }

            if (viewModel.status?.configured == true) {
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
}

/**
 * The one-line status: the in-flight connect, then the configured/ready state,
 * with the pending outbox counts appended when there is work queued. Built from
 * the booleans and counts the bridge already provides plus the model's local
 * in-flight flag — no domain re-derivation.
 */
private fun statusLine(
    working: Boolean,
    status: BridgeSyncStatus?,
    outbox: BridgeOutboxSnapshot?,
): String {
    if (working) return "Connecting…"
    if (status?.configured != true) return "Not connected"
    val base = if (status.ready) "Synced" else "Connected (starting…)"
    return base + pendingSuffix(outbox)
}

/** `" · N to upload, M to delete"` when the outbox has pending work, else empty. */
private fun pendingSuffix(outbox: BridgeOutboxSnapshot?): String {
    if (outbox == null) return ""
    val parts = buildList {
        if (outbox.pendingUploads > 0u) add("${outbox.pendingUploads} to upload")
        if (outbox.pendingDeletes > 0u) add("${outbox.pendingDeletes} to delete")
    }
    return if (parts.isEmpty()) "" else " · " + parts.joinToString(", ")
}
