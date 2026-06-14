import Foundation
import os.log

private let logger = Logger.visible("WelcomeModel")

/// The first-run onboarding state: name and create a home, or join / restore one
/// from a code, plus this device's identity code to hand to a home's owner. Bridge
/// calls touch the keyring, disk, and the network, so they run off the main actor;
/// the read-modify-write of the screen state happens here on the model, not in the
/// view (observable-mutate-on-the-state-not-the-view). Each method launches its
/// own `Task`, so the view calls them synchronously and renders the model's fields.
@MainActor
@Observable
final class WelcomeModel {
    private let session: AppSession

    /// The name for a new home, seeded with a suggestion the user can edit
    /// (form-seeding exemption). Trimmed on submit; the create button is disabled
    /// while it is blank.
    var homeName = "Home"

    /// The codes pasted to join or restore an existing home, kept separate so each
    /// field holds its own value.
    var joinInviteCode = ""
    var restoreInputCode = ""

    /// This device's identity code, sent to a home's owner so they can invite this
    /// device before it has a library. Loaded on appear; read from the global
    /// keyring keypair, so it works with no library on disk.
    private(set) var identityCode: Loadable<String> = .loading

    /// An onboarding call is in flight (disables the action buttons). Local UI
    /// state for the in-flight gesture, not a domain value.
    private(set) var working = false
    /// The last onboarding failure, cleared on the next attempt.
    private(set) var errorMessage: String?

    init(session: AppSession) {
        self.session = session
    }

    /// Whether the create button has a non-blank name.
    var canCreate: Bool {
        !working && !homeName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    /// Load this device's identity code from the global keyring keypair.
    func reload() {
        Task {
            let loaded = await Task.detached { () -> Loadable<String> in
                do {
                    return .loaded(try userIdentityCode())
                } catch {
                    logger.error("loading identity code failed: \(error.localizedDescription, privacy: .public)")
                    return .failed(error.localizedDescription)
                }
            }.value
            identityCode = loaded
        }
    }

    /// Create a fresh local home named after the (trimmed) name field. On success
    /// the session opens onto the new home; a failure surfaces in ``errorMessage``.
    func createHome() {
        let name = homeName.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !name.isEmpty else {
            errorMessage = "Give your home a name first."
            return
        }
        run { await $0.createHome(name: name) }
    }

    /// Join an existing home from the pasted invite code.
    func joinHome() {
        let code = joinInviteCode.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !code.isEmpty else {
            errorMessage = "Paste an invite code first."
            return
        }
        run { await $0.joinHome(code: code) }
    }

    /// Restore an existing home from the pasted restore code.
    func restoreHome() {
        let code = restoreInputCode.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !code.isEmpty else {
            errorMessage = "Paste a restore code first."
            return
        }
        run { await $0.restoreHome(code: code) }
    }

    /// Mark the onboarding call in flight, run it, then clear the flag. On success
    /// the session publishes ``SessionState/open`` and this view is replaced, so
    /// only the failure message lands back here.
    private func run(_ complete: @escaping (AppSession) async -> String?) {
        errorMessage = nil
        working = true
        let session = session
        Task {
            errorMessage = await complete(session)
            working = false
        }
    }
}
