//! The node tree and the live, database-backed [`Inventory`] service.
//!
//! Everything the user owns is a node in one self-referential tree: the house
//! at the root (`parent_id` NULL), then rooms, containers, and things, each
//! optionally carrying one image file. A node's children are its contents.

use coven::library_dir::LibraryDir;
use coven::rusqlite::{params, Connection, OptionalExtension, Row};
use coven::{Database, UpdatedAtStamper};
use tracing::{debug, warn};

use crate::error::CoreError;

/// One node in the tree. `_updated_at` is coven's last-writer-wins register, not
/// domain data the UI reads, so it is absent here — only the live write path
/// touches it, via the stamper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub position: i64,
    pub image_id: Option<String>,
}

impl Node {
    /// Read a node from a row selecting `id, parent_id, name, position, image_id`
    /// in that order.
    fn from_row(row: &Row<'_>) -> coven::rusqlite::Result<Node> {
        Ok(Node {
            id: row.get(0)?,
            parent_id: row.get(1)?,
            name: row.get(2)?,
            position: row.get(3)?,
            image_id: row.get(4)?,
        })
    }
}

/// The columns every node read selects, in the order [`Node::from_row`] expects.
const NODE_COLUMNS: &str = "id, parent_id, name, position, image_id";

/// The live inventory for one open library: the node tree plus its image files.
/// Holds the coven database handle (the owned SQLite connection), the register
/// stamper bound into every node write, and the library directory that locates
/// image files on disk.
pub struct Inventory {
    db: Database,
    stamper: UpdatedAtStamper,
    dir: LibraryDir,
}

impl Inventory {
    pub fn new(db: Database, stamper: UpdatedAtStamper, dir: LibraryDir) -> Self {
        Self { db, stamper, dir }
    }

    /// The single top-level node (the house, `parent_id` NULL).
    pub async fn root(&self) -> Result<Node, CoreError> {
        let node = self
            .db
            .call(|conn| {
                conn.query_row(
                    &format!("SELECT {NODE_COLUMNS} FROM nodes WHERE parent_id IS NULL"),
                    [],
                    Node::from_row,
                )
                .optional()
                .map_err(Into::into)
            })
            .await?;
        node.ok_or_else(|| CoreError::NotFound("no root node".into()))
    }

    /// A node's children, ordered by sibling position.
    pub async fn children(&self, parent_id: &str) -> Result<Vec<Node>, CoreError> {
        let parent_id = parent_id.to_string();
        self.db
            .call(move |conn| {
                let mut stmt = conn.prepare(&format!(
                    "SELECT {NODE_COLUMNS} FROM nodes WHERE parent_id = ?1 ORDER BY position"
                ))?;
                let nodes = stmt
                    .query_map([parent_id], Node::from_row)?
                    .collect::<coven::rusqlite::Result<Vec<_>>>()?;
                Ok(nodes)
            })
            .await
            .map_err(Into::into)
    }

    /// A single node by id, or `None` if it doesn't exist.
    pub async fn get(&self, id: &str) -> Result<Option<Node>, CoreError> {
        let id = id.to_string();
        self.db
            .call(move |conn| {
                conn.query_row(
                    &format!("SELECT {NODE_COLUMNS} FROM nodes WHERE id = ?1"),
                    [id],
                    Node::from_row,
                )
                .optional()
                .map_err(Into::into)
            })
            .await
            .map_err(Into::into)
    }

    /// The breadcrumb from the root down to `id`, root first. Walks `parent_id`
    /// up via a recursive CTE, then returns the path ordered top-down. NotFound
    /// if `id` doesn't exist (the walk is empty).
    pub async fn path_to(&self, id: &str) -> Result<Vec<Node>, CoreError> {
        let id = id.to_string();
        let path = self
            .db
            .call(move |conn| {
                // `depth` counts hops from `id` (0) up to the root; ordering by
                // it descending yields the root-first breadcrumb.
                let mut stmt = conn.prepare(&format!(
                    "WITH RECURSIVE ancestors(id, parent_id, name, position, image_id, depth) AS (
                         SELECT {NODE_COLUMNS}, 0 FROM nodes WHERE id = ?1
                         UNION ALL
                         SELECT n.id, n.parent_id, n.name, n.position, n.image_id, a.depth + 1
                         FROM nodes n
                         JOIN ancestors a ON n.id = a.parent_id
                     )
                     SELECT {NODE_COLUMNS} FROM ancestors ORDER BY depth DESC"
                ))?;
                let nodes = stmt
                    .query_map([id], Node::from_row)?
                    .collect::<coven::rusqlite::Result<Vec<_>>>()?;
                Ok(nodes)
            })
            .await?;
        if path.is_empty() {
            return Err(CoreError::NotFound("no node for breadcrumb".into()));
        }
        Ok(path)
    }

    /// Append a child to `parent_id` at the end of its sibling order. The new
    /// node gets a fresh uuid and its `_updated_at` register stamp.
    pub async fn create_child(&self, parent_id: &str, name: String) -> Result<Node, CoreError> {
        let id = uuid::Uuid::new_v4().to_string();
        let parent_id = parent_id.to_string();
        let updated_at = self.stamper.stamp();
        self.db
            .call(move |conn| {
                let position = next_position(conn, &parent_id)?;
                conn.execute(
                    "INSERT INTO nodes (id, parent_id, name, position, image_id, _updated_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        id,
                        parent_id,
                        name,
                        position,
                        Option::<String>::None,
                        updated_at
                    ],
                )?;
                Ok(Node {
                    id,
                    parent_id: Some(parent_id),
                    name,
                    position,
                    image_id: None,
                })
            })
            .await
            .map_err(Into::into)
    }

    /// Append a child to `parent_id` carrying `bytes` as its one image, in a
    /// single step that leaves no half-state. The image file is written first:
    /// if that fails, no node is created. The node row is then inserted with its
    /// `image_id` already set; if the insert fails, the just-written file is
    /// unlinked. Either both the node row and its image file exist, or neither —
    /// never a node with a missing image or an image with no node.
    pub async fn create_child_with_image(
        &self,
        parent_id: &str,
        name: String,
        bytes: Vec<u8>,
    ) -> Result<Node, CoreError> {
        let image_id = uuid::Uuid::new_v4().to_string();
        let path = self.dir.image_path(&image_id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                CoreError::Io(format!("creating image dir {}: {e}", parent.display()))
            })?;
        }
        std::fs::write(&path, &bytes)
            .map_err(|e| CoreError::Io(format!("writing image {}: {e}", path.display())))?;

        let id = uuid::Uuid::new_v4().to_string();
        let parent_id = parent_id.to_string();
        let updated_at = self.stamper.stamp();
        let row_image_id = image_id.clone();
        let inserted = self
            .db
            .call(move |conn| {
                let position = next_position(conn, &parent_id)?;
                conn.execute(
                    "INSERT INTO nodes (id, parent_id, name, position, image_id, _updated_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![id, parent_id, name, position, row_image_id, updated_at],
                )?;
                Ok(Node {
                    id,
                    parent_id: Some(parent_id),
                    name,
                    position,
                    image_id: Some(row_image_id),
                })
            })
            .await;
        if inserted.is_err() {
            // No node row references the file just written, so it is orphaned
            // under a fresh uuid. Unlink it before surfacing the error.
            self.remove_image_file(&image_id);
        }
        inserted.map_err(Into::into)
    }

    /// Rename a node. NotFound if no row matched.
    pub async fn rename(&self, id: &str, name: String) -> Result<(), CoreError> {
        let updated_at = self.stamper.stamp();
        let id_for_update = id.to_string();
        let affected = self
            .db
            .call(move |conn| {
                conn.execute(
                    "UPDATE nodes SET name = ?1, _updated_at = ?2 WHERE id = ?3",
                    params![name, updated_at, id_for_update],
                )
                .map_err(Into::into)
            })
            .await?;
        if affected == 0 {
            return Err(CoreError::NotFound(format!("no node {id} to rename")));
        }
        Ok(())
    }

    /// Delete a node and its whole subtree (the `parent_id` self-FK cascades the
    /// row deletes), then remove the subtree's image files from disk. The image
    /// ids are collected and the rows deleted in one connection call, so no
    /// concurrent insert can slip into the subtree between collect and delete.
    /// The disk unlinks are best-effort and a failed one is logged, not fatal
    /// (the row is already gone).
    ///
    /// The root house (`parent_id` NULL) cannot be deleted: the tree always has
    /// exactly one root, so deleting it would leave an empty library with no node
    /// to browse. NotFound if the node doesn't exist.
    pub async fn delete(&self, id: &str) -> Result<(), CoreError> {
        // The existence check, the root guard, and the delete all run in one
        // connection call, so the row can't be seen-then-vanish between them and
        // the DELETE always acts on the row we just validated.
        enum Outcome {
            NotFound,
            IsRoot,
            Deleted(Vec<String>),
        }

        let id_owned = id.to_string();
        let outcome = self
            .db
            .call(move |conn| {
                let parent_id: Option<Option<String>> = conn
                    .query_row(
                        "SELECT parent_id FROM nodes WHERE id = ?1",
                        [&id_owned],
                        |r| r.get::<_, Option<String>>(0),
                    )
                    .optional()?;
                match parent_id {
                    None => Ok(Outcome::NotFound),
                    Some(None) => Ok(Outcome::IsRoot),
                    Some(Some(_)) => {
                        // Walk the subtree down `parent_id`, collecting every
                        // node's image_id; the root of the walk is the node being
                        // deleted.
                        let mut stmt = conn.prepare(
                            "WITH RECURSIVE subtree(id) AS (
                                 SELECT id FROM nodes WHERE id = ?1
                                 UNION ALL
                                 SELECT n.id FROM nodes n JOIN subtree s ON n.parent_id = s.id
                             )
                             SELECT n.image_id FROM nodes n JOIN subtree s ON n.id = s.id \
                             WHERE n.image_id IS NOT NULL",
                        )?;
                        let image_ids = stmt
                            .query_map([&id_owned], |r| r.get::<_, String>(0))?
                            .collect::<coven::rusqlite::Result<Vec<_>>>()?;
                        conn.execute("DELETE FROM nodes WHERE id = ?1", [&id_owned])?;
                        Ok(Outcome::Deleted(image_ids))
                    }
                }
            })
            .await?;

        match outcome {
            Outcome::NotFound => Err(CoreError::NotFound(format!("no node {id} to delete"))),
            Outcome::IsRoot => Err(CoreError::Internal("cannot delete the root node".into())),
            Outcome::Deleted(image_ids) => {
                for image_id in image_ids {
                    self.remove_image_file(&image_id);
                }
                Ok(())
            }
        }
    }

    /// Set a node's image: write `bytes` to a fresh content-addressed file, point
    /// the node at it, then unlink the previous image file if there was one.
    /// NotFound if the node doesn't exist — checked before any file is written so
    /// a missing node leaves no orphan file behind.
    pub async fn set_image(&self, id: &str, bytes: Vec<u8>) -> Result<(), CoreError> {
        let existing = self
            .get(id)
            .await?
            .ok_or_else(|| CoreError::NotFound(format!("no node {id} for set_image")))?;

        let image_id = uuid::Uuid::new_v4().to_string();
        let path = self.dir.image_path(&image_id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                CoreError::Io(format!("creating image dir {}: {e}", parent.display()))
            })?;
        }
        std::fs::write(&path, &bytes)
            .map_err(|e| CoreError::Io(format!("writing image {}: {e}", path.display())))?;

        let updated_at = self.stamper.stamp();
        let id = id.to_string();
        let new_image_id = image_id.clone();
        let update = self
            .db
            .call(move |conn| {
                conn.execute(
                    "UPDATE nodes SET image_id = ?1, _updated_at = ?2 WHERE id = ?3",
                    params![new_image_id, updated_at, id],
                )
                .map_err(Into::into)
            })
            .await;
        if update.is_err() {
            // The row never took the new image, so the file just written is
            // orphaned under a fresh uuid nothing references. Unlink it before
            // surfacing the error rather than leaking it.
            self.remove_image_file(&image_id);
        }
        update?;

        if let Some(old) = existing.image_id {
            self.remove_image_file(&old);
        }
        Ok(())
    }

    /// The on-disk path for `image_id` as a string, iff the file exists. Sync —
    /// the UI calls it on the render path to load an image, so it does no
    /// database work.
    pub fn image_path_if_exists(&self, image_id: &str) -> Option<String> {
        let path = self.dir.image_path(image_id);
        match path.try_exists() {
            Ok(true) => Some(path.to_string_lossy().into_owned()),
            Ok(false) => None,
            Err(e) => {
                warn!(image_id, path = %path.display(), "checking image file existence failed: {e}");
                None
            }
        }
    }

    /// Best-effort unlink of an image file. A missing file is fine (already
    /// gone); any other failure is logged and skipped — the node row no longer
    /// references it, so a leftover file is harmless leakage, not a fault that
    /// should fail the operation that already committed.
    fn remove_image_file(&self, image_id: &str) {
        let path = self.dir.image_path(image_id);
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                debug!(image_id, path = %path.display(), "image file already absent during cleanup")
            }
            Err(e) => warn!(image_id, path = %path.display(), "failed to unlink image file: {e}"),
        }
    }
}

/// The next sibling position within `parent_id` (`MAX(position) + 1`, or 0 for
/// the first child).
fn next_position(conn: &Connection, parent_id: &str) -> coven::rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COALESCE(MAX(position) + 1, 0) FROM nodes WHERE parent_id = ?1",
        [parent_id],
        |r| r.get(0),
    )
}

/// The `nodes` schema run by [`crate::app::bootstrap`]'s migrate closure. coven
/// runs its own bookkeeping migration first, then this. `parent_id` is a
/// self-FK with `ON DELETE CASCADE` (coven turns `foreign_keys` ON), so deleting
/// a node deletes its whole subtree in one statement.
pub const SCHEMA: &str = "\
CREATE TABLE IF NOT EXISTS nodes (
    id          TEXT PRIMARY KEY NOT NULL,
    parent_id   TEXT REFERENCES nodes(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    position    INTEGER NOT NULL,
    image_id    TEXT,
    _updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_nodes_parent ON nodes(parent_id, position);
";

/// Insert the root house node (parent NULL, position 0) with the given name. Used
/// at library creation; the caller supplies the register stamp so the write
/// shares the library's clock.
pub fn insert_root(
    conn: &Connection,
    id: &str,
    name: &str,
    updated_at: &str,
) -> coven::rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO nodes (id, parent_id, name, position, image_id, _updated_at) \
         VALUES (?1, NULL, ?2, 0, NULL, ?3)",
        params![id, name, updated_at],
    )?;
    Ok(())
}
