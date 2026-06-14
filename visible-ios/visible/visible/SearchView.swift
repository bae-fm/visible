import SwiftUI

/// Search the whole tree by node name. An auto-focused text field drives
/// ``SearchModel``; the screen renders the model's tri-state (idle / loading /
/// results / no matches / failed). Each result row shows the node's thumbnail,
/// its name, and the core-built ancestor breadcrumb; tapping a row navigates the
/// browse stack to that node. Shared by iOS and macOS. The view iterates over the
/// model's state and renders it; the model owns the search and the concurrency.
struct SearchView: View {
    // Hand the tapped result's breadcrumb (root→node ancestor ids) up to the
    // browse navigation, which resets the stack so the landed node's back button
    // walks up the real ancestor chain.
    let onNavigate: ([BridgeNode]) -> Void

    @State private var model: SearchModel
    @FocusState private var queryFocused: Bool

    init(handle: AppHandle, onNavigate: @escaping ([BridgeNode]) -> Void) {
        self.onNavigate = onNavigate
        _model = State(initialValue: SearchModel(handle: handle))
    }

    var body: some View {
        VStack(spacing: 0) {
            TextField("Search", text: $model.query)
                .textFieldStyle(.roundedBorder)
                .focused($queryFocused)
                #if os(iOS)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                #endif
                .padding(16)

            results
        }
        .inlineNavigationTitle("Search")
        .onAppear { queryFocused = true }
    }

    @ViewBuilder
    private var results: some View {
        switch model.state {
        case .idle:
            hint("Search for anything in your home by name.")
        case .loading:
            ProgressView()
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        case .noMatches:
            hint("No matches.")
        case let .failed(message):
            Text(message)
                .foregroundStyle(.red)
                .multilineTextAlignment(.center)
                .padding(24)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        case let .results(hits):
            List(hits, id: \.node.id) { hit in
                Button { onNavigate(hit.path) } label: {
                    SearchResultRow(hit: hit, path: hit.node.imageId.flatMap(model.imagePath))
                }
                .buttonStyle(.plain)
            }
            .listStyle(.plain)
        }
    }

    private func hint(_ text: String) -> some View {
        Text(text)
            .foregroundStyle(.secondary)
            .multilineTextAlignment(.center)
            .padding(24)
            .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

/// One search match: its thumbnail, name, and the ancestor breadcrumb the core
/// built (rendered as-is; the view does not join names). An untitled match shows
/// the shared "Untitled" placeholder; a match directly under the root still has
/// the root in its breadcrumb, so the breadcrumb is hidden only when empty.
private struct SearchResultRow: View {
    let hit: BridgeSearchResult
    let path: String?

    var body: some View {
        HStack(spacing: 12) {
            NodeImageView(path: path, cornerRadius: 8)
                .frame(width: 48, height: 48)

            VStack(alignment: .leading, spacing: 2) {
                NodeName(name: hit.node.name)
                    .font(.body)
                    .lineLimit(1)
                if !hit.pathLabel.isEmpty {
                    Text(hit.pathLabel)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .contentShape(Rectangle())
    }
}
