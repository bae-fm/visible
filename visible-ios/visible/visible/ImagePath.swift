import Foundation
import os.log

private let logger = Logger.visible("ImagePath")

/// Resolving a node's image id to a local file path. Shared by the browse,
/// search, and move-picker models so the existence check and the missing-image
/// log live in one place.
enum ImagePath {
    /// The local file path for `imageId` if its file exists, else nil. The bridge
    /// call does no database work (a filesystem existence check), so the image
    /// views call it directly on the render path. A node whose image file isn't
    /// on disk renders the placeholder.
    static func resolve(_ handle: AppHandle, _ imageId: String) -> String? {
        let path = handle.imagePathIfExists(imageId: imageId)
        if path == nil {
            logger.debug("no image file for \(imageId, privacy: .public); showing placeholder")
        }
        return path
    }
}
