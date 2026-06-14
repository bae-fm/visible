import SwiftUI

/// The cloud-sync settings screen: an S3 connection form, a Connect/Disconnect
/// action, and a status line. Reached from the browse root's gear. Shared by iOS
/// and macOS. The view only calls ``SettingsModel`` methods and renders; the
/// model owns the state mutation and the concurrency.
struct SettingsView: View {
    @State private var model: SettingsModel

    init(handle: AppHandle) {
        _model = State(initialValue: SettingsModel(handle: handle))
    }

    var body: some View {
        Form {
            Section("Status") {
                Text(statusLine)
                    .foregroundStyle(.secondary)
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

                if isConnected {
                    Button("Disconnect", role: .destructive) { model.disconnect() }
                        .disabled(model.working)
                }
            }
        }
        .inlineNavigationTitle("Sync")
        .task { model.reload() }
    }

    /// Whether a provider is configured (a Disconnect action makes sense).
    private var isConnected: Bool {
        model.status?.configured ?? false
    }

    /// The one-line status: the in-flight connect, then the configured/ready
    /// state, with the pending outbox counts appended when there is work queued.
    /// Built from the booleans and counts the bridge already provides plus the
    /// model's local in-flight flag — no domain re-derivation.
    private var statusLine: String {
        if model.working {
            return "Connecting…"
        }
        guard let status = model.status, status.configured else {
            return "Not connected"
        }
        let base = status.ready ? "Synced" : "Connected (starting…)"
        return base + pendingSuffix
    }

    /// `" · N to upload, M to delete"` when the outbox has pending work, else
    /// empty. The counts come pre-computed from the bridge.
    private var pendingSuffix: String {
        guard let outbox = model.outbox else { return "" }
        var parts: [String] = []
        if outbox.pendingUploads > 0 { parts.append("\(outbox.pendingUploads) to upload") }
        if outbox.pendingDeletes > 0 { parts.append("\(outbox.pendingDeletes) to delete") }
        return parts.isEmpty ? "" : " · " + parts.joined(separator: ", ")
    }
}
