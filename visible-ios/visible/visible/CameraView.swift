import SwiftUI
import UIKit
import os.log

private let logger = Logger.visible("CameraView")

/// The device camera, wrapping `UIImagePickerController` with `sourceType =
/// .camera`. On capture it hands the photo's JPEG bytes to `onCaptured`; on
/// cancel it calls `onCancel`. The system prompts for camera permission the
/// first time the picker opens.
struct CameraView: UIViewControllerRepresentable {
    let onCaptured: (Data) -> Void
    let onCancel: () -> Void

    func makeUIViewController(context: Context) -> UIImagePickerController {
        let picker = UIImagePickerController()
        picker.sourceType = .camera
        picker.delegate = context.coordinator
        return picker
    }

    func updateUIViewController(_: UIImagePickerController, context _: Context) {}

    func makeCoordinator() -> Coordinator {
        Coordinator(onCaptured: onCaptured, onCancel: onCancel)
    }

    final class Coordinator: NSObject, UIImagePickerControllerDelegate, UINavigationControllerDelegate {
        private let onCaptured: (Data) -> Void
        private let onCancel: () -> Void

        init(onCaptured: @escaping (Data) -> Void, onCancel: @escaping () -> Void) {
            self.onCaptured = onCaptured
            self.onCancel = onCancel
        }

        func imagePickerController(
            _: UIImagePickerController,
            didFinishPickingMediaWithInfo info: [UIImagePickerController.InfoKey: Any]
        ) {
            guard let image = info[.originalImage] as? UIImage else {
                logger.warning("camera picker returned no image; dismissing capture")
                onCancel()
                return
            }
            guard let jpeg = image.jpegData(compressionQuality: 0.85) else {
                logger.warning("jpegData conversion of the captured image failed; dismissing capture")
                onCancel()
                return
            }
            onCaptured(jpeg)
        }

        func imagePickerControllerDidCancel(_: UIImagePickerController) {
            onCancel()
        }
    }
}
