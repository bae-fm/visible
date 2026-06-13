import SwiftUI

/// Opens the session, then hosts the browse navigation stack once it is open.
struct AppRootView: View {
    let session: AppSession

    @State private var state: SessionState = .loading
    // Bumping this re-keys the open task, which re-runs session.open. A failure
    // is not cached, so the retry re-attempts it.
    @State private var attempt = 0

    var body: some View {
        Group {
            switch state {
            case .loading:
                ProgressView()
            case let .failed(message):
                VStack(spacing: 16) {
                    Text(message)
                        .foregroundStyle(.red)
                        .multilineTextAlignment(.center)
                    Button("Retry") { attempt += 1 }
                }
                .padding(24)
            case let .open(handle, rootId):
                BrowseNavigation(handle: handle, rootId: rootId)
            }
        }
        .task(id: attempt) {
            state = .loading
            state = await session.open()
        }
    }
}

/// The browse navigation stack, one screen per node id. The root house is the
/// stack's root; tapping a child appends its id; the system back button or a
/// delete of the current node pops the last id.
private struct BrowseNavigation: View {
    let handle: AppHandle
    let rootId: String

    @State private var path: [String] = []

    var body: some View {
        NavigationStack(path: $path) {
            BrowseView(
                handle: handle,
                nodeId: rootId,
                onOpenChild: { path.append($0) },
                onPop: {}
            )
            .navigationDestination(for: String.self) { nodeId in
                BrowseView(
                    handle: handle,
                    nodeId: nodeId,
                    onOpenChild: { path.append($0) },
                    onPop: { if !path.isEmpty { path.removeLast() } }
                )
            }
        }
        .tint(Theme.accent)
    }
}
