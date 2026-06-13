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
}
