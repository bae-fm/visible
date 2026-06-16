import SwiftUI

/// The visible brand mark: an eye glyph in an accent rounded square beside the
/// "visible" wordmark (the "visi" stem tinted in the accent), with the
/// "See what you own." tagline. Shown atop the welcome screen. Shared by iOS and
/// macOS.
struct Brand: View {
    var body: some View {
        VStack(spacing: 12) {
            HStack(spacing: 12) {
                RoundedRectangle(cornerRadius: 11, style: .continuous)
                    .fill(Theme.accent)
                    .frame(width: 44, height: 44)
                    .overlay {
                        Image(systemName: "eye")
                            .font(.system(size: 24))
                            .foregroundStyle(Theme.onAccent)
                    }
                (Text("visi").foregroundStyle(Theme.accent) + Text("ble"))
                    .font(.system(size: 34, weight: .bold))
                    .tracking(-1)
            }
            Text("See what you own.")
                .font(.callout)
                .foregroundStyle(.secondary)
        }
    }
}
