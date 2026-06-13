import SwiftUI

/// Confirms deleting a node and everything inside it.
struct DeleteConfirmSheet: View {
    let name: String
    let onConfirm: () -> Void
    let onCancel: () -> Void

    var body: some View {
        NavigationStack {
            VStack {
                Text("Delete \"\(name)\" and everything in it?")
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
}
