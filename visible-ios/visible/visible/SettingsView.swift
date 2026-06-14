import SwiftUI

/// The settings shell for the open home, reached from the browse root's gear:
/// This Home (name + rename), Cloud Sync (the S3 connect form), Sharing & Members
/// (→ ``SharingView``), Switch Home (start a fresh home, or join / restore one,
/// each replacing the current home), and About (app version + library id). Shared
/// by iOS and macOS. The view only calls ``SettingsModel`` methods and renders;
/// the model owns the state mutation and the concurrency.
struct SettingsView: View {
    let handle: AppHandle
    let session: AppSession

    @State private var model: SettingsModel
    @State private var showSharing = false
    @State private var showRename = false

    init(handle: AppHandle, session: AppSession, rootId: String, libraryId: String) {
        self.handle = handle
        self.session = session
        _model = State(
            initialValue: SettingsModel(
                handle: handle,
                session: session,
                rootId: rootId,
                libraryId: libraryId
            )
        )
    }

    var body: some View {
        Form {
            thisHomeSection
            cloudSyncSection
            sharingSection
            switchHomeSection
            aboutSection

            if let error = model.errorMessage {
                Section {
                    Text(error)
                        .foregroundStyle(.red)
                }
            }
        }
        .inlineNavigationTitle("Settings")
        .task {
            model.loadHome()
            model.reload()
        }
        .navigationDestination(isPresented: $showSharing) {
            SharingView(handle: handle, session: session)
        }
        .sheet(isPresented: $showRename) {
            NameSheet(
                initial: renameSeed,
                onConfirm: { name in
                    model.renameHome(name)
                    showRename = false
                },
                onCancel: { showRename = false }
            )
        }
        .confirmationDialog(
            "This replaces your current home on this device.",
            isPresented: newHomeConfirmBinding,
            titleVisibility: .visible
        ) {
            Button("Replace home", role: .destructive) { model.startNewHome() }
            Button("Cancel", role: .cancel) { model.dismissNewHome() }
        }
    }

    /// Seed the rename sheet with the current home name, or empty while it loads.
    private var renameSeed: String {
        if case let .loaded(name) = model.homeName { return name }
        return ""
    }

    private var newHomeConfirmBinding: Binding<Bool> {
        Binding(
            get: { model.pendingNewHome != nil },
            set: { if !$0 { model.dismissNewHome() } }
        )
    }

    @ViewBuilder
    private var thisHomeSection: some View {
        Section("This Home") {
            switch model.homeName {
            case .loading:
                Text("Loading…")
                    .foregroundStyle(.secondary)
            case let .failed(message):
                Text(message)
                    .foregroundStyle(.red)
            case let .loaded(name):
                HStack {
                    NodeName(name: name.isEmpty ? nil : name)
                    Spacer()
                    Button("Rename") { showRename = true }
                        .disabled(model.working)
                }
            }
        }
    }

    @ViewBuilder
    private var cloudSyncSection: some View {
        Section("Cloud Sync") {
            Text(model.statusLine)
                .foregroundStyle(.secondary)

            TextField("Bucket", text: $model.bucket)
            TextField("Region", text: $model.region)
            TextField("Endpoint (optional)", text: $model.endpoint)
            TextField("Key prefix (optional)", text: $model.keyPrefix)
            TextField("Access key", text: $model.accessKey)
            SecureField("Secret key", text: $model.secretKey)
        }
        .textFieldStyle(.roundedBorder)
        #if os(iOS)
        .textInputAutocapitalization(.never)
        .autocorrectionDisabled()
        #endif

        Section {
            Button("Connect") { model.connect() }
                .disabled(!model.canConnect)

            if model.isConnected {
                Button("Sync now") { model.triggerSync() }
                    .disabled(model.working)

                Button("Disconnect", role: .destructive) { model.disconnect() }
                    .disabled(model.working)
            }
        }
    }

    private var sharingSection: some View {
        Section("Sharing & Members") {
            Button("Members & invites") { showSharing = true }
        }
    }

    private var switchHomeSection: some View {
        Section("Switch Home") {
            TextField("New home name", text: $model.newHomeName)
                .textFieldStyle(.roundedBorder)
                #if os(iOS)
                .textInputAutocapitalization(.words)
                #endif
            Button("Start a new home") { model.confirmNewHome() }
                .disabled(model.working)

            Button("Join or restore a home") { showSharing = true }
                .disabled(model.working)
        }
    }

    private var aboutSection: some View {
        Section("About") {
            LabeledContent("Version", value: model.appVersion)
            LabeledContent("Library id", value: model.libraryId)
                .font(.footnote)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)
        }
    }
}
