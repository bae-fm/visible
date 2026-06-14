import SwiftUI

/// First-run onboarding shown when no library exists yet: create a home, or join /
/// restore one from a code, plus this device's identity code to send to a home's
/// owner. Shared by iOS and macOS. On completion the session opens onto the home
/// and ``AppRootView`` replaces this screen with the browse stack. The view only
/// calls ``WelcomeModel`` methods and renders; the model owns the state mutation
/// and the concurrency.
struct WelcomeView: View {
    @State private var model: WelcomeModel

    init(session: AppSession) {
        _model = State(initialValue: WelcomeModel(session: session))
    }

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    Text("Set up the home you want to keep track of, or join one a co-householder already shares with you.")
                        .foregroundStyle(.secondary)
                }

                Section("Create a home") {
                    TextField("Home name", text: $model.homeName)
                        .textFieldStyle(.roundedBorder)
                        #if os(iOS)
                        .textInputAutocapitalization(.words)
                        #endif
                    Button("Create home") { model.createHome() }
                        .disabled(!model.canCreate)
                }

                Section("Join a home") {
                    TextField("Paste an invite code", text: $model.joinInviteCode)
                        .textFieldStyle(.roundedBorder)
                        #if os(iOS)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        #endif
                    Button("Join home") { model.joinHome() }
                        .disabled(model.working)

                    TextField("Paste a restore code", text: $model.restoreInputCode)
                        .textFieldStyle(.roundedBorder)
                        #if os(iOS)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        #endif
                    Button("Restore home") { model.restoreHome() }
                        .disabled(model.working)
                }

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
                    Text("Send this to whoever owns the home you want to join, so they can invite this device.")
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }

                if let error = model.errorMessage {
                    Section {
                        Text(error)
                            .foregroundStyle(.red)
                    }
                }
            }
            .inlineNavigationTitle("Welcome")
            .task { model.reload() }
        }
        .tint(Theme.accent)
    }
}
