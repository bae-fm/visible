import SwiftUI
import os.log

private let logger = Logger.visible("FullScreenImageView")

/// A node's photo shown filling the screen, dismissed by a tap or a Done
/// control. Loads the image at `path` off the main thread, the same decode the
/// thumbnail uses; a file that won't decode shows a message rather than a blank
/// screen (the path came from `imagePathIfExists`, so the file was present).
///
/// iOS presents this in a `fullScreenCover`; macOS, which has no full-screen
/// cover, presents it in a sheet sized large — the sanctioned platform-mechanism
/// difference for the same "view the photo big" intent.
struct FullScreenImageView: View {
    let path: String
    let onDismiss: () -> Void

    @State private var image: PlatformImage?

    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()

            if let image {
                platformImage(image)
                    .resizable()
                    .scaledToFit()
            } else {
                Text("This photo couldn't be loaded.")
                    .foregroundStyle(.white)
                    .multilineTextAlignment(.center)
                    .padding(24)
            }
        }
        .contentShape(Rectangle())
        .onTapGesture { onDismiss() }
        .overlay(alignment: .topTrailing) {
            Button("Done", action: onDismiss)
                .padding()
        }
        #if os(macOS)
        .frame(minWidth: 640, minHeight: 480)
        #endif
        .task(id: path) { await load() }
    }

    private func platformImage(_ image: PlatformImage) -> Image {
        #if os(macOS)
        Image(nsImage: image)
        #else
        Image(uiImage: image)
        #endif
    }

    private func load() async {
        let decoded = await Task.detached(priority: .userInitiated) {
            PlatformImage(contentsOfFile: path)
        }.value
        if Task.isCancelled { return }
        if decoded == nil {
            // The path came from `imagePathIfExists`, so the file was present; a
            // nil decode means its bytes aren't a valid image.
            logger.warning("decoding image at \(path, privacy: .public) failed; showing the unloadable-photo message")
        }
        image = decoded
    }
}
