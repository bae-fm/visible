package fm.bae.visible.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.wrapContentHeight
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.InputChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.rememberDatePickerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp

/**
 * The node-detail edit screen: a form of the node's attributes (quantity, value,
 * acquired date, notes, serial, barcode) and a tag editor. Reached from the node
 * actions menu. The composable reads [NodeDetailViewModel] fields and renders
 * them; the view model owns the form-seeding, the parse on save, and the
 * concurrency.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun NodeDetailScreen(
    viewModel: NodeDetailViewModel,
    onPop: () -> Unit,
) {
    LaunchedEffect(viewModel) { viewModel.reload() }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Details") },
                navigationIcon = {
                    IconButton(onClick = onPop) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        Box(modifier = Modifier.fillMaxSize().padding(padding)) {
            when (viewModel.content) {
                is NodeDetailContent.Loading ->
                    CircularProgressIndicator(Modifier.align(Alignment.Center))
                is NodeDetailContent.Failed -> Text(
                    text = (viewModel.content as NodeDetailContent.Failed).message,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.align(Alignment.Center).padding(24.dp),
                )
                is NodeDetailContent.Loaded -> Form(viewModel)
            }
        }
    }
}

@Composable
private fun Form(viewModel: NodeDetailViewModel) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(16.dp)
            .verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        OutlinedTextField(
            value = viewModel.quantity,
            onValueChange = { viewModel.quantity = it },
            label = { Text("Quantity") },
            singleLine = true,
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
            modifier = Modifier.fillMaxWidth(),
        )
        OutlinedTextField(
            value = viewModel.valueDollars,
            onValueChange = { viewModel.valueDollars = it },
            label = { Text("Value") },
            prefix = { Text("$") },
            singleLine = true,
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
            modifier = Modifier.fillMaxWidth(),
        )

        AcquiredDateField(
            millis = viewModel.acquiredDateMillis,
            onPick = viewModel::setAcquiredDate,
            onClear = viewModel::clearAcquiredDate,
        )

        OutlinedTextField(
            value = viewModel.serial,
            onValueChange = { viewModel.serial = it },
            label = { Text("Serial") },
            singleLine = true,
            keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.None),
            modifier = Modifier.fillMaxWidth(),
        )
        OutlinedTextField(
            value = viewModel.barcode,
            onValueChange = { viewModel.barcode = it },
            label = { Text("Barcode") },
            singleLine = true,
            keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.None),
            modifier = Modifier.fillMaxWidth(),
        )
        OutlinedTextField(
            value = viewModel.notes,
            onValueChange = { viewModel.notes = it },
            label = { Text("Notes") },
            minLines = 3,
            modifier = Modifier.fillMaxWidth(),
        )

        Text("Tags", style = MaterialTheme.typography.titleSmall)
        TagEditor(
            tags = viewModel.tags,
            newTag = viewModel.newTag,
            onNewTagChange = { viewModel.newTag = it },
            onAdd = viewModel::addTag,
            onRemove = viewModel::removeTag,
        )

        viewModel.errorMessage?.let { message ->
            Text(text = message, color = MaterialTheme.colorScheme.error)
        }

        Button(
            onClick = viewModel::save,
            enabled = !viewModel.working,
            modifier = Modifier.fillMaxWidth(),
        ) {
            Text("Save")
        }
    }
}

/**
 * The acquired-date row: a button showing the picked date (or a prompt) that opens
 * the Material date picker, plus a Clear control when a date is set. The picker
 * speaks UTC epoch millis, the same unit the view model seeds and saves from.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun AcquiredDateField(
    millis: Long?,
    onPick: (Long?) -> Unit,
    onClear: () -> Unit,
) {
    var showPicker by remember { mutableStateOf(false) }

    OutlinedButton(
        onClick = { showPicker = true },
        modifier = Modifier.fillMaxWidth(),
    ) {
        Text(if (millis != null) "Acquired: ${NodeDetailLogic.isoFromMillis(millis)}" else "Add acquired date")
    }
    if (millis != null) {
        TextButton(onClick = onClear) { Text("Clear date") }
    }

    if (showPicker) {
        val state = rememberDatePickerState(initialSelectedDateMillis = millis)
        DatePickerDialog(
            onDismissRequest = { showPicker = false },
            confirmButton = {
                TextButton(onClick = {
                    onPick(state.selectedDateMillis)
                    showPicker = false
                }) {
                    Text("OK")
                }
            },
            dismissButton = {
                TextButton(onClick = { showPicker = false }) { Text("Cancel") }
            },
        ) {
            DatePicker(state = state)
        }
    }
}

/** The tag editor: a wrapping row of removable chips plus a field to add one. */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
private fun TagEditor(
    tags: List<String>,
    newTag: String,
    onNewTagChange: (String) -> Unit,
    onAdd: () -> Unit,
    onRemove: (String) -> Unit,
) {
    if (tags.isNotEmpty()) {
        FlowRow(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            tags.forEach { tag ->
                InputChip(
                    selected = false,
                    onClick = { onRemove(tag) },
                    label = { Text(tag) },
                    trailingIcon = {
                        Icon(
                            Icons.Filled.Close,
                            contentDescription = "Remove tag",
                            modifier = Modifier.wrapContentHeight(),
                        )
                    },
                )
            }
        }
    }
    OutlinedTextField(
        value = newTag,
        onValueChange = onNewTagChange,
        label = { Text("Add a tag") },
        singleLine = true,
        keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.None),
        trailingIcon = {
            TextButton(onClick = onAdd, enabled = newTag.trim().isNotEmpty()) {
                Text("Add")
            }
        },
        modifier = Modifier.fillMaxWidth(),
    )
}
