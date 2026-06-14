import SwiftUI

/// Confirms removing a node's photo. The node and its contents stay; only the
/// image is cleared.
struct RemovePhotoConfirmSheet: View {
    let onConfirm: () -> Void
    let onCancel: () -> Void

    var body: some View {
        NavigationStack {
            VStack {
                Text("Remove this photo?")
                    .multilineTextAlignment(.center)
                    .padding()
                Spacer()
            }
            .frame(maxWidth: .infinity)
            .inlineNavigationTitle("Remove Photo")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel", action: onCancel)
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Remove", role: .destructive, action: onConfirm)
                }
            }
        }
        .sheetChrome()
    }
}
