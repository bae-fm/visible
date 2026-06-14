import SwiftUI

/// A single-field name prompt for renaming a node. `onConfirm` receives the
/// trimmed name; the confirm button is disabled while the trimmed text is blank,
/// so an empty name can't be submitted.
struct NameSheet: View {
    let title: String
    let onConfirm: (String) -> Void
    let onCancel: () -> Void

    @State private var text: String

    init(
        title: String,
        initial: String,
        onConfirm: @escaping (String) -> Void,
        onCancel: @escaping () -> Void
    ) {
        self.title = title
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
                    #if os(iOS)
                    .textInputAutocapitalization(.sentences)
                    #endif
                    .submitLabel(.done)
                    .onSubmit { if !trimmed.isEmpty { onConfirm(trimmed) } }
            }
            .inlineNavigationTitle(title)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel", action: onCancel)
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button(title) { onConfirm(trimmed) }
                        .disabled(trimmed.isEmpty)
                }
            }
        }
        .sheetChrome()
    }
}
