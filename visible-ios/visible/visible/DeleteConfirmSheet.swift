import SwiftUI

/// Confirms deleting a node and everything inside it.
struct DeleteConfirmSheet: View {
    let name: String?
    let onConfirm: () -> Void
    let onCancel: () -> Void

    var body: some View {
        NavigationStack {
            VStack {
                message
                    .multilineTextAlignment(.center)
                    .padding()
                Spacer()
            }
            .frame(maxWidth: .infinity)
            .inlineNavigationTitle("Delete")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel", action: onCancel)
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Delete", role: .destructive, action: onConfirm)
                }
            }
        }
        .sheetChrome()
    }

    /// Names a titled node directly; shows a dimmed "Untitled" for one with no
    /// name, so the absence reads as a placeholder rather than a name.
    private var message: Text {
        let nodeName = if let name { Text(name) } else { Text("Untitled").foregroundStyle(.secondary) }
        return Text("Delete \"") + nodeName + Text("\" and everything in it?")
    }
}
