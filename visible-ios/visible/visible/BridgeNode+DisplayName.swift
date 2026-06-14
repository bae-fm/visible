extension BridgeNode {
    /// The title to show for a node. An untitled node (name = nil, e.g. a
    /// photo-first child not yet renamed) shows "Untitled".
    var displayName: String { name ?? "Untitled" }
}
