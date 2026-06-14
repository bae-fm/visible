import SwiftUI

/// Renders a node's title. A named node shows its name; an untitled node
/// (name = nil, e.g. a photo-first child not yet renamed) shows a dimmed
/// "Untitled" placeholder so the absence reads as a placeholder, not a real name.
struct NodeName: View {
    let name: String?

    var body: some View {
        if let name {
            Text(name)
        } else {
            Text("Untitled")
                .foregroundStyle(.secondary)
        }
    }
}
