import os.log
#if os(iOS)
import UIKit
#else
import AppKit
#endif

/// Copy and share affordances for the sharing codes. Copy writes to the system
/// pasteboard on both platforms; share presents the system share sheet on iOS.
/// macOS has no separate share action — the codes are copy/paste text, so Copy
/// is the only affordance there.
@MainActor
enum ShareActions {
    /// Write `text` to the system pasteboard.
    static func copy(_ text: String) {
        #if os(iOS)
        UIPasteboard.general.string = text
        #else
        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.setString(text, forType: .string)
        #endif
    }

    #if os(iOS)
    /// Present the system share sheet for `text` from the active window scene's
    /// root view controller.
    static func share(_ text: String) {
        let scenes = UIApplication.shared.connectedScenes
        guard let scene = scenes.first(where: { $0.activationState == .foregroundActive }) as? UIWindowScene,
              let root = scene.keyWindow?.rootViewController else {
            // No active window to anchor the sheet — copy so the action still
            // gives the user the code.
            Logger.visible("ShareActions").warning("no foreground window to anchor the share sheet; copied instead")
            copy(text)
            return
        }
        let controller = UIActivityViewController(activityItems: [text], applicationActivities: nil)
        // iPad requires a popover anchor; center it on the presenting view.
        controller.popoverPresentationController?.sourceView = root.view
        controller.popoverPresentationController?.sourceRect = CGRect(
            x: root.view.bounds.midX,
            y: root.view.bounds.midY,
            width: 0,
            height: 0
        )
        var presenter = root
        while let presented = presenter.presentedViewController {
            presenter = presented
        }
        presenter.present(controller, animated: true)
    }
    #endif
}
