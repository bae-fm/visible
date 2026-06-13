import SwiftUI
import os.log

private let logger = Logger.visible("NodeImageView")

#if os(macOS)
typealias PlatformImage = NSImage
#else
typealias PlatformImage = UIImage
#endif

/// A node's photo at `path`, clipped to a rounded square, or a neutral
/// placeholder when `path` is nil (no image, or its file is missing). The path
/// comes straight from the bridge's `imagePathIfExists`; replacing a photo mints
/// a new image id and thus a new path, so the image reloads on its own.
struct NodeImageView: View {
    let path: String?
    var cornerRadius: CGFloat = 0

    @State private var image: PlatformImage?

    var body: some View {
        ZStack {
            if let image {
                platformImage(image)
                    .resizable()
                    .scaledToFill()
            } else {
                Theme.placeholder
                Image(systemName: "photo")
                    .resizable()
                    .scaledToFit()
                    .frame(width: 40, height: 40)
                    .foregroundStyle(Theme.placeholderIcon)
            }
        }
        .clipShape(RoundedRectangle(cornerRadius: cornerRadius))
        .contentShape(Rectangle())
        .task(id: path) {
            await load()
        }
    }

    private func platformImage(_ image: PlatformImage) -> Image {
        #if os(macOS)
        Image(nsImage: image)
        #else
        Image(uiImage: image)
        #endif
    }

    private func load() async {
        guard let path else {
            image = nil
            return
        }
        let decoded = await Task.detached(priority: .userInitiated) {
            PlatformImage(contentsOfFile: path)
        }.value
        if Task.isCancelled { return }
        if decoded == nil {
            // The path came from `imagePathIfExists`, so the file was present;
            // a nil decode means its bytes aren't a valid image.
            logger.warning("decoding image at \(path, privacy: .public) failed; showing placeholder")
        }
        image = decoded
    }
}
