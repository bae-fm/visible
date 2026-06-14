import XCTest

@testable import visible

/// `SettingsLogic` derives the settings status line, the connect gate, and the
/// optional-field absence mapping `SettingsModel` delegates to. The macOS test
/// target compiles the same shared Swift sources the iOS app does, so this also
/// covers the iOS model's logic.
final class SettingsLogicTests: XCTestCase {
    func testStatusLineShowsConnectingWhileWorking() {
        // The in-flight flag wins over everything else.
        XCTAssertEqual(
            SettingsLogic.statusLine(working: true, configured: true, ready: true, pendingDeletes: 3),
            "Connecting…"
        )
    }

    func testStatusLineNotConnectedWhenNoProvider() {
        XCTAssertEqual(
            SettingsLogic.statusLine(working: false, configured: false, ready: false, pendingDeletes: 0),
            "Not connected"
        )
    }

    func testStatusLineStartingWhenConfiguredButNotReady() {
        XCTAssertEqual(
            SettingsLogic.statusLine(working: false, configured: true, ready: false, pendingDeletes: 0),
            "Connected (starting…)"
        )
    }

    func testStatusLineSyncedWithNoPending() {
        XCTAssertEqual(
            SettingsLogic.statusLine(working: false, configured: true, ready: true, pendingDeletes: 0),
            "Synced"
        )
    }

    func testStatusLineAppendsPendingDeleteCount() {
        XCTAssertEqual(
            SettingsLogic.statusLine(working: false, configured: true, ready: true, pendingDeletes: 2),
            "Synced · 2 to delete"
        )
    }

    func testCanConnectRequiresBucketRegionAndBothKeys() {
        XCTAssertTrue(SettingsLogic.canConnect(bucket: "b", region: "r", accessKey: "ak", secretKey: "sk", working: false))
    }

    func testCanConnectFalseWhenAnyRequiredFieldIsBlank() {
        XCTAssertFalse(SettingsLogic.canConnect(bucket: "", region: "r", accessKey: "ak", secretKey: "sk", working: false))
        XCTAssertFalse(SettingsLogic.canConnect(bucket: "b", region: "", accessKey: "ak", secretKey: "sk", working: false))
        XCTAssertFalse(SettingsLogic.canConnect(bucket: "b", region: "r", accessKey: "", secretKey: "sk", working: false))
        XCTAssertFalse(SettingsLogic.canConnect(bucket: "b", region: "r", accessKey: "ak", secretKey: "", working: false))
    }

    func testCanConnectFalseWhileWorking() {
        // The endpoint and prefix are optional, so a full set still connects, but
        // an in-flight connect disables it regardless.
        XCTAssertFalse(SettingsLogic.canConnect(bucket: "b", region: "r", accessKey: "ak", secretKey: "sk", working: true))
    }

    func testOptionalFieldTrimsAndKeepsAValue() {
        XCTAssertEqual(SettingsLogic.optionalField("  https://s3.example.com  "), "https://s3.example.com")
    }

    func testOptionalFieldMapsBlankToNil() {
        XCTAssertNil(SettingsLogic.optionalField(""))
        XCTAssertNil(SettingsLogic.optionalField("   "))
    }
}
