import SwiftUI

/// The node-detail edit screen: a form of the node's attributes (quantity, value,
/// acquired date, notes, serial, barcode) and a tag editor. Reached from the node
/// actions menu. The view reads ``NodeDetailModel`` fields and renders them; the
/// model owns the form-seeding, the parse on save, and the concurrency. Shared by
/// iOS and macOS.
struct NodeDetailView: View {
    @State private var model: NodeDetailModel

    init(handle: AppHandle, nodeId: String) {
        _model = State(initialValue: NodeDetailModel(handle: handle, nodeId: nodeId))
    }

    var body: some View {
        content
            .inlineNavigationTitle("Details")
            .task { model.reload() }
    }

    @ViewBuilder
    private var content: some View {
        switch model.content {
        case .loading:
            ProgressView()
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        case let .failed(message):
            Text(message)
                .foregroundStyle(.red)
                .multilineTextAlignment(.center)
                .padding(24)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        case .loaded:
            form
        }
    }

    private var form: some View {
        Form {
            Section("Attributes") {
                LabeledContent("Quantity") {
                    TextField("1", text: $model.quantity)
                        #if os(iOS)
                        .keyboardType(.numberPad)
                        #endif
                        .multilineTextAlignment(.trailing)
                }
                LabeledContent("Value") {
                    HStack(spacing: 4) {
                        Text("$")
                            .foregroundStyle(.secondary)
                        TextField("0.00", text: $model.valueDollars)
                            #if os(iOS)
                            .keyboardType(.decimalPad)
                            #endif
                            .multilineTextAlignment(.trailing)
                    }
                }
                AcquiredDateRow(date: $model.acquiredDate)
                TextField("Serial", text: $model.serial)
                    #if os(iOS)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    #endif
                TextField("Barcode", text: $model.barcode)
                    #if os(iOS)
                    .keyboardType(.numbersAndPunctuation)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    #endif
            }

            Section("Notes") {
                TextField("Notes", text: $model.notes, axis: .vertical)
                    .lineLimit(3...8)
            }

            Section("Tags") {
                TagEditor(
                    tags: model.tags,
                    newTag: $model.newTag,
                    onAdd: { model.addTag() },
                    onRemove: { model.removeTag($0) }
                )
            }

            if let error = model.errorMessage {
                Section {
                    Text(error)
                        .foregroundStyle(.red)
                }
            }

            Section {
                Button("Save") { model.save() }
                    .disabled(model.working)
            }
        }
    }
}

/// The acquired-date row: a native date picker plus a Clear button so the date can
/// be set to "none". SwiftUI's `DatePicker` has no empty state, so the row shows
/// the picker only when a date is set; an "Add date" button seeds today when
/// there is none, and Clear removes it (mapping back to `None` on save).
private struct AcquiredDateRow: View {
    @Binding var date: Date?

    var body: some View {
        if let bound = Binding($date) {
            DatePicker("Acquired", selection: bound, displayedComponents: .date)
            Button("Clear date", role: .destructive) { date = nil }
        } else {
            Button("Add acquired date") { date = Date() }
        }
    }
}

/// The tag editor: a wrapping list of removable chips plus a field to add one.
private struct TagEditor: View {
    let tags: [String]
    @Binding var newTag: String
    let onAdd: () -> Void
    let onRemove: (String) -> Void

    private var trimmedNew: String {
        newTag.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    var body: some View {
        if !tags.isEmpty {
            TagChips(tags: tags, onRemove: onRemove)
        }
        HStack {
            TextField("Add a tag", text: $newTag)
                #if os(iOS)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                #endif
                .submitLabel(.done)
                .onSubmit { if !trimmedNew.isEmpty { onAdd() } }
            Button("Add", action: onAdd)
                .disabled(trimmedNew.isEmpty)
        }
    }
}

/// A wrapping row of tag chips, each with a remove control.
private struct TagChips: View {
    let tags: [String]
    let onRemove: (String) -> Void

    private let columns = [GridItem(.adaptive(minimum: 80), spacing: 8, alignment: .leading)]

    var body: some View {
        LazyVGrid(columns: columns, alignment: .leading, spacing: 8) {
            ForEach(tags, id: \.self) { tag in
                HStack(spacing: 4) {
                    Text(tag)
                        .lineLimit(1)
                    Button {
                        onRemove(tag)
                    } label: {
                        Image(systemName: "xmark.circle.fill")
                            .foregroundStyle(.secondary)
                    }
                    .buttonStyle(.plain)
                }
                .padding(.horizontal, 10)
                .padding(.vertical, 6)
                .background(Theme.placeholder)
                .clipShape(Capsule())
            }
        }
    }
}
