import SwiftUI

/// A labelled, selectable, monospace code with Copy and (on iOS) Share actions.
/// Renders the sharing/restore/invite/identity codes the same way wherever they
/// appear — the sharing screen and the onboarding Welcome screen.
struct CodeRow: View {
    let label: String
    let code: String

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(label)
                .font(.footnote)
                .foregroundStyle(.secondary)
            Text(code)
                .font(.system(.body, design: .monospaced))
                .textSelection(.enabled)
            HStack {
                Button("Copy") { ShareActions.copy(code) }
                    .buttonStyle(.borderless)
                #if os(iOS)
                Button("Share") { ShareActions.share(code) }
                    .buttonStyle(.borderless)
                #endif
            }
        }
    }
}
