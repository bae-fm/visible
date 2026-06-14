import SwiftUI
import os.log

private let logger = Logger.visible("PhotoLibraryPicker")

#if os(iOS)
import PhotosUI
import UIKit

/// The photo library, wrapping `PHPickerViewController` for a single image. On
/// pick it loads the item as a `UIImage`, re-encodes it as JPEG, and hands the
/// bytes to `onPicked`; on cancel (or a load/encode failure) it calls `onCancel`.
/// PHPicker runs out of process and needs no photo-library permission. The same
/// `onPicked: (Data) -> Void` / `onCancel: () -> Void` API as `CameraView`, so
/// `BrowseView` presents either as a sheet feeding the same `setNodeImage` path.
struct PhotoLibraryPicker: UIViewControllerRepresentable {
    let onPicked: (Data) -> Void
    let onCancel: () -> Void

    func makeUIViewController(context: Context) -> PHPickerViewController {
        var config = PHPickerConfiguration()
        config.filter = .images
        config.selectionLimit = 1
        let picker = PHPickerViewController(configuration: config)
        picker.delegate = context.coordinator
        return picker
    }

    func updateUIViewController(_: PHPickerViewController, context _: Context) {}

    func makeCoordinator() -> Coordinator {
        Coordinator(onPicked: onPicked, onCancel: onCancel)
    }

    final class Coordinator: NSObject, PHPickerViewControllerDelegate {
        private let onPicked: (Data) -> Void
        private let onCancel: () -> Void

        init(onPicked: @escaping (Data) -> Void, onCancel: @escaping () -> Void) {
            self.onPicked = onPicked
            self.onCancel = onCancel
        }

        func picker(_: PHPickerViewController, didFinishPicking results: [PHPickerResult]) {
            guard let provider = results.first?.itemProvider else {
                // No selection (the user tapped Cancel).
                onCancel()
                return
            }
            guard provider.canLoadObject(ofClass: UIImage.self) else {
                logger.warning("picked item can't load as an image; dismissing import")
                onCancel()
                return
            }
            provider.loadObject(ofClass: UIImage.self) { [onPicked, onCancel] object, error in
                // The provider calls back on its own queue. Re-encode to JPEG
                // here (`object` is task-isolated, so it can't cross to the main
                // actor) and hop only the resulting `Data` to feed the UI's
                // import path.
                let jpeg: Data?
                if let error {
                    logger.warning("loading the picked image failed: \(error.localizedDescription, privacy: .public); dismissing import")
                    jpeg = nil
                } else if let image = object as? UIImage {
                    jpeg = image.jpegData(compressionQuality: 0.85)
                    if jpeg == nil {
                        logger.warning("jpegData conversion of the picked image failed; dismissing import")
                    }
                } else {
                    logger.warning("picked item loaded as no image; dismissing import")
                    jpeg = nil
                }
                Task { @MainActor in
                    if let jpeg { onPicked(jpeg) } else { onCancel() }
                }
            }
        }
    }
}
#else
import AppKit
import UniformTypeIdentifiers

/// The Mac's image import. macOS has no photo library, so this runs an
/// `NSOpenPanel` restricted to image files: on a chosen file it reads the bytes
/// and hands them to `onPicked`; on cancel (or a read failure) it calls
/// `onCancel`. The same `onPicked: (Data) -> Void` / `onCancel: () -> Void` API
/// as the iOS picker feeds the same `setNodeImage` path. The bytes are the
/// file's own (no re-encode): core stores whatever image bytes it is given, and
/// a sandbox-granted read of a user-selected file is the equivalent of the
/// library pick. The panel is application-modal, so it runs directly rather than
/// inside a SwiftUI sheet (which would leave an empty host on screen).
@MainActor
func runImagePanel(onPicked: (Data) -> Void, onCancel: () -> Void) {
    let panel = NSOpenPanel()
    panel.allowsMultipleSelection = false
    panel.canChooseDirectories = false
    panel.canChooseFiles = true
    panel.allowedContentTypes = [.image]
    guard panel.runModal() == .OK, let url = panel.url else {
        onCancel()
        return
    }
    do {
        let data = try Data(contentsOf: url)
        onPicked(data)
    } catch {
        logger.warning("reading the chosen image at \(url.path, privacy: .public) failed: \(error.localizedDescription, privacy: .public); dismissing import")
        onCancel()
    }
}
#endif
