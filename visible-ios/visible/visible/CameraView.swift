import SwiftUI
import os.log

private let logger = Logger.visible("CameraView")

#if os(iOS)
import UIKit

/// The device camera, wrapping `UIImagePickerController` with `sourceType =
/// .camera`. On capture it hands the photo's JPEG bytes to `onCaptured`; on
/// cancel it calls `onCancel`. The system prompts for camera permission the
/// first time the picker opens.
struct CameraView: UIViewControllerRepresentable {
    let onCaptured: (Data) -> Void
    let onCancel: () -> Void

    /// Whether this device has a camera the picker can use (false on the
    /// simulator and camera-less devices).
    static var isAvailable: Bool {
        UIImagePickerController.isSourceTypeAvailable(.camera)
    }

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
#else
import AVFoundation
import AppKit

/// The Mac's camera. macOS has no `UIImagePickerController`, so this drives an
/// `AVCaptureSession` directly: a live preview from the default video device,
/// with Capture and Cancel buttons over it. On capture it hands the photo's
/// JPEG bytes to `onCaptured`; on cancel (or any failure) it calls `onCancel`.
/// The same `onCaptured: (Data) -> Void` / `onCancel: () -> Void` API as the
/// iOS picker, so `BrowseView` presents it identically on both platforms.
struct CameraView: View {
    let onCaptured: (Data) -> Void
    let onCancel: () -> Void

    /// Whether the Mac has a video capture device.
    static var isAvailable: Bool {
        AVCaptureDevice.default(for: .video) != nil
    }

    @State private var capture = CameraCapture()

    var body: some View {
        ZStack(alignment: .bottom) {
            CameraPreview(session: capture.session)
            HStack(spacing: 24) {
                Button("Cancel", action: onCancel)
                    .keyboardShortcut(.cancelAction)
                Button("Capture") {
                    capture.capturePhoto(onCaptured: onCaptured, onCancel: onCancel)
                }
                .keyboardShortcut(.defaultAction)
            }
            .padding()
        }
        .frame(minWidth: 480, minHeight: 360)
        .task {
            await capture.start(onCaptured: onCaptured, onCancel: onCancel)
        }
        .onDisappear { capture.stop() }
    }
}

/// Owns the `AVCaptureSession`: requests camera access, wires the default video
/// input to a photo output, runs the session off the main thread, and delivers
/// the captured photo's JPEG bytes through the photo-capture delegate.
///
/// `AVCaptureSession` and `AVCapturePhotoOutput` are documented thread-safe but
/// not `Sendable`, so all of their mutating calls (configure, start, stop,
/// capture) run on `sessionQueue` and reach it through a `nonisolated(unsafe)`
/// capture — the queue serializes access, satisfying the safety the type can't
/// express. Only `session` (which the preview layer reads on the main thread,
/// also allowed) is exposed.
@MainActor
@Observable
private final class CameraCapture {
    let session = AVCaptureSession()

    @ObservationIgnored private let output = AVCapturePhotoOutput()
    @ObservationIgnored private let sessionQueue = DispatchQueue(label: "fm.bae.visible.camera")
    @ObservationIgnored private var delegate: PhotoCaptureDelegate?

    /// Requests camera access, configures the session, and starts it. On a
    /// denied permission or a missing/unusable device it logs and calls
    /// `onCancel` without starting.
    func start(onCaptured: @escaping (Data) -> Void, onCancel: @escaping () -> Void) async {
        guard await AVCaptureDevice.requestAccess(for: .video) else {
            logger.warning("camera access denied; dismissing capture")
            onCancel()
            return
        }
        guard let device = AVCaptureDevice.default(for: .video),
              let input = try? AVCaptureDeviceInput(device: device)
        else {
            logger.warning("no usable camera device; dismissing capture")
            onCancel()
            return
        }

        nonisolated(unsafe) let session = session
        nonisolated(unsafe) let output = output
        nonisolated(unsafe) let deviceInput = input
        sessionQueue.async {
            session.beginConfiguration()
            if session.canAddInput(deviceInput) { session.addInput(deviceInput) }
            if session.canAddOutput(output) { session.addOutput(output) }
            session.commitConfiguration()
            session.startRunning()
        }
    }

    func capturePhoto(onCaptured: @escaping (Data) -> Void, onCancel: @escaping () -> Void) {
        // The delegate runs on the session queue and reports the JPEG bytes, or
        // nil when the capture failed (it logs the cause itself). Hop to the main
        // actor to call back into the UI.
        let delegate = PhotoCaptureDelegate { jpeg in
            Task { @MainActor in
                if let jpeg { onCaptured(jpeg) } else { onCancel() }
            }
        }
        self.delegate = delegate

        nonisolated(unsafe) let output = output
        sessionQueue.async {
            output.capturePhoto(with: AVCapturePhotoSettings(), delegate: delegate)
        }
    }

    func stop() {
        nonisolated(unsafe) let session = session
        sessionQueue.async {
            if session.isRunning { session.stopRunning() }
        }
    }
}

/// The photo-capture callback target. `AVCapturePhotoOutput` holds the delegate
/// weakly, so `CameraCapture` keeps it alive for the duration of the capture.
/// Reports the captured JPEG bytes, or nil when the capture failed (logging the
/// cause) so the caller can dismiss. `@unchecked Sendable` so it can ride the
/// session queue: AVFoundation calls `photoOutput` once on its own queue, and
/// `completion` only reaches the main-actor callbacks through a `Task`.
private final class PhotoCaptureDelegate: NSObject, AVCapturePhotoCaptureDelegate, @unchecked Sendable {
    private let completion: (Data?) -> Void

    init(completion: @escaping (Data?) -> Void) {
        self.completion = completion
    }

    func photoOutput(
        _: AVCapturePhotoOutput,
        didFinishProcessingPhoto photo: AVCapturePhoto,
        error: Error?
    ) {
        if let error {
            logger.warning("photo capture failed: \(error.localizedDescription, privacy: .public); dismissing capture")
            completion(nil)
            return
        }
        guard let data = photo.fileDataRepresentation() else {
            logger.warning("captured photo had no data representation; dismissing capture")
            completion(nil)
            return
        }
        completion(data)
    }
}

/// The live camera preview: an `NSView` backed by an
/// `AVCaptureVideoPreviewLayer` showing the session's video.
private struct CameraPreview: NSViewRepresentable {
    let session: AVCaptureSession

    func makeNSView(context _: Context) -> PreviewView {
        let view = PreviewView()
        view.previewLayer.session = session
        view.previewLayer.videoGravity = .resizeAspectFill
        return view
    }

    func updateNSView(_ view: PreviewView, context _: Context) {
        view.previewLayer.session = session
    }

    final class PreviewView: NSView {
        let previewLayer = AVCaptureVideoPreviewLayer()

        override init(frame: NSRect) {
            super.init(frame: frame)
            wantsLayer = true
            layer = previewLayer
        }

        @available(*, unavailable)
        required init?(coder _: NSCoder) { fatalError("not used") }
    }
}
#endif
