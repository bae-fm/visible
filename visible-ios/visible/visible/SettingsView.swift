import SwiftUI

/// The cloud-sync settings screen: an S3 connection form, a Connect/Disconnect
/// action, and a status line. Reached from the browse root's gear. Shared by iOS
/// and macOS. The view only calls ``SettingsModel`` methods and renders; the
/// model owns the state mutation and the concurrency.
struct SettingsView: View {
    let handle: AppHandle
    let session: AppSession

    @State private var model: SettingsModel
    @State private var showSharing = false

    init(handle: AppHandle, session: AppSession) {
        self.handle = handle
        self.session = session
        _model = State(initialValue: SettingsModel(handle: handle))
    }

    var body: some View {
        Form {
            Section("Status") {
                Text(model.statusLine)
                    .foregroundStyle(.secondary)
            }

            Section("Sharing") {
                Button("Members & invites") { showSharing = true }
            }

            Section("Amazon S3") {
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

            if let error = model.errorMessage {
                Section {
                    Text(error)
                        .foregroundStyle(.red)
                }
            }

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
        .inlineNavigationTitle("Sync")
        .task { model.reload() }
        .navigationDestination(isPresented: $showSharing) {
            SharingView(handle: handle, session: session)
        }
    }
}
