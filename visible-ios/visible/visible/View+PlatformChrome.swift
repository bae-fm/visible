import SwiftUI

/// Platform presentation chrome shared by the sheets and the browse screen. Each
/// modifier carries the `#if os(iOS)` branch so the call sites read as one
/// intent instead of repeating the platform guard.
extension View {
    /// A navigation title that displays inline on iOS. macOS has no inline title
    /// mode, so the title is applied unchanged there.
    func inlineNavigationTitle(_ title: String) -> some View {
        navigationTitle(title)
            #if os(iOS)
            .navigationBarTitleDisplayMode(.inline)
            #endif
    }

    /// Sheet sizing: a medium detent on iOS, a fixed minimum frame on macOS
    /// (which has no detents and would otherwise size the sheet to its content).
    func sheetChrome() -> some View {
        #if os(iOS)
        presentationDetents([.medium])
        #else
        frame(minWidth: 360, minHeight: 160)
        #endif
    }

    /// Presents the platform's image import while `isPresented` is true, feeding
    /// the chosen image's bytes to `onPicked` (or `onCancel` on dismissal/failure)
    /// and clearing the binding. iOS presents `PhotoLibraryPicker` (PHPicker) in a
    /// sheet; macOS, whose `NSOpenPanel` is application-modal, runs the panel
    /// directly when the binding flips true — the sanctioned platform-mechanism
    /// difference for the same "pick an image" intent. The closures are
    /// `@MainActor @Sendable` so the iOS picker's background load callback can hop
    /// the result back to the main actor.
    func photoLibraryImport(
        isPresented: Binding<Bool>,
        onPicked: @escaping @MainActor @Sendable (Data) -> Void,
        onCancel: @escaping @MainActor @Sendable () -> Void
    ) -> some View {
        #if os(iOS)
        return sheet(isPresented: isPresented) {
            PhotoLibraryPicker(onPicked: onPicked, onCancel: onCancel)
                .ignoresSafeArea()
        }
        #else
        // The panel is modal, so there's no sheet to present; react to the flag
        // by running it once, then clear the flag.
        return onChange(of: isPresented.wrappedValue) { _, presenting in
            guard presenting else { return }
            isPresented.wrappedValue = false
            runImagePanel(onPicked: onPicked, onCancel: onCancel)
        }
        #endif
    }

    /// Presents a node's photo full-screen for the path bound to `path`,
    /// dismissed by clearing the binding. iOS uses a `fullScreenCover`; macOS,
    /// which has no full-screen cover, uses a large sheet — the same "view the
    /// photo big" intent through each platform's mechanism.
    func fullScreenImageCover(path: Binding<String?>) -> some View {
        let dismiss = { path.wrappedValue = nil }
        return Group {
            #if os(iOS)
            fullScreenCover(item: PhotoPath.binding(path)) { item in
                FullScreenImageView(path: item.path, onDismiss: dismiss)
            }
            #else
            sheet(item: PhotoPath.binding(path)) { item in
                FullScreenImageView(path: item.path, onDismiss: dismiss)
            }
            #endif
        }
    }
}

/// Wraps a photo's file path so it drives `sheet`/`fullScreenCover(item:)`, which
/// need an `Identifiable`. The path is both the photo to show and the identity.
private struct PhotoPath: Identifiable {
    let path: String
    var id: String { path }

    /// Bridges an optional-path binding to the optional-item binding the
    /// item-based presenters take: present when a path is set, dismiss (clear the
    /// path) when the item is cleared.
    static func binding(_ path: Binding<String?>) -> Binding<PhotoPath?> {
        Binding(
            get: { path.wrappedValue.map(PhotoPath.init) },
            set: { path.wrappedValue = $0?.path }
        )
    }
}
