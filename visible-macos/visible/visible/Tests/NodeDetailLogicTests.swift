import XCTest

@testable import visible

/// `NodeDetailLogic` converts the node edit form between its stored shape (cents,
/// ISO date) and its editable shape (dollars string, `Date?`). The macOS test
/// target compiles the same shared Swift sources the iOS app does, so this also
/// covers the iOS model's logic.
final class NodeDetailLogicTests: XCTestCase {
    func testCentsAndDollarsRoundTrip() {
        // A cents amount survives the cents → dollars string → cents round trip.
        XCTAssertEqual(NodeDetailLogic.dollars(fromCents: 1299), "12.99")
        XCTAssertEqual(NodeDetailLogic.cents(fromDollars: "12.99"), 1299)
        XCTAssertEqual(NodeDetailLogic.cents(fromDollars: NodeDetailLogic.dollars(fromCents: 500)), 500)
    }

    func testDollarsToCentsRoundsToTheNearestCent() {
        // Plain (half-up) rounding so a third-of-a-cent input lands on a whole cent.
        XCTAssertEqual(NodeDetailLogic.cents(fromDollars: "12.985"), 1299)
    }

    func testCentsFromBlankIsNil() {
        XCTAssertNil(NodeDetailLogic.cents(fromDollars: ""))
        XCTAssertNil(NodeDetailLogic.cents(fromDollars: "   "))
    }

    func testCentsFromUnparseableIsNil() {
        XCTAssertNil(NodeDetailLogic.cents(fromDollars: "not a number"))
    }

    func testInt64ParsesWholeNumbers() {
        XCTAssertEqual(NodeDetailLogic.int64(from: " 7 "), 7)
    }

    func testInt64FromBlankIsNil() {
        XCTAssertNil(NodeDetailLogic.int64(from: ""))
    }

    func testInt64FromUnparseableIsNil() {
        XCTAssertNil(NodeDetailLogic.int64(from: "1.5"))
        XCTAssertNil(NodeDetailLogic.int64(from: "abc"))
    }

    func testBlankToNilTrimsAndDropsBlanks() {
        XCTAssertEqual(NodeDetailLogic.blankToNil("  note  "), "note")
        XCTAssertNil(NodeDetailLogic.blankToNil("   "))
    }

    func testDateIsoRoundTrip() throws {
        let iso = "2024-01-02"
        let date = try XCTUnwrap(NodeDetailLogic.dateFromIso(iso))
        XCTAssertEqual(NodeDetailLogic.isoFromDate(date), iso)
    }

    func testDateFromUnparseableIsoIsNil() {
        XCTAssertNil(NodeDetailLogic.dateFromIso("not a date"))
    }
}
