//! visible's [`coven::blob::BlobPlan`]: which rows carry image blobs, where their
//! plaintext lives locally, and how each is scoped for encryption.
//!
//! coven owns the cloud blob layout (`images/{ab}/{cd}/{id}`) and encryption;
//! visible decides that a `node_images` row is one image blob, located on disk at
//! [`LibraryDir::image_path`], encrypted with the library master key.
//!
//! `node_images` is immutable: a row is inserted when an image is added and
//! deleted when it is removed or replaced, never updated (see [`crate::node`]).
//! That is why the changeset channel carries images correctly. coven uploads a
//! blob for every INSERT it pushes and downloads one for every INSERT it pulls,
//! keyed by the row's primary key — which is the image id. Because replacing a
//! photo is "DELETE the old row, INSERT a new one" rather than an UPDATE, the new
//! image always rides a fresh INSERT to an already-synced peer; there is no
//! UPDATE whose changeset would expose only the old (replaced) value. Deletes
//! carry no blob to move — the cloud blob delete is enqueued separately through
//! coven's cloud outbox.

use coven::blob::{BlobPlan, BlobRef, BlobScope};
use coven::changeset::{ChangeOp, RowChange};
use coven::library_dir::LibraryDir;
use coven::rusqlite::Connection;
use tracing::warn;

/// The synced table whose rows are image blobs (one row per image, primary key =
/// image id).
const NODE_IMAGES_TABLE: &str = "node_images";

/// The cloud namespace image blobs live under (`images/{ab}/{cd}/{image_id}`).
/// The single home for this name: the push/pull path here and the outbox
/// delete-key path in [`crate::node`] both use it, so the changeset upload and
/// the delete name the same cloud object.
pub const IMAGE_NAMESPACE: &str = "images";

/// Maps visible's `node_images` rows to their cloud blobs.
pub struct NodeBlobPlan {
    dir: LibraryDir,
}

impl NodeBlobPlan {
    pub fn new(dir: LibraryDir) -> Self {
        Self { dir }
    }

    /// One image blob, scoped to the library master key (every member reads it).
    /// The cloud key is `images/{ab}/{cd}/{image_id}`; the local plaintext is the
    /// content-addressed file at [`LibraryDir::image_path`].
    fn image_ref(&self, image_id: &str) -> BlobRef {
        BlobRef {
            namespace: IMAGE_NAMESPACE.to_string(),
            id: image_id.to_string(),
            local_path: self.dir.image_path(image_id),
            scope: BlobScope::Master,
        }
    }

    /// The image blobs an INSERT changeset references. A `node_images` row's
    /// primary key is the image id, and the table is immutable, so a row's blob is
    /// always carried by the INSERT that adds it — the same INSERT whether the
    /// image is a node's first photo or a replacement. UPDATEs never occur on this
    /// table; a DELETE carries no blob to move (its cloud blob delete is enqueued
    /// separately).
    fn inserted_image_refs(&self, changes: &[RowChange]) -> Vec<BlobRef> {
        let mut refs = Vec::new();
        for change in changes {
            if change.table != NODE_IMAGES_TABLE || change.op != ChangeOp::Insert {
                continue;
            }
            match change.pk() {
                Some(image_id) => refs.push(self.image_ref(image_id)),
                None => warn!("node_images INSERT has no primary key; skipping its blob"),
            }
        }
        refs
    }
}

impl BlobPlan for NodeBlobPlan {
    fn blobs_to_push(&self, changes: &[RowChange]) -> Vec<BlobRef> {
        self.inserted_image_refs(changes)
    }

    fn blobs_to_pull(&self, changes: &[RowChange]) -> Vec<BlobRef> {
        self.inserted_image_refs(changes)
    }

    /// Every image currently in the DB, for the snapshot-bootstrap backfill. A
    /// device that bootstraps from a snapshot receives the `node_images` rows but
    /// not their files (the snapshot carries no blobs, and the incremental pull
    /// starts past the INSERTs that first carried them), so coven downloads the
    /// missing ones from this list. Every image encrypts with the master key,
    /// exactly as the changeset path scopes it.
    fn blobs_in_db(&self, conn: &Connection) -> coven::rusqlite::Result<Vec<BlobRef>> {
        let mut stmt = conn.prepare("SELECT id FROM node_images")?;
        let image_ids = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<coven::rusqlite::Result<Vec<_>>>()?;
        if let Err(e) = stmt.finalize() {
            warn!("finalizing blobs_in_db statement: {e}");
        }
        Ok(image_ids.iter().map(|id| self.image_ref(id)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan() -> NodeBlobPlan {
        NodeBlobPlan::new(LibraryDir::new("/lib"))
    }

    /// A `node_images` changeset row in schema order: id, node_id, _updated_at.
    /// The value chosen per column follows the op's old/new semantics, which the
    /// caller can't override — but the primary key (id) is present for every op.
    fn node_image_row(op: ChangeOp, image_id: &str) -> RowChange {
        RowChange {
            table: NODE_IMAGES_TABLE.to_string(),
            op,
            columns: vec![
                Some(image_id.to_string()),
                Some("node-1".to_string()),
                Some("stamp".to_string()),
            ],
        }
    }

    #[test]
    fn insert_yields_a_master_scoped_blob_keyed_by_image_id() {
        let refs = plan().blobs_to_push(&[node_image_row(ChangeOp::Insert, "img-1")]);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].namespace, "images");
        // Keyed by the node_images primary key, which is the image id.
        assert_eq!(refs[0].id, "img-1");
        assert_eq!(refs[0].scope, BlobScope::Master);
        assert_eq!(
            refs[0].local_path,
            LibraryDir::new("/lib").image_path("img-1")
        );
    }

    #[test]
    fn deletes_carry_no_changeset_blob() {
        // A DELETE moves no changeset blob; its cloud blob removal is enqueued
        // separately through the outbox.
        let changes = [node_image_row(ChangeOp::Delete, "img-1")];
        assert!(plan().blobs_to_push(&changes).is_empty());
        assert!(plan().blobs_to_pull(&changes).is_empty());
    }

    #[test]
    fn pull_matches_push() {
        let changes = [node_image_row(ChangeOp::Insert, "img-1")];
        let push = plan().blobs_to_push(&changes);
        let pull = plan().blobs_to_pull(&changes);
        assert_eq!(push.len(), pull.len());
        assert_eq!(push[0].id, pull[0].id);
    }

    #[test]
    fn other_tables_carry_no_blobs() {
        let mut row = node_image_row(ChangeOp::Insert, "img-1");
        row.table = "nodes".to_string();
        assert!(plan().blobs_to_push(&[row]).is_empty());
    }

    #[test]
    fn blobs_in_db_lists_every_image_row_with_master_scope() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE node_images (\
                 id TEXT PRIMARY KEY, node_id TEXT NOT NULL, _updated_at TEXT NOT NULL);\n\
             INSERT INTO node_images VALUES ('img-2', 'n2', 's');\n\
             INSERT INTO node_images VALUES ('img-3', 'n3', 's');",
        )
        .unwrap();

        let refs = plan().blobs_in_db(&conn).unwrap();
        assert_eq!(refs.len(), 2);
        let ids: Vec<&str> = refs.iter().map(|r| r.id.as_str()).collect();
        assert!(ids.contains(&"img-2"));
        assert!(ids.contains(&"img-3"));
        for r in &refs {
            assert_eq!(r.namespace, "images");
            assert_eq!(r.scope, BlobScope::Master);
        }
    }
}
