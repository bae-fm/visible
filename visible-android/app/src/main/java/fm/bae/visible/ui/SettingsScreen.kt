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
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp

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
                text = viewModel.statusLine,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

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
