package fm.bae.visible.ui

import uniffi.visible_bridge.BridgeNode

/**
 * The title to show for a node. An untitled node (name = null, e.g. a
 * photo-first child not yet renamed) shows "Untitled".
 */
val BridgeNode.displayName: String
    get() = name ?: "Untitled"
