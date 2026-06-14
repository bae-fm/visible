//! visible's [`coven::blob::BlobPlan`]: which node rows carry image blobs, where
//! their plaintext lives locally, and how each is scoped for encryption.
//!
//! coven owns the cloud blob layout (`images/{ab}/{cd}/{id}`) and encryption;
//! visible decides that a node's image file is its blob, located on disk at
//! [`LibraryDir::image_path`], encrypted with the library master key.
//!
//! ## The two blob channels and why uploads go through the outbox
//!
//! coven moves a blob two ways: the *changeset channel* (this plan's
//! `blobs_to_push`/`blobs_to_pull`, walking a changeset's [`RowChange`]s) and the
//! *outbox* ([`coven::Database::enqueue_upload_on`], drained per cycle). visible
//! uploads through the outbox (see [`crate::node`]) because the changeset channel
//! cannot key an image by the *new* `image_id` on an UPDATE: a captured UPDATE
//! [`RowChange`] exposes only the OLD value of a changed column
//! (`RowChange::col` collapses to the old value when present), and `set_image`
//! re-points `image_id` on an UPDATE. So:
//!
//! - `blobs_to_push`/`blobs_to_pull` recognize **INSERTs** — the only op whose
//!   `image_id` column reads as the new value — carrying a new node's photo to an
//!   online peer over the incremental pull.
//! - `blobs_in_db` enumerates every node's *current* `image_id`, so a snapshot
//!   bootstrap lands the right files and a `set_image` (UPDATE) propagates to an
//!   already-synced peer through the next snapshot.

use coven::blob::{BlobPlan, BlobRef, BlobScope};
use coven::changeset::{ChangeOp, RowChange};
use coven::library_dir::LibraryDir;
use coven::rusqlite::Connection;
use tracing::warn;

/// The synced table whose rows carry image blobs.
const NODES_TABLE: &str = "nodes";

/// Column index of `image_id` in the `nodes` schema
/// (`id, parent_id, name, position, image_id, _updated_at`). The blob id for a
/// node row is this column's value, not the row primary key (col 0): the image
/// file is content-addressed by `image_id`, which is null for an imageless node
/// and changes when the photo is replaced.
const IMAGE_ID_COL: usize = 4;

/// Maps visible's image-bearing node rows to their cloud blobs.
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
            namespace: "images".to_string(),
            id: image_id.to_string(),
            local_path: self.dir.image_path(image_id),
            scope: BlobScope::Master,
        }
    }

    /// The image blobs an INSERT changeset references. An INSERT's `image_id`
    /// column reads as the new value, so this carries a new node's photo across
    /// the changeset channel. UPDATEs are excluded: a changeset exposes only the
    /// old value of a changed column, so an UPDATE's `image_id` would name the
    /// replaced (already-deleted) file, not the new one — `set_image` uploads the
    /// new image through the outbox and propagates it to peers via the snapshot.
    /// Deletes carry no blob to move (the cloud delete is enqueued separately).
    fn inserted_image_refs(&self, changes: &[RowChange]) -> Vec<BlobRef> {
        let mut refs = Vec::new();
        for change in changes {
            if change.table != NODES_TABLE || change.op != ChangeOp::Insert {
                continue;
            }
            // An imageless node has a NULL `image_id` and carries no blob.
            if let Some(image_id) = change.col(IMAGE_ID_COL) {
                refs.push(self.image_ref(image_id));
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

    /// Every image a node row currently references, for the snapshot-bootstrap
    /// backfill. Reads the live `image_id` of each node — the current value after
    /// any `set_image` — so this is also how a photo replacement reaches an
    /// already-synced peer (the incremental UPDATE pull can't name the new image;
    /// the next snapshot can). Every image encrypts with the master key, exactly
    /// as the changeset path scopes it.
    fn blobs_in_db(&self, conn: &Connection) -> coven::rusqlite::Result<Vec<BlobRef>> {
        let mut stmt = conn.prepare("SELECT image_id FROM nodes WHERE image_id IS NOT NULL")?;
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

    /// `nodes` columns in schema order: id, parent_id, name, position, image_id,
    /// _updated_at. A changeset row carries every column; the value chosen per
    /// column follows the op's old/new semantics, which the caller can't override.
    fn node_row(op: ChangeOp, id: &str, image_id: Option<&str>) -> RowChange {
        RowChange {
            table: NODES_TABLE.to_string(),
            op,
            columns: vec![
                Some(id.to_string()),
                Some("parent".to_string()),
                Some("name".to_string()),
                Some("0".to_string()),
                image_id.map(str::to_string),
                Some("stamp".to_string()),
            ],
        }
    }

    #[test]
    fn insert_with_image_yields_a_master_scoped_blob_keyed_by_image_id() {
        let refs = plan().blobs_to_push(&[node_row(ChangeOp::Insert, "node-1", Some("img-1"))]);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].namespace, "images");
        // Keyed by the image_id column, not the node primary key.
        assert_eq!(refs[0].id, "img-1");
        assert_eq!(refs[0].scope, BlobScope::Master);
        assert_eq!(
            refs[0].local_path,
            LibraryDir::new("/lib").image_path("img-1")
        );
    }

    #[test]
    fn insert_without_image_carries_no_blob() {
        assert!(plan()
            .blobs_to_push(&[node_row(ChangeOp::Insert, "node-1", None)])
            .is_empty());
    }

    #[test]
    fn updates_and_deletes_carry_no_changeset_blob() {
        // An UPDATE's image_id column reads as the OLD (replaced) value, so the
        // changeset channel must not act on it; the new image moves via the
        // outbox. A DELETE's blob removal is a separate outbox delete.
        let changes = [
            node_row(ChangeOp::Update, "node-1", Some("img-old")),
            node_row(ChangeOp::Delete, "node-1", Some("img-1")),
        ];
        assert!(plan().blobs_to_push(&changes).is_empty());
        assert!(plan().blobs_to_pull(&changes).is_empty());
    }

    #[test]
    fn pull_matches_push() {
        let changes = [node_row(ChangeOp::Insert, "node-1", Some("img-1"))];
        let push = plan().blobs_to_push(&changes);
        let pull = plan().blobs_to_pull(&changes);
        assert_eq!(push.len(), pull.len());
        assert_eq!(push[0].id, pull[0].id);
    }

    #[test]
    fn other_tables_carry_no_blobs() {
        let mut row = node_row(ChangeOp::Insert, "x", Some("img-1"));
        row.table = "item_keys".to_string();
        assert!(plan().blobs_to_push(&[row]).is_empty());
    }

    #[test]
    fn blobs_in_db_lists_every_node_with_an_image() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE nodes (\
                 id TEXT PRIMARY KEY, parent_id TEXT, name TEXT, position INTEGER, \
                 image_id TEXT, _updated_at TEXT NOT NULL);\n\
             INSERT INTO nodes VALUES ('n1', NULL, 'house', 0, NULL, 's');\n\
             INSERT INTO nodes VALUES ('n2', 'n1', NULL, 0, 'img-2', 's');\n\
             INSERT INTO nodes VALUES ('n3', 'n1', 'box', 1, 'img-3', 's');",
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
