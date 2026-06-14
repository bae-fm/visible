import Foundation
import os.log

private let logger = Logger.visible("ImagePath")

/// The local file path for `imageId` if its file exists, else nil. The bridge
/// call does no database work (a filesystem existence check), so the image views
/// call it directly on the render path. A node whose image file isn't on disk
/// renders the placeholder. Shared by the browse and search models so the
/// existence check and the missing-image log live in one place.
func imagePath(_ handle: AppHandle, _ imageId: String) -> String? {
    let path = handle.imagePathIfExists(imageId: imageId)
    if path == nil {
        logger.debug("no image file for \(imageId, privacy: .public); showing placeholder")
    }
    return path
}
