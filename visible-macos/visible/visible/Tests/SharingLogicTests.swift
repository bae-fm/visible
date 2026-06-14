import XCTest

@testable import visible

/// `SharingLogic.roleName` labels a member's role for the members list and the
/// invite picker. The macOS test target compiles the same shared Swift sources
/// the iOS app does, so this also covers the iOS model's logic.
final class SharingLogicTests: XCTestCase {
    func testRoleNameLabelsEachRole() {
        XCTAssertEqual(SharingLogic.roleName(.owner), "Owner")
        XCTAssertEqual(SharingLogic.roleName(.member), "Member")
        XCTAssertEqual(SharingLogic.roleName(.follower), "Follower")
    }
}
