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
            case let .failed(message):
                VStack(spacing: 16) {
                    Text(message)
                        .foregroundStyle(.red)
                        .multilineTextAlignment(.center)
                    Button("Retry") { Task { await session.open() } }
                }
                .padding(24)
            case let .open(handle, rootId):
                BrowseNavigation(session: session, handle: handle, rootId: rootId)
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

    @State private var path: [String] = []
    @State private var showSettings = false

    var body: some View {
        NavigationStack(path: $path) {
            BrowseView(
                handle: handle,
                nodeId: rootId,
                onOpenChild: { path.append($0) },
                onPop: {},
                onOpenSettings: { showSettings = true }
            )
            .navigationDestination(for: String.self) { nodeId in
                BrowseView(
                    handle: handle,
                    nodeId: nodeId,
                    onOpenChild: { path.append($0) },
                    onPop: { if !path.isEmpty { path.removeLast() } }
                )
            }
            .navigationDestination(isPresented: $showSettings) {
                SettingsView(handle: handle, session: session)
            }
        }
        .tint(Theme.accent)
    }
}
