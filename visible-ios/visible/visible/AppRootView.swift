import SwiftUI

/// Opens the session, then hosts the browse navigation stack once it is open.
/// Reads the session's published ``SessionState`` so a library switch (join /
/// restore) re-renders the stack onto the new home.
struct AppRootView: View {
    let session: AppSession

    var body: some View {
        Group {
            switch session.state {
            case .loading:
                ProgressView()
            case .onboarding:
                WelcomeView(session: session)
            case let .failed(message):
                VStack(spacing: 16) {
                    Text(message)
                        .foregroundStyle(.red)
                        .multilineTextAlignment(.center)
                    Button("Retry") { Task { await session.open() } }
                }
                .padding(24)
            case let .open(handle, rootId, libraryId):
                BrowseNavigation(session: session, handle: handle, rootId: rootId, libraryId: libraryId)
                    // Re-key the stack on the open library's root so a switch to a
                    // joined home resets the navigation path to the new root.
                    .id(rootId)
            }
        }
        .task { await session.open() }
    }
}

/// The browse navigation stack, one screen per node id. The root house is the
/// stack's root; tapping a child appends its id; the system back button or a
/// delete of the current node pops the last id.
private struct BrowseNavigation: View {
    let session: AppSession
    let handle: AppHandle
    let rootId: String
    let libraryId: String

    @State private var path: [String] = []
    @State private var showSettings = false
    @State private var showSearch = false
    // The node whose detail edit screen is open, pushed onto the stack; nil while
    // no detail screen is shown. A node id rather than a Bool because the screen
    // edits a specific node.
    @State private var detailNodeId: String?

    var body: some View {
        NavigationStack(path: $path) {
            BrowseView(
                handle: handle,
                nodeId: rootId,
                onOpenChild: { path.append($0) },
                onPop: {},
                onOpenDetail: { detailNodeId = $0 },
                onOpenSearch: { showSearch = true },
                onOpenSettings: { showSettings = true }
            )
            .navigationDestination(for: String.self) { nodeId in
                BrowseView(
                    handle: handle,
                    nodeId: nodeId,
                    onOpenChild: { path.append($0) },
                    onPop: { if !path.isEmpty { path.removeLast() } },
                    onOpenDetail: { detailNodeId = $0 },
                    onOpenSearch: { showSearch = true }
                )
            }
            .navigationDestination(isPresented: $showSettings) {
                SettingsView(handle: handle, session: session, rootId: rootId, libraryId: libraryId)
            }
            .navigationDestination(isPresented: $showSearch) {
                SearchView(handle: handle, onNavigate: navigate)
            }
            .navigationDestination(item: $detailNodeId) { nodeId in
                NodeDetailView(handle: handle, nodeId: nodeId)
            }
        }
        .tint(Theme.accent)
    }

    /// Land the browse stack on a searched node: set the navigation path to the
    /// node's ancestor ids below the root (the root is the stack's own root), and
    /// pop the search screen. Rebuilding the path this way puts every ancestor on
    /// the stack, so the back button from the landed node walks up the real
    /// ancestor chain to the root.
    private func navigate(to breadcrumb: [BridgeNode]) {
        path = breadcrumb.dropFirst().map(\.id)
        showSearch = false
    }
}
