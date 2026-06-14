import Foundation

/// The pure role-to-label derivation behind ``SharingModel``, with no
/// `AppHandle` or `@Observable` state. The model delegates here so the labels
/// are exercised directly.
enum SharingLogic {
    /// The label for a role: the row label in the members list and the option
    /// label in the invite picker.
    static func roleName(_ role: BridgeMemberRole) -> String {
        switch role {
        case .owner: "Owner"
        case .member: "Member"
        case .follower: "Follower"
        }
    }
}
