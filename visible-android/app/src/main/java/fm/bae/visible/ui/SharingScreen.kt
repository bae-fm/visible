package fm.bae.visible.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.text.KeyboardOptions
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
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.unit.dp
import uniffi.visible_bridge.BridgeMemberRole

/**
 * The sharing screen, reached from Settings: this device's identity code, the
 * member list (owner can remove), inviting someone, and joining or restoring a
 * home. The composable reads the [SharingViewModel] fields and renders them; the
 * model owns the mutation and the concurrency.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SharingScreen(
    viewModel: SharingViewModel,
    onPop: () -> Unit,
) {
    LaunchedEffect(viewModel) { viewModel.reload() }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Sharing") },
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
            ThisDeviceSection(viewModel)

            if (viewModel.connected) {
                MembersSection(viewModel)
                InviteSection(viewModel)
            }

            JoinOrRestoreSection(viewModel)

            viewModel.errorMessage?.let { message ->
                Text(text = message, color = MaterialTheme.colorScheme.error)
            }
        }
    }

    viewModel.pendingRemoval?.let {
        AlertDialog(
            onDismissRequest = viewModel::dismissRemove,
            title = { Text("Remove member?") },
            text = { Text("This re-keys the library.") },
            confirmButton = {
                TextButton(onClick = viewModel::removePending) { Text("Remove") }
            },
            dismissButton = {
                TextButton(onClick = viewModel::dismissRemove) { Text("Cancel") }
            },
        )
    }

    viewModel.pendingSwitch?.let {
        AlertDialog(
            onDismissRequest = viewModel::dismissSwitch,
            title = { Text("Replace home?") },
            text = { Text("This replaces your current home on this device.") },
            confirmButton = {
                TextButton(onClick = viewModel::switchPending) { Text("Replace home") }
            },
            dismissButton = {
                TextButton(onClick = viewModel::dismissSwitch) { Text("Cancel") }
            },
        )
    }
}

@Composable
private fun ThisDeviceSection(viewModel: SharingViewModel) {
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
            text = "Send this to whoever owns the home you want to join.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun MembersSection(viewModel: SharingViewModel) {
    SectionColumn("Members") {
        when (val state = viewModel.members) {
            is Loadable.Loading -> Text(
                "Loading…",
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            is Loadable.Failed -> Text(
                state.message,
                color = MaterialTheme.colorScheme.error,
            )
            is Loadable.Loaded -> if (state.value.isEmpty()) {
                Text("No members yet.", color = MaterialTheme.colorScheme.onSurfaceVariant)
            } else {
                state.value.forEach { member ->
                    MemberRow(
                        shortPubkey = member.shortPubkey,
                        role = viewModel.roleLabel(member),
                        isSelf = member.isSelf,
                        onRemove = if (member.isSelf) null else { -> viewModel.confirmRemove(member) },
                    )
                }
            }
        }
    }
}

@Composable
private fun InviteSection(viewModel: SharingViewModel) {
    SectionColumn("Invite someone") {
        OutlinedTextField(
            value = viewModel.inviteIdentityCode,
            onValueChange = { viewModel.inviteIdentityCode = it },
            label = { Text("Their identity code") },
            singleLine = true,
            keyboardOptions = KeyboardOptions(
                capitalization = KeyboardCapitalization.None,
                autoCorrectEnabled = false,
            ),
            modifier = Modifier.fillMaxWidth(),
        )

        RolePicker(viewModel)

        Button(
            onClick = viewModel::invite,
            enabled = !viewModel.working,
            modifier = Modifier.fillMaxWidth(),
        ) {
            Text("Create invite code")
        }

        viewModel.inviteCode?.let { code ->
            CodeBlock(label = "Invite code — send this back", code = code)
        }
    }
}

@Composable
private fun JoinOrRestoreSection(viewModel: SharingViewModel) {
    SectionColumn("Join or restore a home") {
        OutlinedTextField(
            value = viewModel.joinInviteCode,
            onValueChange = { viewModel.joinInviteCode = it },
            label = { Text("Paste an invite code") },
            singleLine = true,
            keyboardOptions = KeyboardOptions(
                capitalization = KeyboardCapitalization.None,
                autoCorrectEnabled = false,
            ),
            modifier = Modifier.fillMaxWidth(),
        )
        OutlinedButton(
            onClick = viewModel::confirmJoin,
            enabled = !viewModel.working,
            modifier = Modifier.fillMaxWidth(),
        ) {
            Text("Join home")
        }

        OutlinedTextField(
            value = viewModel.restoreInputCode,
            onValueChange = { viewModel.restoreInputCode = it },
            label = { Text("Paste a restore code") },
            singleLine = true,
            keyboardOptions = KeyboardOptions(
                capitalization = KeyboardCapitalization.None,
                autoCorrectEnabled = false,
            ),
            modifier = Modifier.fillMaxWidth(),
        )
        OutlinedButton(
            onClick = viewModel::confirmRestore,
            enabled = !viewModel.working,
            modifier = Modifier.fillMaxWidth(),
        ) {
            Text("Restore home")
        }

        if (viewModel.connected) {
            OutlinedButton(
                onClick = viewModel::showRestoreCode,
                enabled = !viewModel.working,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text("Show my restore code")
            }
        }
        viewModel.restoreCode?.let { code ->
            CodeBlock(label = "Restore code — save this", code = code)
        }
    }
}

/** One member row: the shortened pubkey, the role, a "(this device)" marker, and
 * a Remove button when the owner can remove this member. */
@Composable
private fun MemberRow(
    shortPubkey: String,
    role: String,
    isSelf: Boolean,
    onRemove: (() -> Unit)?,
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(text = shortPubkey, fontFamily = FontFamily.Monospace)
            Text(
                text = if (isSelf) "$role (this device)" else role,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        if (onRemove != null) {
            TextButton(onClick = onRemove) { Text("Remove") }
        }
    }
}

/** The grantable-role picker (Member / Follower — not Owner). */
@Composable
private fun RolePicker(viewModel: SharingViewModel) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        listOf(BridgeMemberRole.MEMBER, BridgeMemberRole.FOLLOWER).forEach { role ->
            val selected = viewModel.inviteRole == role
            Row(
                modifier = Modifier.selectable(
                    selected = selected,
                    onClick = { viewModel.inviteRole = role },
                ).padding(end = 16.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                RadioButton(selected = selected, onClick = { viewModel.inviteRole = role })
                Text(viewModel.roleName(role))
            }
        }
    }
}

