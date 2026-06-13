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
            .navigationTitle("Delete")
            #if os(iOS)
            .navigationBarTitleDisplayMode(.inline)
            #endif
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel", action: onCancel)
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Delete", role: .destructive, action: onConfirm)
                }
            }
        }
        #if os(iOS)
        .presentationDetents([.medium])
        #else
        .frame(minWidth: 360, minHeight: 160)
        #endif
    }
}
