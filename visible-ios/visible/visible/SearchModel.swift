import Foundation
import os.log

private let logger = Logger.visible("SearchModel")

/// What the search screen is showing for the current query. Idle is the
/// empty-query state (nothing to search yet); loading is a search in flight;
/// results carries a non-empty hit list; noMatches is a non-empty query that
/// matched nothing; failed carries a bridge error message. The states are
/// distinct so the screen never shows a perpetual spinner, and a failed search
/// reads as a failure rather than as "no matches" (never-mask).
enum SearchState {
    case idle
    case loading
    case results([BridgeSearchResult])
    case noMatches
    case failed(String)
}

/// Loads search results for one query off the main actor. The view binds its
/// text field to ``query``; each change runs the search and lands a
/// ``SearchState``. Bridge calls touch SQLite so they run on a detached task;
/// the state mutation happens here on the model, not in the view
/// (observable-mutate-on-the-state-not-the-view). The view iterates over
/// ``state`` and renders it.
@MainActor
@Observable
final class SearchModel {
    private let handle: AppHandle

    var query: String = "" {
        didSet { search() }
    }

    private(set) var state: SearchState = .idle

    // The in-flight search, cancelled when a newer keystroke supersedes it so a
    // slow earlier query can't land its results over a newer one.
    @ObservationIgnored
    private var searchTask: Task<Void, Never>?

    init(handle: AppHandle) {
        self.handle = handle
    }

    /// Run the search for the current query. An empty or whitespace-only query
    /// has nothing to match, so it resets to idle without calling the bridge
    /// (core would return no results anyway). Otherwise show loading, then the
    /// results or the no-matches state.
    private func search() {
        searchTask?.cancel()

        let query = query
        if query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            state = .idle
            return
        }

        state = .loading
        let handle = handle
        searchTask = Task {
            let outcome = await Task.detached { () -> SearchOutcome in
                do {
                    return .hits(try handle.search(query: query))
                } catch {
                    logger.error("searching for \(query, privacy: .public) failed: \(error.localizedDescription, privacy: .public)")
                    return .failed(error.localizedDescription)
                }
            }.value
            if Task.isCancelled { return }
            switch outcome {
            case let .failed(message):
                state = .failed(message)
            case let .hits(hits):
                state = hits.isEmpty ? .noMatches : .results(hits)
            }
        }
    }

    /// The result of one off-main search call: either the hits or a failure
    /// message already logged. Carried back to the main actor so the state
    /// transition (results / no-matches vs failed) happens on the model.
    private enum SearchOutcome {
        case hits([BridgeSearchResult])
        case failed(String)
    }

    /// The local file path for `imageId` if its file exists, else nil. The bridge
    /// call does no database work (a filesystem existence check), so the row's
    /// thumbnail calls it directly on the render path.
    func imagePath(_ imageId: String) -> String? {
        let path = handle.imagePathIfExists(imageId: imageId)
        if path == nil {
            logger.debug("no image file for \(imageId, privacy: .public); showing placeholder")
        }
        return path
    }
}
