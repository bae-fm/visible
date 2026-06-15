//! The node tree and the live, database-backed [`Inventory`] service.
//!
//! Everything the user owns is a node in one self-referential tree: the house
//! at the root (`parent_id` NULL), then rooms, containers, and things, each
//! optionally carrying one image file. A node's children are its contents.
//!
//! ## Image blobs and cloud sync
//!
//! A node's image is stored locally as a content-addressed plaintext file named
//! by its image id. Each image is also an immutable row in the synced
//! `node_images` table (id = image id, `node_id` = the owning node); `nodes`
//! keeps `image_id` as the pointer to the current image. A node's image is
//! always added as a fresh `node_images` INSERT and removed as a DELETE — the
//! row is never updated, so replacing a photo is "DELETE the old row, INSERT a
//! new one", never an UPDATE.
//!
//! That immutability is what makes images propagate over cloud sync: coven's
//! blob channel uploads a blob for every INSERT it pushes (see
//! [`crate::blob_plan`]), so a new image — whether the first photo on a node or
//! a replacement — rides its `node_images` INSERT to every online peer. Image
//! deletes go through coven's cloud outbox (the blob delete is enqueued after the
//! row is gone), and the intent is recorded unconditionally — for a local-only
//! library the outbox simply never drains.

use coven::clock::ClockRef;
use coven::id_provider::IdRef;
use coven::library_dir::LibraryDir;
use coven::rusqlite::{params, Connection, OptionalExtension, Row};
use coven::{Database, UpdatedAtStamper};
use tracing::{debug, warn};

use crate::error::CoreError;

/// One node in the tree. `_updated_at` is coven's last-writer-wins register, not
/// domain data the UI reads, so it is absent here — only the live write path
/// touches it, via the stamper.
///
/// The attribute fields below the structural ones (`quantity` … `barcode`) are
/// each individually optional: a node may carry any subset of them and absence
/// is `None`, never a zero or empty string. They stay in their stored form here —
/// `value_cents` in cents, `acquired_at` as the ISO `YYYY-MM-DD` string — and the
/// edit form converts to its editable representation (dollars, a date picker) on
/// the way in and back on save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub id: String,
    pub parent_id: Option<String>,
    /// The node's title, or `None` while it is untitled. A photo-first child
    /// starts untitled (the photo is its identity) and stays so until renamed.
    /// The root house is always titled.
    pub name: Option<String>,
    pub position: i64,
    pub image_id: Option<String>,
    /// How many of this thing the node stands for (a single item is `None`, not
    /// `1` — absence, not a count of one).
    pub quantity: Option<i64>,
    /// Free-text notes.
    pub notes: Option<String>,
    /// The thing's value in whole cents.
    pub value_cents: Option<i64>,
    /// When the thing was acquired, as an ISO `YYYY-MM-DD` date string.
    pub acquired_at: Option<String>,
    /// The thing's serial number.
    pub serial: Option<String>,
    /// The barcode printed on the thing.
    pub barcode: Option<String>,
}

impl Node {
    /// The "×N" count badge the browse card shows, or `None` for a node that
    /// stands for a single thing. A quantity of `None` or `1` is one item and
    /// carries no badge; a quantity greater than one renders as `×N`. The
    /// threshold and format are a display derivation, precomputed in core (like
    /// `SearchHit::path_label`) so the cards render the string directly rather
    /// than each platform re-deriving it.
    pub fn quantity_badge(&self) -> Option<String> {
        match self.quantity {
            Some(n) if n > 1 => Some(format!("×{n}")),
            _ => None,
        }
    }

    /// Read a node from a row selecting [`NODE_COLUMNS`] in that order.
    fn from_row(row: &Row<'_>) -> coven::rusqlite::Result<Node> {
        Ok(Node {
            id: row.get(0)?,
            parent_id: row.get(1)?,
            name: row.get(2)?,
            position: row.get(3)?,
            image_id: row.get(4)?,
            quantity: row.get(5)?,
            notes: row.get(6)?,
            value_cents: row.get(7)?,
            acquired_at: row.get(8)?,
            serial: row.get(9)?,
            barcode: row.get(10)?,
        })
    }
}

/// One node with its tags, returned by [`Inventory::detail`] for the edit screen.
/// Tags live in their own synced table, so the node row and its tags are read
/// together and handed across the bridge as one record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeDetail {
    pub node: Node,
    pub tags: Vec<String>,
}

/// The columns every node read selects, in the order [`Node::from_row`] expects.
const NODE_COLUMNS: &str =
    "id, parent_id, name, position, image_id, quantity, notes, value_cents, acquired_at, serial, barcode";

/// The recursive CTE that enumerates a node's whole subtree by id: the node at
/// `?1` plus every descendant reached by walking `parent_id` downward. Both the
/// delete cleanup ([`Inventory::delete`], collecting the subtree's image rows)
/// and the move cycle-check ([`Inventory::move_node`], rejecting a move into the
/// node's own subtree) prepend this to their own tail SELECT, so the descendant
/// walk is defined in one place.
const SUBTREE_CTE: &str = "\
WITH RECURSIVE subtree(id) AS (
    SELECT id FROM nodes WHERE id = ?1
    UNION ALL
    SELECT n.id FROM nodes n JOIN subtree s ON n.parent_id = s.id
)";

/// The placeholder a search breadcrumb shows for an untitled ancestor, matching
/// the UI's untitled placeholder so a `path_label` reads the same as the rest of
/// the app.
const UNTITLED_LABEL: &str = "Untitled";

/// The separator between breadcrumb segments in a [`SearchHit::path_label`].
const BREADCRUMB_SEPARATOR: &str = " › ";

/// One search match: the matching [`Node`], its breadcrumb from the root down to
/// it inclusive (`path`, the same shape [`Inventory::path_to`] returns), and a
/// display label of just its ancestors (`path_label`, root→parent, the matched
/// node excluded). The label is built in core so the UI renders it directly
/// rather than joining names itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub node: Node,
    pub path: Vec<Node>,
    pub path_label: String,
}

/// The cloud key for an image blob: `images/{ab}/{cd}/{image_id}`. coven's blob
/// layout and cloud outbox use the same content-addressed key, so the changeset
/// channel upload of a `node_images` INSERT and the outbox delete of the same
/// image name the same object. The single home for this key, called by the
/// delete path here and by the integration tests asserting outbox intents. The
/// namespace is shared with the push/pull path in [`crate::blob_plan`] so the two
/// can't drift.
pub fn image_cloud_key(image_id: &str) -> String {
    LibraryDir::hashed_path(crate::blob_plan::IMAGE_NAMESPACE, image_id)
}

/// The live inventory for one open library: the node tree plus its image files.
/// Holds the coven database handle (the owned SQLite connection), the register
/// stamper bound into every node write, the library directory that locates image
/// files on disk, the id source for new nodes and images, and the wall clock
/// that timestamps cloud-outbox intents.
pub struct Inventory {
    db: Database,
    stamper: UpdatedAtStamper,
    dir: LibraryDir,
    ids: IdRef,
    clock: ClockRef,
}

impl Inventory {
    pub fn new(
        db: Database,
        stamper: UpdatedAtStamper,
        dir: LibraryDir,
        ids: IdRef,
        clock: ClockRef,
    ) -> Self {
        Self {
            db,
            stamper,
            dir,
            ids,
            clock,
        }
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

    /// The breadcrumb from the root down to `id`, root first (the same shape
    /// [`Inventory::search`] returns per match). NotFound if `id` doesn't exist
    /// (the walk is empty).
    pub async fn path_to(&self, id: &str) -> Result<Vec<Node>, CoreError> {
        let id = id.to_string();
        let path = self.db.call(move |conn| Ok(ancestors(conn, &id)?)).await?;
        if path.is_empty() {
            return Err(CoreError::NotFound("no node for breadcrumb".into()));
        }
        Ok(path)
    }

    /// Every node whose `name` contains `query` (case-insensitive substring),
    /// each with its breadcrumb. The root house is excluded — you are always "in"
    /// it, so search finds its contents, not the house itself — and untitled
    /// nodes (`name` NULL) never match a text query. An empty or whitespace-only
    /// query has nothing to match, so returns no results. The match is a literal
    /// substring: LIKE's own `%` and `_` wildcards are escaped so a query like
    /// `100%` matches the text "100%", not "anything starting with 100". The
    /// match is case-insensitive (LIKE folds ASCII case); results are then ordered
    /// by name under `NOCASE` so they read in natural alphabetical order
    /// regardless of capitalization. Each match's breadcrumb is walked on the same
    /// connection as the match query, so the whole search is one connection call
    /// (no per-match round trip).
    pub async fn search(&self, query: &str) -> Result<Vec<SearchHit>, CoreError> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(vec![]);
        }
        // The query is a literal substring, so escape LIKE's wildcards (`%`, `_`)
        // and the escape char itself, then wrap in `%…%`. Declared with `ESCAPE
        // '\'` below so a typed `%` or `_` matches that character, not a glob.
        let escaped = query
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let pattern = format!("%{escaped}%");
        self.db
            .call(move |conn| {
                let mut stmt = conn.prepare(&format!(
                    "SELECT {NODE_COLUMNS} FROM nodes \
                     WHERE parent_id IS NOT NULL \
                       AND name LIKE ?1 ESCAPE '\\' \
                     ORDER BY name COLLATE NOCASE"
                ))?;
                let matches = stmt
                    .query_map([&pattern], Node::from_row)?
                    .collect::<coven::rusqlite::Result<Vec<_>>>()?;
                let hits = matches
                    .into_iter()
                    .map(|node| {
                        let path = ancestors(conn, &node.id)?;
                        let path_label = breadcrumb_label(&path);
                        Ok(SearchHit {
                            node,
                            path,
                            path_label,
                        })
                    })
                    .collect::<coven::rusqlite::Result<Vec<_>>>()?;
                Ok(hits)
            })
            .await
            .map_err(Into::into)
    }

    /// Append an untitled child to `parent_id` carrying `bytes` as its one image,
    /// in a single step that leaves no half-state. The image file is written
    /// first: if that fails, no node is created. Then one connection call inserts
    /// the node row (its `image_id` set, `name = NULL` — a photo-first child
    /// starts untitled until renamed) and the matching `node_images` row, so the
    /// node and its image row commit together. The `node_images` INSERT is what
    /// carries the image blob to peers over coven's changeset channel. If the
    /// insert fails, the just-written file is unlinked. Either both the node row
    /// and its image file exist, or neither — never a node with a missing image
    /// or an image with no node.
    pub async fn create_child_with_image(
        &self,
        parent_id: &str,
        bytes: Vec<u8>,
    ) -> Result<Node, CoreError> {
        let image_id = self.store_new_image(&bytes)?;

        let id = self.ids.new_id();
        let parent_id = parent_id.to_string();
        let updated_at = self.stamper.stamp();
        let row_image_id = image_id.clone();
        let inserted = self
            .db
            .call(move |conn| {
                let position = next_position(conn, &parent_id)?;
                conn.execute(
                    "INSERT INTO nodes (id, parent_id, name, position, image_id, _updated_at) \
                     VALUES (?1, ?2, NULL, ?3, ?4, ?5)",
                    params![id, parent_id, position, row_image_id, updated_at],
                )?;
                insert_node_image(conn, &row_image_id, &id, &updated_at)?;
                Ok(Node {
                    id,
                    parent_id: Some(parent_id),
                    name: None,
                    position,
                    image_id: Some(row_image_id),
                    // A fresh node carries no attributes; the INSERT omits these
                    // columns so the row stores NULL for each.
                    quantity: None,
                    notes: None,
                    value_cents: None,
                    acquired_at: None,
                    serial: None,
                    barcode: None,
                })
            })
            .await;
        if inserted.is_err() {
            // No node row references the file just written, so it is orphaned
            // under a fresh id. Unlink it before surfacing the error.
            self.remove_image_file(&image_id);
        }
        inserted.map_err(Into::into)
    }

    /// Rename a node, giving it a title (an untitled node becomes titled).
    /// Rename always sets a name; it never clears one back to untitled. NotFound
    /// if no row matched.
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

    /// Set a node's attributes in one write: quantity, notes, value (cents),
    /// acquired date (ISO `YYYY-MM-DD`), serial, and barcode. Each is optional —
    /// `None` clears the column to NULL. A blank or whitespace-only string is the
    /// absence of the value, so it is normalized to NULL here (mirroring the
    /// S3-config endpoint/prefix normalization): a cleared text field is stored as
    /// absent, never as `""`. NotFound if the node doesn't exist.
    #[allow(clippy::too_many_arguments)]
    pub async fn set_attributes(
        &self,
        id: &str,
        quantity: Option<i64>,
        notes: Option<String>,
        value_cents: Option<i64>,
        acquired_at: Option<String>,
        serial: Option<String>,
        barcode: Option<String>,
    ) -> Result<(), CoreError> {
        let notes = blank_to_none(notes);
        let acquired_at = blank_to_none(acquired_at);
        let serial = blank_to_none(serial);
        let barcode = blank_to_none(barcode);

        let updated_at = self.stamper.stamp();
        let id_for_update = id.to_string();
        let affected = self
            .db
            .call(move |conn| {
                conn.execute(
                    "UPDATE nodes SET \
                         quantity = ?1, notes = ?2, value_cents = ?3, \
                         acquired_at = ?4, serial = ?5, barcode = ?6, _updated_at = ?7 \
                     WHERE id = ?8",
                    params![
                        quantity,
                        notes,
                        value_cents,
                        acquired_at,
                        serial,
                        barcode,
                        updated_at,
                        id_for_update
                    ],
                )
                .map_err(Into::into)
            })
            .await?;
        if affected == 0 {
            return Err(CoreError::NotFound(format!(
                "no node {id} to set attributes on"
            )));
        }
        Ok(())
    }

    /// Add `tag` to a node. The tag is trimmed; a blank tag is nothing to add and
    /// is skipped (logged). The insert is `INSERT OR IGNORE` against the
    /// `UNIQUE(node_id, tag)` constraint, so adding a tag the node already has is a
    /// no-op rather than an error.
    pub async fn add_tag(&self, id: &str, tag: String) -> Result<(), CoreError> {
        let tag = tag.trim().to_string();
        if tag.is_empty() {
            debug!(node_id = id, "skipping add of a blank tag");
            return Ok(());
        }
        let row_id = self.ids.new_id();
        let updated_at = self.stamper.stamp();
        let node_id = id.to_string();
        self.db
            .call(move |conn| {
                conn.execute(
                    "INSERT OR IGNORE INTO node_tags (id, node_id, tag, _updated_at) \
                     VALUES (?1, ?2, ?3, ?4)",
                    params![row_id, node_id, tag, updated_at],
                )
                .map_err(Into::into)
            })
            .await?;
        Ok(())
    }

    /// Remove `tag` from a node. A tag the node doesn't have is nothing to remove
    /// (the DELETE affects no rows).
    pub async fn remove_tag(&self, id: &str, tag: String) -> Result<(), CoreError> {
        let node_id = id.to_string();
        self.db
            .call(move |conn| {
                conn.execute(
                    "DELETE FROM node_tags WHERE node_id = ?1 AND tag = ?2",
                    params![node_id, tag],
                )
                .map_err(Into::into)
            })
            .await?;
        Ok(())
    }

    /// A node with its tags, for the edit screen. NotFound if the node doesn't
    /// exist. The node and its tags are read in one connection call so the screen
    /// loads them together.
    pub async fn detail(&self, id: &str) -> Result<NodeDetail, CoreError> {
        let query_id = id.to_string();
        let detail = self
            .db
            .call(move |conn| {
                let node = conn
                    .query_row(
                        &format!("SELECT {NODE_COLUMNS} FROM nodes WHERE id = ?1"),
                        [&query_id],
                        Node::from_row,
                    )
                    .optional()?;
                let Some(node) = node else {
                    return Ok(None);
                };
                let mut stmt =
                    conn.prepare("SELECT tag FROM node_tags WHERE node_id = ?1 ORDER BY tag")?;
                let tags = stmt
                    .query_map([&query_id], |r| r.get::<_, String>(0))?
                    .collect::<coven::rusqlite::Result<Vec<_>>>()?;
                Ok(Some(NodeDetail { node, tags }))
            })
            .await?;
        detail.ok_or_else(|| CoreError::NotFound(format!("no node {id} for detail")))
    }

    /// Delete a node and its whole subtree (the `parent_id` self-FK cascades the
    /// node row deletes, and each node's `node_images` rows cascade off their
    /// `node_id` FK), then remove the subtree's image files from disk and enqueue
    /// each image's cloud blob for deletion. The image ids are collected and the
    /// rows deleted in one connection call, so no concurrent insert can slip into
    /// the subtree between collect and delete. The cloud deletes are enqueued
    /// after the rows are gone (coven exposes no transaction-composable delete
    /// enqueue, and a delete needs no atomicity with the row — the referencing
    /// row is already removed). The disk unlinks are best-effort and a failed one
    /// is logged, not fatal (the row is already gone).
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
                        // image row's id from `node_images`; the root of the walk
                        // is the node being deleted. These ids drive the cloud
                        // deletes and on-disk unlinks below; the rows themselves
                        // cascade off the node DELETE.
                        let mut stmt = conn.prepare(&format!(
                            "{SUBTREE_CTE} \
                             SELECT ni.id FROM node_images ni JOIN subtree s ON ni.node_id = s.id",
                        ))?;
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
                    self.remove_image_blob(&image_id).await;
                }
                Ok(())
            }
        }
    }

    /// Move a node (with its whole subtree) under a different parent. The subtree
    /// rides along: only the moved node's own `parent_id` changes, so its
    /// descendants keep their parents and stay attached.
    ///
    /// The check and the write run in one connection call so nothing can slip
    /// between validating the move and applying it:
    ///
    /// - NotFound if `id` or `new_parent_id` doesn't exist.
    /// - The root house (`parent_id` NULL) cannot be moved — the tree has exactly
    ///   one root (`CoreError::Internal`, like [`Inventory::delete`]'s root guard).
    /// - A move into the node's own subtree is rejected: `new_parent_id` must be
    ///   neither `id` itself nor any descendant of `id`. Re-parenting a node under
    ///   one of its own descendants would detach that branch from the tree and
    ///   leave a cycle, so it is refused (`CoreError::Internal`). The subtree is
    ///   walked with the same recursive CTE [`Inventory::delete`] uses.
    /// - A move to the parent the node already has is nothing to do, so it returns
    ///   early without touching `position`.
    ///
    /// Otherwise the node's `parent_id` is set to `new_parent_id` and its
    /// `position` to the next sibling slot under that parent (the same
    /// `MAX(position) + 1` [`Inventory::create_child_with_image`] appends with), so
    /// the moved node lands last among its new siblings.
    pub async fn move_node(&self, id: &str, new_parent_id: &str) -> Result<(), CoreError> {
        enum Outcome {
            NotFound,
            IsRoot,
            IntoOwnSubtree,
            NoOp,
            Moved,
        }

        let id_owned = id.to_string();
        let new_parent_owned = new_parent_id.to_string();
        let updated_at = self.stamper.stamp();
        let outcome = self
            .db
            .call(move |conn| {
                let current_parent: Option<Option<String>> = conn
                    .query_row(
                        "SELECT parent_id FROM nodes WHERE id = ?1",
                        [&id_owned],
                        |r| r.get::<_, Option<String>>(0),
                    )
                    .optional()?;
                let current_parent = match current_parent {
                    None => return Ok(Outcome::NotFound),
                    // The root has no parent, so it has nowhere to move to.
                    Some(None) => return Ok(Outcome::IsRoot),
                    Some(Some(parent)) => parent,
                };

                // The destination must exist before anything else acts on it.
                let new_parent_exists = conn
                    .query_row(
                        "SELECT 1 FROM nodes WHERE id = ?1",
                        [&new_parent_owned],
                        |_| Ok(()),
                    )
                    .optional()?
                    .is_some();
                if !new_parent_exists {
                    return Ok(Outcome::NotFound);
                }

                // Already a child of the destination: nothing to move.
                if current_parent == new_parent_owned {
                    return Ok(Outcome::NoOp);
                }

                // The destination must not be the node itself nor anything inside
                // its subtree. Walk the subtree down `parent_id` from `id` and
                // reject if the destination is in it — moving there would detach
                // the branch and create a cycle.
                let destination_in_subtree = conn
                    .query_row(
                        &format!("{SUBTREE_CTE} SELECT 1 FROM subtree WHERE id = ?2"),
                        params![&id_owned, &new_parent_owned],
                        |_| Ok(()),
                    )
                    .optional()?
                    .is_some();
                if destination_in_subtree {
                    return Ok(Outcome::IntoOwnSubtree);
                }

                let position = next_position(conn, &new_parent_owned)?;
                conn.execute(
                    "UPDATE nodes SET parent_id = ?1, position = ?2, _updated_at = ?3 WHERE id = ?4",
                    params![new_parent_owned, position, updated_at, id_owned],
                )?;
                Ok(Outcome::Moved)
            })
            .await?;

        match outcome {
            Outcome::NotFound => Err(CoreError::NotFound(format!(
                "no node {id} or parent {new_parent_id} to move"
            ))),
            Outcome::IsRoot => Err(CoreError::Internal("cannot move the root node".into())),
            Outcome::IntoOwnSubtree => Err(CoreError::Internal(
                "cannot move a node into its own subtree".into(),
            )),
            Outcome::NoOp | Outcome::Moved => Ok(()),
        }
    }

    /// Set a node's image: write `bytes` to a fresh content-addressed file, then
    /// in one connection call insert a new `node_images` row, re-point the node
    /// at it, and delete the old image's row. The new `node_images` INSERT is
    /// what carries the replacement blob to peers over coven's changeset channel
    /// (an immutable row, never an UPDATE, so the new value is always readable).
    /// After the row write, unlink the previous image file and enqueue its cloud
    /// blob for deletion if there was one. NotFound if the node doesn't exist —
    /// checked before any file is written so a missing node leaves no orphan file
    /// behind.
    pub async fn set_image(&self, id: &str, bytes: Vec<u8>) -> Result<(), CoreError> {
        let existing = self
            .get(id)
            .await?
            .ok_or_else(|| CoreError::NotFound(format!("no node {id} for set_image")))?;

        let image_id = self.store_new_image(&bytes)?;

        let updated_at = self.stamper.stamp();
        let id = id.to_string();
        let new_image_id = image_id.clone();
        let old_image_id = existing.image_id.clone();
        let update = self
            .db
            .call(move |conn| {
                insert_node_image(conn, &new_image_id, &id, &updated_at)?;
                conn.execute(
                    "UPDATE nodes SET image_id = ?1, _updated_at = ?2 WHERE id = ?3",
                    params![new_image_id, updated_at, id],
                )?;
                if let Some(old) = &old_image_id {
                    detach_image_row(conn, old)?;
                }
                Ok(())
            })
            .await;
        if update.is_err() {
            // The row never took the new image, so the file just written is
            // orphaned under a fresh id nothing references. Unlink it before
            // surfacing the error rather than leaking it.
            self.remove_image_file(&image_id);
        }
        update?;

        if let Some(old) = existing.image_id {
            self.remove_image_blob(&old).await;
        }
        Ok(())
    }

    /// Clear a node's image: null its `image_id` and delete the old
    /// `node_images` row in one connection call, then unlink the old file and
    /// enqueue its cloud blob for deletion (best-effort, after the row is gone).
    /// This is the old-image cleanup half of [`Inventory::set_image`] with no
    /// replacement inserted — the row DELETE is what carries the removal to peers
    /// over coven's changeset channel. A node with no image is nothing to clear,
    /// so it returns `Ok(())` with a `debug!`. NotFound if the node doesn't
    /// exist.
    pub async fn clear_image(&self, id: &str) -> Result<(), CoreError> {
        let existing = self
            .get(id)
            .await?
            .ok_or_else(|| CoreError::NotFound(format!("no node {id} for clear_image")))?;

        let Some(old_image_id) = existing.image_id else {
            debug!(node_id = id, "node has no image to clear");
            return Ok(());
        };

        let updated_at = self.stamper.stamp();
        let id = id.to_string();
        let row_image_id = old_image_id.clone();
        self.db
            .call(move |conn| {
                conn.execute(
                    "UPDATE nodes SET image_id = NULL, _updated_at = ?1 WHERE id = ?2",
                    params![updated_at, id],
                )?;
                detach_image_row(conn, &row_image_id)?;
                Ok(())
            })
            .await?;

        self.remove_image_blob(&old_image_id).await;
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

    /// Mint a fresh image id, ensure its directory exists, and write `bytes` to
    /// the file at that id. Returns the new image id so the caller can point a
    /// node row at it (and unlink the file if the row write then fails). The id
    /// is fresh per call, so the file nothing references yet can't collide.
    fn store_new_image(&self, bytes: &[u8]) -> Result<String, CoreError> {
        let image_id = self.ids.new_id();
        let path = self.dir.image_path(&image_id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                CoreError::Io(format!("creating image dir {}: {e}", parent.display()))
            })?;
        }
        std::fs::write(&path, bytes)
            .map_err(|e| CoreError::Io(format!("writing image {}: {e}", path.display())))?;
        Ok(image_id)
    }

    /// Remove an image everywhere it lives once its `node_images` row is gone:
    /// unlink the local file and enqueue the cloud blob for deletion. Both steps
    /// are best-effort (see the two callees) — the row is already deleted, so a
    /// leftover file or cloud blob is leakage, not a fault. Called from the
    /// `delete` subtree cleanup, the `set_image` replace path, and the
    /// `clear_image` removal path, keeping the local-unlink and cloud-delete
    /// intent together.
    async fn remove_image_blob(&self, image_id: &str) {
        self.remove_image_file(image_id);
        self.enqueue_image_delete(image_id).await;
    }

    /// Enqueue an image's cloud blob for deletion. Best-effort: a failed enqueue
    /// is logged, not fatal — the node row that referenced the image is already
    /// gone, so a lingering cloud blob is leakage, not a fault that should fail
    /// the operation that already committed. Runs even for a local-only library
    /// (the outbox just never drains).
    async fn enqueue_image_delete(&self, image_id: &str) {
        let created_at = self.clock.now().to_rfc3339();
        if let Err(e) = self
            .db
            .enqueue_delete(&image_cloud_key(image_id), &created_at)
            .await
        {
            warn!(image_id, "failed to enqueue cloud blob delete: {e}");
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

/// The breadcrumb from the root down to `id`, root first. Walks `parent_id` up
/// via a recursive CTE, then returns the path ordered top-down (root, …, the
/// node). Empty when `id` doesn't exist (the walk starts from no row). The single
/// home for the ancestor walk, called by [`Inventory::path_to`] for one node's
/// breadcrumb and by [`Inventory::search`] for each match's, so the breadcrumb
/// query lives in one place.
fn ancestors(conn: &Connection, id: &str) -> coven::rusqlite::Result<Vec<Node>> {
    // `depth` counts hops from `id` (0) up to the root; ordering by it descending
    // yields the root-first breadcrumb. The walk carries no node columns of its
    // own — it climbs `parent_id` collecting ids with their depth, then the outer
    // query joins back to `nodes` to read the full [`NODE_COLUMNS`], so adding a
    // node column never touches the recursive step.
    let mut stmt = conn.prepare(&format!(
        "WITH RECURSIVE ancestors(node_id, next_parent, depth) AS (
             SELECT id, parent_id, 0 FROM nodes WHERE id = ?1
             UNION ALL
             SELECT n.id, n.parent_id, a.depth + 1
             FROM nodes n
             JOIN ancestors a ON n.id = a.next_parent
         )
         SELECT {NODE_COLUMNS} FROM nodes
         JOIN ancestors ON nodes.id = ancestors.node_id
         ORDER BY ancestors.depth DESC"
    ))?;
    let nodes = stmt
        .query_map([id], Node::from_row)?
        .collect::<coven::rusqlite::Result<Vec<_>>>()?;
    Ok(nodes)
}

/// The display label for a search match's ancestors: every node in its breadcrumb
/// except the matched node itself (the last element), joined by
/// [`BREADCRUMB_SEPARATOR`], with an untitled ancestor rendered as
/// [`UNTITLED_LABEL`]. The breadcrumb is root→node inclusive, so the ancestors
/// are every element before the last: a thing on a shelf with breadcrumb
/// `[Home, Living Room, Shelf, thing]` yields `"Home › Living Room › Shelf"`.
/// Empty for the root itself (no ancestors), though search never returns the
/// root.
fn breadcrumb_label(path: &[Node]) -> String {
    let ancestor_count = path.len().saturating_sub(1);
    path[..ancestor_count]
        .iter()
        .map(|node| node.name.as_deref().unwrap_or(UNTITLED_LABEL))
        .collect::<Vec<_>>()
        .join(BREADCRUMB_SEPARATOR)
}

/// An optional string with blank (empty or whitespace-only) treated as absence.
/// A cleared text field arrives as `Some("")` or `Some("   ")` from the form; its
/// meaning is "no value", so it normalizes to `None` and the column stores NULL.
/// A present value is trimmed of surrounding whitespace.
fn blank_to_none(value: Option<String>) -> Option<String> {
    value
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
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

/// Insert one immutable `node_images` row (id = image id, `node_id` = the owning
/// node). Shares the node write's register stamp so the image row and the node
/// it belongs to carry the same sync timestamp. Called inside the same
/// connection call as the node INSERT/UPDATE so the two commit together.
fn insert_node_image(
    conn: &Connection,
    image_id: &str,
    node_id: &str,
    updated_at: &str,
) -> coven::rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO node_images (id, node_id, _updated_at) VALUES (?1, ?2, ?3)",
        params![image_id, node_id, updated_at],
    )?;
    Ok(())
}

/// Delete the old image's immutable `node_images` row when a node's image is
/// replaced ([`Inventory::set_image`]) or cleared ([`Inventory::clear_image`]).
/// The DELETE is what carries the removal to peers over coven's changeset
/// channel; the matching local file and cloud blob are removed after the
/// connection call by [`Inventory::remove_image_blob`]. Run inside the same
/// connection call as the node UPDATE so the re-point and the row delete commit
/// together.
fn detach_image_row(conn: &Connection, old_image_id: &str) -> coven::rusqlite::Result<()> {
    conn.execute("DELETE FROM node_images WHERE id = ?1", [old_image_id])?;
    Ok(())
}

/// The schema run by [`crate::app::bootstrap`]'s migrate closure. coven runs its
/// own bookkeeping migration first, then this. `parent_id` is a self-FK with
/// `ON DELETE CASCADE` (coven turns `foreign_keys` ON), so deleting a node
/// deletes its whole subtree in one statement.
///
/// `node_images` is the synced table that carries image blobs: each row is one
/// image (id = image id), `node_id` references the owning node `ON DELETE
/// CASCADE` so deleting a node drops its image rows. The row is immutable —
/// inserted when an image is added, deleted when it is removed or replaced,
/// never updated — so every image change is an INSERT or DELETE that coven's
/// blob channel can carry. `nodes.image_id` points at the current image's id.
///
/// The attribute columns on `nodes` (`quantity` … `barcode`) are all nullable —
/// every node starts with them NULL and the edit form sets the ones the user
/// fills. `node_tags` is a plain synced table (no blob): each row is one tag on
/// one node, `UNIQUE(node_id, tag)` so adding the same tag twice is idempotent,
/// `ON DELETE CASCADE` so deleting a node drops its tags.
pub const SCHEMA: &str = "\
CREATE TABLE IF NOT EXISTS nodes (
    id          TEXT PRIMARY KEY NOT NULL,
    parent_id   TEXT REFERENCES nodes(id) ON DELETE CASCADE,
    name        TEXT,
    position    INTEGER NOT NULL,
    image_id    TEXT,
    quantity    INTEGER,
    notes       TEXT,
    value_cents INTEGER,
    acquired_at TEXT,
    serial      TEXT,
    barcode     TEXT,
    _updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_nodes_parent ON nodes(parent_id, position);
CREATE TABLE IF NOT EXISTS node_images (
    id          TEXT PRIMARY KEY NOT NULL,
    node_id     TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    _updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_node_images_node ON node_images(node_id);
CREATE TABLE IF NOT EXISTS node_tags (
    id          TEXT PRIMARY KEY NOT NULL,
    node_id     TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    tag         TEXT NOT NULL,
    _updated_at TEXT NOT NULL,
    UNIQUE(node_id, tag)
);
CREATE INDEX IF NOT EXISTS idx_node_tags_node ON node_tags(node_id);
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
