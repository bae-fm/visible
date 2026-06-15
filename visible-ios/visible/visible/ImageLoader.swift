import SwiftUI
import os.log

private let logger = Logger.visible("ImageLoader")

#if os(macOS)
typealias PlatformImage = NSImage
#else
typealias PlatformImage = UIImage
#endif

/// Decoding a node's image file into a SwiftUI `Image`, shared by the thumbnail
/// (``NodeImageView``) and the full-screen viewer (``FullScreenImageView``).
/// Both load the same way: decode off the main thread, treat a nil decode as a
/// real failure (the path came from `imagePathIfExists`, so the file was
/// present), and wrap the platform image for SwiftUI.
enum ImageLoader {
    /// Decode the image file at `path` off the main thread. Returns nil when the
    /// bytes aren't a valid image — logged, since the file was present, so a nil
    /// decode means the bytes themselves are bad.
    static func decode(path: String) async -> PlatformImage? {
        let decoded = await Task.detached(priority: .userInitiated) {
            PlatformImage(contentsOfFile: path)
        }.value
        if decoded == nil {
            logger.warning("decoding image at \(path, privacy: .public) failed")
        }
        return decoded
    }

    /// Wrap a decoded platform image as a SwiftUI `Image`.
    static func image(_ image: PlatformImage) -> Image {
        #if os(macOS)
        Image(nsImage: image)
        #else
        Image(uiImage: image)
        #endif
    }
}
