# visible

See what you own. visible is a home-inventory app for organizing the physical
things in your house as a hierarchy you can browse on your phone — your house at
the top, then rooms, then containers, then individual things, each with a photo
you take with your camera.

The name comes from *visible storage*: a museum showing the holdings normally
kept in the back. visible does that for your home — it takes the invisible (the
stuff you own, scattered and forgotten) and makes it visible.

## How it works

Everything is a *node* in a tree. The root is your house; its children are
rooms; their children are shelves, drawers, boxes, and the things inside them.
Any node can hold other nodes, to any depth, and any node can carry a photo. You
browse down into a node to see what it contains and back up to where it sits.

Storage runs on [coven](https://github.com/bae-fm/coven): an
end-to-end-encrypted, multi-writer SQLite sync layer over bring-your-own cloud
storage. v1 keeps everything on the device; the schema is coven's sync-ready
shape, so sharing a household inventory with the people you live with comes next.

## Layout

| Component | Description |
|-----------|-------------|
| `visible-core` | Rust: the node-tree domain, local image storage, library lifecycle. Owns the SQLite database through coven. |
| `visible-bridge` | UniFFI bridge — translates `visible-core` types to Swift and Kotlin. Type translation only. |
| `visible-android` | Native Android app (Jetpack Compose). |
| `visible-ios` | Native iOS app (SwiftUI). |

## Status

Pre-1.0.
