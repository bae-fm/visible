import Foundation
import os.log

private let logger = Logger.visible("SharingModel")

/// A value loaded off the main actor: in flight, failed with a message, or the
/// loaded value. Used for the identity code and the member list so a failure is
/// rendered, never silently stuck on "Loading…".
enum Loadable<Value: Sendable>: Sendable {
    case loading
    case failed(String)
    case loaded(Value)
}

/// Loads and mutates the sharing state for one library: this device's identity
/// code, the member list, inviting a member, and joining or restoring a home.
/// Bridge calls touch SQLite, the keyring, and the network, so they run off the
/// main actor; the read-modify-write of the screen state happens here on the
/// model, not in the view (observable-mutate-on-the-state-not-the-view). The
/// model owns the concurrency: each method launches its own `Task`, so the view
/// calls them synchronously and iterates over the model's fields to render them.
@MainActor
@Observable
final class SharingModel {
    private let handle: AppHandle
    private let session: AppSession

    /// Whether the sync loop is running. The members list, inviting, and the
    /// restore code require a connected library; the identity code and joining /
    /// restoring a home do not. nil until the first load.
    private(set) var connected = false

    /// This device's identity code, sent to a home's owner so they can invite
    /// this device. Loaded on appear.
    private(set) var identityCode: Loadable<String> = .loading

    /// The member list. Reloaded after a remove.
    private(set) var members: Loadable<[BridgeMember]> = .loading

    /// The invitee's identity code the owner pastes, and the role to grant.
    var inviteIdentityCode = ""
    var inviteRole: BridgeMemberRole = .member
    /// The invite code produced by the last successful invite, to send back to
    /// the invitee; nil until one is minted.
    private(set) var inviteCode: String?

    /// The codes pasted to join or restore a home, kept separate so each field
    /// holds its own value.
    var joinInviteCode = ""
    var restoreInputCode = ""

    /// This owner device's restore code, rendered for the user to save; nil until
    /// "Show my restore code" mints it.
    private(set) var restoreCode: String?

    /// The member the owner is confirming a remove of; nil when no confirm is up.
    /// Set by ``confirmRemove`` and cleared on confirm or dismiss.
    var pendingRemoval: BridgeMember?
    /// The join or restore the user is confirming (it replaces the current home);
    /// nil when no confirm is up.
    var pendingSwitch: HomeSwitch?

    /// A bridge call is in flight (disables the action buttons). Local UI state
    /// for the in-flight gesture, not a domain value.
    private(set) var working = false
    /// The last action failure, cleared on the next attempt.
    private(set) var errorMessage: String?

    init(handle: AppHandle, session: AppSession) {
        self.handle = handle
        self.session = session
    }

    /// A member's role as a row label. The role-to-label decision is domain, so
    /// it lives on the model, not the view.
    func roleLabel(_ member: BridgeMember) -> String {
        roleName(member.role)
    }

    /// The label for a grantable role in the invite picker.
    func roleName(_ role: BridgeMemberRole) -> String {
        switch role {
        case .owner: "Owner"
        case .member: "Member"
        case .follower: "Follower"
        }
    }

    /// Load the identity code, the connected flag, and the member list. The
    /// member list and connected flag are loaded together so the section
    /// visibility and its rows reflect the same point in time.
    func reload() {
        let handle = handle
        Task {
            let loaded = await Task.detached { () -> (Loadable<String>, Bool, Loadable<[BridgeMember]>) in
                let identity: Loadable<String>
                do {
                    identity = .loaded(try handle.userIdentityCode())
                } catch {
                    logger.error("loading identity code failed: \(error.localizedDescription, privacy: .public)")
                    identity = .failed(error.localizedDescription)
                }
                let connected = handle.syncStatus().ready
                let members: Loadable<[BridgeMember]>
                do {
                    members = .loaded(try handle.members())
                } catch {
                    logger.error("loading members failed: \(error.localizedDescription, privacy: .public)")
                    members = .failed(error.localizedDescription)
                }
                return (identity, connected, members)
            }.value
            identityCode = loaded.0
            connected = loaded.1
            members = loaded.2
        }
    }

    /// Open the remove confirmation for `member` (the owner removing another
    /// device). Removing re-keys the library.
    func confirmRemove(_ member: BridgeMember) {
        pendingRemoval = member
    }

    func dismissRemove() {
        pendingRemoval = nil
    }

    /// Remove the pending member, then reload the list. Re-keys the library in
    /// the bridge, so it runs off the main actor.
    func removePending() {
        guard let member = pendingRemoval else {
            logger.error("removePending called with no pending member")
            return
        }
        pendingRemoval = nil
        errorMessage = nil
        working = true
        Task {
            let failure = await BridgeWrite.run("removing member \(member.pubkey)", handle: handle) {
                try $0.removeMember(pubkey: member.pubkey)
            }
            working = false
            if let failure {
                errorMessage = failure
            } else {
                members = .loading
                reload()
            }
        }
    }

    /// Invite the device whose identity code is in the field, granting the picked
    /// role, and show the returned invite code to send back. Validates the field
    /// is non-empty.
    func invite() {
        let code = inviteIdentityCode.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !code.isEmpty else {
            errorMessage = "Paste the invitee's identity code first."
            return
        }
        let role = inviteRole
        errorMessage = nil
        working = true
        let handle = handle
        Task {
            let result = await Task.detached { () -> Result<String, Error> in
                do {
                    return .success(try handle.inviteMember(identityCode: code, role: role))
                } catch {
                    logger.error("inviting member failed: \(error.localizedDescription, privacy: .public)")
                    return .failure(error)
                }
            }.value
            working = false
            switch result {
            case let .success(invite):
                inviteCode = invite
            case let .failure(error):
                errorMessage = error.localizedDescription
            }
        }
    }

    /// Mint and show this owner device's restore code for the user to save.
    func showRestoreCode() {
        errorMessage = nil
        working = true
        let handle = handle
        Task {
            let result = await Task.detached { () -> Result<String, Error> in
                do {
                    return .success(try handle.restoreCode())
                } catch {
                    logger.error("loading restore code failed: \(error.localizedDescription, privacy: .public)")
                    return .failure(error)
                }
            }.value
            working = false
            switch result {
            case let .success(code):
                restoreCode = code
            case let .failure(error):
                errorMessage = error.localizedDescription
            }
        }
    }

    /// Open the replace confirmation for joining from the pasted invite code.
    func confirmJoin() {
        let code = joinInviteCode.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !code.isEmpty else {
            errorMessage = "Paste an invite code first."
            return
        }
        pendingSwitch = .join(code)
    }

    /// Open the replace confirmation for restoring from the pasted restore code.
    func confirmRestore() {
        let code = restoreInputCode.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !code.isEmpty else {
            errorMessage = "Paste a restore code first."
            return
        }
        pendingSwitch = .restore(code)
    }

    func dismissSwitch() {
        pendingSwitch = nil
    }

    /// Carry out the confirmed join or restore: ``AppSession/switchToHome(_:)``
    /// writes the new library to disk, opens it, and removes the current home. A
    /// failure there leaves the current home intact.
    func switchPending() {
        guard let pending = pendingSwitch else {
            logger.error("switchPending called with no pending switch")
            return
        }
        pendingSwitch = nil
        errorMessage = nil
        working = true
        let session = session
        Task {
            errorMessage = await session.switchToHome(pending)
            working = false
        }
    }
}

/// A pending home switch the user is confirming: joining from an invite code, or
/// restoring from a restore code. Both replace the current home on this device.
enum HomeSwitch: Identifiable {
    case join(String)
    case restore(String)

    var id: String {
        switch self {
        case let .join(code): "join-\(code)"
        case let .restore(code): "restore-\(code)"
        }
    }

    /// Write the new library to disk via the joiner-side core call for this
    /// source, returning its identity for the session to open. Runs off the main
    /// actor inside ``AppSession/switchToHome(_:)``.
    func writeLibrary(dataDir: String) throws -> BridgeLibrary {
        switch self {
        case let .join(code): try joinLibraryFromInvite(dataDir: dataDir, inviteCode: code)
        case let .restore(code): try restoreLibraryFromCode(dataDir: dataDir, restoreCode: code)
        }
    }

    /// A label for the log line on failure, naming the operation without the code.
    var logLabel: String {
        switch self {
        case .join: "joining from invite code"
        case .restore: "restoring from restore code"
        }
    }
}

