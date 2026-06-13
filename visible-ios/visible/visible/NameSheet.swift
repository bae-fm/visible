import SwiftUI

/// A single-field name prompt used for both adding a child and renaming a node.
/// `onConfirm` receives the trimmed name; the confirm button is disabled while
/// the trimmed text is blank, so an empty name can't be submitted.
struct NameSheet: View {
    let title: String
    let confirmLabel: String
    let initial: String
    let onConfirm: (String) -> Void
    let onCancel: () -> Void

    @State private var text: String

    init(
        title: String,
        confirmLabel: String,
        initial: String,
        onConfirm: @escaping (String) -> Void,
        onCancel: @escaping () -> Void
    ) {
        self.title = title
        self.confirmLabel = confirmLabel
        self.initial = initial
        self.onConfirm = onConfirm
        self.onCancel = onCancel
        _text = State(initialValue: initial)
    }

    private var trimmed: String {
        text.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    var body: some View {
        NavigationStack {
            Form {
                TextField("Name", text: $text)
                    .textInputAutocapitalization(.sentences)
                    .submitLabel(.done)
                    .onSubmit { if !trimmed.isEmpty { onConfirm(trimmed) } }
            }
            .navigationTitle(title)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel", action: onCancel)
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button(confirmLabel) { onConfirm(trimmed) }
                        .disabled(trimmed.isEmpty)
                }
            }
        }
        .presentationDetents([.medium])
    }
}
