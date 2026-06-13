import SwiftUI

/// The app palette, mirroring the Android `VisibleTheme` teal scheme. The
/// accent tints controls; the placeholder fills a node's photo area when it has
/// no image.
enum Theme {
    static let accent = Color(
        light: Color(red: 0.220, green: 0.416, blue: 0.416),
        dark: Color(red: 0.627, green: 0.812, blue: 0.808)
    )

    /// The neutral fill behind a missing photo (Android `surfaceVariant`).
    static let placeholder = Color(
        light: Color(red: 0.855, green: 0.855, blue: 0.855),
        dark: Color(red: 0.251, green: 0.282, blue: 0.298)
    )

    /// The tint of the placeholder's image glyph (Android `onSurfaceVariant`).
    static let placeholderIcon = Color(
        light: Color(red: 0.255, green: 0.282, blue: 0.298),
        dark: Color(red: 0.753, green: 0.784, blue: 0.800)
    )
}

private extension Color {
    /// A color that resolves to `light` or `dark` by the active interface style.
    init(light: Color, dark: Color) {
        self.init(uiColor: UIColor { traits in
            traits.userInterfaceStyle == .dark
                ? UIColor(dark) : UIColor(light)
        })
    }
}
