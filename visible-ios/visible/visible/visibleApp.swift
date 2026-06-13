import SwiftUI

@main
struct visibleApp: App {
    private let session = AppSession()

    var body: some Scene {
        WindowGroup {
            AppRootView(session: session)
        }
    }
}
