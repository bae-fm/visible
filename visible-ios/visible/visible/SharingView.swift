import SwiftUI

/// The sharing screen, reached from Settings: this device's identity code, the
/// member list (owner can remove), inviting someone, and joining or restoring a
/// home. Shared by iOS and macOS. The view only calls ``SharingModel`` methods
/// and renders; the model owns the state mutation and the concurrency.
struct SharingView: View {
    @State private var model: SharingModel

    init(handle: AppHandle, session: AppSession) {
        _model = State(initialValue: SharingModel(handle: handle, session: session))
    }

    var body: some View {
        Form {
            thisDeviceSection

            if model.connected {
                membersSection
                inviteSection
            }

            joinOrRestoreSection

            if let error = model.errorMessage {
                Section {
                    Text(error)
                        .foregroundStyle(.red)
                }
            }
        }
        .inlineNavigationTitle("Sharing")
        .task { model.reload() }
        .confirmationDialog(
            "Remove member? This re-keys the library.",
            isPresented: removeConfirmBinding,
            titleVisibility: .visible
        ) {
            Button("Remove", role: .destructive) { model.removePending() }
            Button("Cancel", role: .cancel) { model.dismissRemove() }
        }
        .confirmationDialog(
            "This replaces your current home on this device.",
            isPresented: switchConfirmBinding,
            titleVisibility: .visible
        ) {
            Button("Replace home", role: .destructive) { model.switchPending() }
            Button("Cancel", role: .cancel) { model.dismissSwitch() }
        }
    }

    private var removeConfirmBinding: Binding<Bool> {
        Binding(
            get: { model.pendingRemoval != nil },
            set: { if !$0 { model.dismissRemove() } }
        )
    }

    private var switchConfirmBinding: Binding<Bool> {
        Binding(
            get: { model.pendingSwitch != nil },
            set: { if !$0 { model.dismissSwitch() } }
        )
    }

    @ViewBuilder
    private var thisDeviceSection: some View {
        Section("This device") {
            switch model.identityCode {
            case .loading:
                Text("Loading…")
                    .foregroundStyle(.secondary)
            case let .failed(message):
                Text(message)
                    .foregroundStyle(.red)
            case let .loaded(code):
                CodeRow(label: "Your identity code", code: code)
            }
            Text("Send this to whoever owns the home you want to join.")
                .font(.footnote)
                .foregroundStyle(.secondary)
        }
    }

    @ViewBuilder
    private var membersSection: some View {
        Section("Members") {
            switch model.members {
            case .loading:
                Text("Loading…")
                    .foregroundStyle(.secondary)
            case let .failed(message):
                Text(message)
                    .foregroundStyle(.red)
            case let .loaded(members):
                if members.isEmpty {
                    Text("No members yet.")
                        .foregroundStyle(.secondary)
                } else {
                    ForEach(members, id: \.pubkey) { member in
                        MemberRow(
                            shortPubkey: member.shortPubkey,
                            role: model.roleLabel(member),
                            isSelf: member.isSelf,
                            onRemove: member.isSelf ? nil : { model.confirmRemove(member) }
                        )
                    }
                }
            }
        }
    }

    @ViewBuilder
    private var inviteSection: some View {
        Section("Invite someone") {
            TextField("Their identity code", text: $model.inviteIdentityCode)
                .textFieldStyle(.roundedBorder)
                #if os(iOS)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                #endif

            Picker("Role", selection: $model.inviteRole) {
                Text(model.roleName(.member)).tag(BridgeMemberRole.member)
                Text(model.roleName(.follower)).tag(BridgeMemberRole.follower)
            }

            Button("Create invite code") { model.invite() }
                .disabled(model.working)

            if let invite = model.inviteCode {
                CodeRow(label: "Invite code — send this back", code: invite)
            }
        }
    }

    private var joinOrRestoreSection: some View {
        Section("Join or restore a home") {
            TextField("Paste an invite code", text: $model.joinInviteCode)
                .textFieldStyle(.roundedBorder)
                #if os(iOS)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                #endif
            Button("Join home") { model.confirmJoin() }
                .disabled(model.working)

            TextField("Paste a restore code", text: $model.restoreInputCode)
                .textFieldStyle(.roundedBorder)
                #if os(iOS)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                #endif
            Button("Restore home") { model.confirmRestore() }
                .disabled(model.working)

            if model.connected {
                Button("Show my restore code") { model.showRestoreCode() }
                    .disabled(model.working)
            }
            if let code = model.restoreCode {
                CodeRow(label: "Restore code — save this", code: code)
            }
        }
    }
}

/// One member row: the shortened pubkey, the role, a "(this device)" marker for
/// the current device, and a Remove button when the owner can remove this member.
private struct MemberRow: View {
    let shortPubkey: String
    let role: String
    let isSelf: Bool
    let onRemove: (() -> Void)?

    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                Text(shortPubkey)
                    .font(.system(.body, design: .monospaced))
                Text(isSelf ? "\(role) (this device)" : role)
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }
            Spacer()
            if let onRemove {
                Button("Remove", role: .destructive, action: onRemove)
                    .buttonStyle(.borderless)
            }
        }
    }
}

