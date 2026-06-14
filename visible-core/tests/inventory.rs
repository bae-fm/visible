//! Tests the real [`Inventory`] against a real coven [`Database`] on a temp
//! directory — the production unit, not a reconstruction.

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use coven::clock::{ClockRef, FixedClock};
use coven::id_provider::{IdRef, SequentialIdProvider};
use coven::library_dir::LibraryDir;
use coven::rusqlite::OptionalExtension;
use coven::{Database, UpdatedAtStamper};
use tempfile::TempDir;
use visible_core::app::open_database;
use visible_core::node::insert_root;
use visible_core::Inventory;

/// Open a real database on a fresh temp library dir, lay down the schema and a
/// root house node, and return the live inventory (plus the tempdir, which must
/// outlive it). Mirrors what `library::create` + `app::bootstrap` do, so the
/// test drives the production node-tree code. Injects a deterministic id source
/// and a fixed clock so node/image ids and outbox timestamps are reproducible.
async fn open_inventory() -> (Inventory, TempDir) {
    let (inv, _db, temp) = open_inventory_with_db().await;
    (inv, temp)
}

/// Like [`open_inventory`] but also returns the database handle, so a test can
/// read coven's cloud outbox to assert the image delete intents the node writes
/// enqueue.
async fn open_inventory_with_db() -> (Inventory, Database, TempDir) {
    let temp = TempDir::new().expect("temp dir");
    let dir = LibraryDir::new(temp.path().join("library"));
    std::fs::create_dir_all(&*dir).expect("create library dir");

    let (db, stamper) = open_database(&dir, "test-device".to_string()).expect("open database");

    seed_root(&db, &stamper).await;

    let ids: IdRef = Arc::new(SequentialIdProvider::new("node"));
    let clock: ClockRef = Arc::new(FixedClock(
        Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
    ));
    let inv = Inventory::new(db.clone(), stamper, dir, ids, clock);
    (inv, db, temp)
}

async fn seed_root(db: &Database, stamper: &UpdatedAtStamper) {
    let updated_at = stamper.stamp();
    db.call(move |conn| insert_root(conn, "root", "Home", &updated_at).map_err(Into::into))
        .await
        .expect("seed root");
}

/// Small stand-in image bytes for tests that need a child but don't care about
/// the image itself; every child is created photo-first.
const DUMMY_IMAGE: &[u8] = b"img";

#[tokio::test]
async fn root_exists_after_create() {
    let (inv, _temp) = open_inventory().await;
    let root = inv.root().await.expect("root");
    assert_eq!(root.id, "root");
    assert_eq!(root.name.as_deref(), Some("Home"));
    assert_eq!(root.parent_id, None);
    assert_eq!(root.position, 0);
}

#[tokio::test]
async fn children_keep_insertion_order() {
    let (inv, _temp) = open_inventory().await;

    let first = inv
        .create_child_with_image("root", DUMMY_IMAGE.to_vec())
        .await
        .unwrap();
    let second = inv
        .create_child_with_image("root", DUMMY_IMAGE.to_vec())
        .await
        .unwrap();
    let third = inv
        .create_child_with_image("root", DUMMY_IMAGE.to_vec())
        .await
        .unwrap();

    assert_eq!(first.position, 0);
    assert_eq!(second.position, 1);
    assert_eq!(third.position, 2);

    let children = inv.children("root").await.unwrap();
    let ids: Vec<&str> = children.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(
        ids,
        vec![first.id.as_str(), second.id.as_str(), third.id.as_str()]
    );
    // A photo-first child is untitled until renamed.
    assert!(children.iter().all(|n| n.name.is_none()));
}

#[tokio::test]
async fn rename_titles_an_untitled_child() {
    let (inv, _temp) = open_inventory().await;
    let node = inv
        .create_child_with_image("root", DUMMY_IMAGE.to_vec())
        .await
        .unwrap();
    assert_eq!(node.name, None);

    inv.rename(&node.id, "Bookshelf".into()).await.unwrap();

    let reloaded = inv.get(&node.id).await.unwrap().unwrap();
    assert_eq!(reloaded.name.as_deref(), Some("Bookshelf"));
}

#[tokio::test]
async fn rename_missing_node_is_not_found() {
    let (inv, _temp) = open_inventory().await;
    let err = inv.rename("nope", "X".into()).await.unwrap_err();
    assert!(
        matches!(err, visible_core::CoreError::NotFound(_)),
        "{err:?}"
    );
}

#[tokio::test]
async fn delete_removes_subtree_and_its_image_files() {
    let (inv, _temp) = open_inventory().await;

    // root -> room -> box (with an image) -> thing (with an image)
    let room = inv
        .create_child_with_image("root", DUMMY_IMAGE.to_vec())
        .await
        .unwrap();
    let container = inv
        .create_child_with_image(&room.id, DUMMY_IMAGE.to_vec())
        .await
        .unwrap();
    let thing = inv
        .create_child_with_image(&container.id, DUMMY_IMAGE.to_vec())
        .await
        .unwrap();

    inv.set_image(&container.id, b"box-photo".to_vec())
        .await
        .unwrap();
    inv.set_image(&thing.id, b"thing-photo".to_vec())
        .await
        .unwrap();

    let box_image = inv
        .get(&container.id)
        .await
        .unwrap()
        .unwrap()
        .image_id
        .unwrap();
    let thing_image = inv.get(&thing.id).await.unwrap().unwrap().image_id.unwrap();
    assert!(inv.image_path_if_exists(&box_image).is_some());
    assert!(inv.image_path_if_exists(&thing_image).is_some());

    inv.delete(&room.id).await.unwrap();

    // The whole subtree is gone.
    assert!(inv.get(&room.id).await.unwrap().is_none());
    assert!(inv.get(&container.id).await.unwrap().is_none());
    assert!(inv.get(&thing.id).await.unwrap().is_none());

    // And so are their image files.
    assert!(inv.image_path_if_exists(&box_image).is_none());
    assert!(inv.image_path_if_exists(&thing_image).is_none());

    // The root is untouched.
    assert!(inv.root().await.is_ok());
}

#[tokio::test]
async fn delete_missing_node_is_not_found() {
    let (inv, _temp) = open_inventory().await;
    let err = inv.delete("nope").await.unwrap_err();
    assert!(
        matches!(err, visible_core::CoreError::NotFound(_)),
        "{err:?}"
    );
}

#[tokio::test]
async fn delete_root_errors_and_leaves_the_tree_intact() {
    let (inv, _temp) = open_inventory().await;
    let room = inv
        .create_child_with_image("root", DUMMY_IMAGE.to_vec())
        .await
        .unwrap();

    let err = inv.delete("root").await.unwrap_err();
    assert!(
        matches!(err, visible_core::CoreError::Internal(_)),
        "{err:?}"
    );

    // The root and its subtree are untouched; a non-root delete still works.
    assert!(inv.root().await.is_ok());
    assert!(inv.get(&room.id).await.unwrap().is_some());
    inv.delete(&room.id).await.unwrap();
    assert!(inv.get(&room.id).await.unwrap().is_none());
    assert!(inv.root().await.is_ok());
}

#[tokio::test]
async fn set_image_writes_then_replacing_removes_the_old_file() {
    let (inv, _temp) = open_inventory().await;
    let node = inv
        .create_child_with_image("root", DUMMY_IMAGE.to_vec())
        .await
        .unwrap();

    inv.set_image(&node.id, b"first".to_vec()).await.unwrap();
    let first_image = inv.get(&node.id).await.unwrap().unwrap().image_id.unwrap();
    let first_path = inv
        .image_path_if_exists(&first_image)
        .expect("first image on disk");
    assert_eq!(std::fs::read(&first_path).unwrap(), b"first");

    inv.set_image(&node.id, b"second".to_vec()).await.unwrap();
    let second_image = inv.get(&node.id).await.unwrap().unwrap().image_id.unwrap();
    assert_ne!(first_image, second_image);

    // The old file is gone; the new one holds the new bytes.
    assert!(inv.image_path_if_exists(&first_image).is_none());
    let second_path = inv
        .image_path_if_exists(&second_image)
        .expect("second image on disk");
    assert_eq!(std::fs::read(&second_path).unwrap(), b"second");
}

#[tokio::test]
async fn create_child_with_image_lands_node_and_file_together() {
    let (inv, _temp) = open_inventory().await;

    let node = inv
        .create_child_with_image("root", b"toaster-photo".to_vec())
        .await
        .unwrap();

    // A photo-first child lands untitled.
    assert_eq!(node.name, None);
    assert_eq!(node.parent_id.as_deref(), Some("root"));
    let image_id = node.image_id.expect("node carries an image id");

    // The image file is on disk with the bytes we passed.
    let path = inv
        .image_path_if_exists(&image_id)
        .expect("image file on disk");
    assert_eq!(std::fs::read(&path).unwrap(), b"toaster-photo");

    // The node is a child of root, with its image id preserved on reload.
    let children = inv.children("root").await.unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].id, node.id);
    assert_eq!(children[0].image_id.as_deref(), Some(image_id.as_str()));
}

#[tokio::test]
async fn set_image_on_missing_node_writes_nothing() {
    let (inv, _temp) = open_inventory().await;
    let err = inv.set_image("nope", b"x".to_vec()).await.unwrap_err();
    assert!(
        matches!(err, visible_core::CoreError::NotFound(_)),
        "{err:?}"
    );
}

#[tokio::test]
async fn path_to_returns_root_first_breadcrumb() {
    let (inv, _temp) = open_inventory().await;

    let room = inv
        .create_child_with_image("root", DUMMY_IMAGE.to_vec())
        .await
        .unwrap();
    let container = inv
        .create_child_with_image(&room.id, DUMMY_IMAGE.to_vec())
        .await
        .unwrap();
    let thing = inv
        .create_child_with_image(&container.id, DUMMY_IMAGE.to_vec())
        .await
        .unwrap();

    let path = inv.path_to(&thing.id).await.unwrap();
    let ids: Vec<&str> = path.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(
        ids,
        vec![
            "root",
            room.id.as_str(),
            container.id.as_str(),
            thing.id.as_str()
        ]
    );

    // The root's own breadcrumb is just itself.
    let root_path = inv.path_to("root").await.unwrap();
    assert_eq!(root_path.len(), 1);
    assert_eq!(root_path[0].id, "root");
}

#[tokio::test]
async fn path_to_missing_node_is_not_found() {
    let (inv, _temp) = open_inventory().await;
    let err = inv.path_to("nope").await.unwrap_err();
    assert!(
        matches!(err, visible_core::CoreError::NotFound(_)),
        "{err:?}"
    );
}

/// Create a child of `parent` and give it `name`, returning the titled node id.
/// Every node is photo-first then renamed, mirroring how the app titles things.
async fn create_named(inv: &Inventory, parent: &str, name: &str) -> String {
    let node = inv
        .create_child_with_image(parent, DUMMY_IMAGE.to_vec())
        .await
        .unwrap();
    inv.rename(&node.id, name.into()).await.unwrap();
    node.id
}

#[tokio::test]
async fn move_node_reparents_a_leaf_and_appends_it_under_the_new_parent() {
    let (inv, _temp) = open_inventory().await;

    // Home -> Garage, Home -> Closet (with one existing thing so the moved node
    // lands at the next position, not 0).
    let garage = create_named(&inv, "root", "Garage").await;
    let closet = create_named(&inv, "root", "Closet").await;
    create_named(&inv, &closet, "Coat").await;
    let drill = create_named(&inv, &garage, "Drill").await;

    inv.move_node(&drill, &closet).await.unwrap();

    // The moved node now points at its new parent and sits after the existing
    // closet child.
    let moved = inv.get(&drill).await.unwrap().unwrap();
    assert_eq!(moved.parent_id.as_deref(), Some(closet.as_str()));
    assert_eq!(moved.position, 1);

    // It appears under the new parent and is gone from the old one.
    let closet_children: Vec<String> = inv
        .children(&closet)
        .await
        .unwrap()
        .into_iter()
        .map(|n| n.id)
        .collect();
    assert!(closet_children.contains(&drill));
    assert!(inv.children(&garage).await.unwrap().is_empty());
}

#[tokio::test]
async fn move_node_carries_its_whole_subtree() {
    let (inv, _temp) = open_inventory().await;

    // Home -> Garage -> Toolbox -> Wrench, and a separate Home -> Closet.
    let garage = create_named(&inv, "root", "Garage").await;
    let closet = create_named(&inv, "root", "Closet").await;
    let toolbox = create_named(&inv, &garage, "Toolbox").await;
    let wrench = create_named(&inv, &toolbox, "Wrench").await;

    // Move the toolbox (with the wrench inside it) under the closet.
    inv.move_node(&toolbox, &closet).await.unwrap();

    // The grandchild still resolves under the moved node: its breadcrumb is now
    // Home -> Closet -> Toolbox -> Wrench, proving the subtree travelled intact.
    let path: Vec<String> = inv
        .path_to(&wrench)
        .await
        .unwrap()
        .into_iter()
        .map(|n| n.id)
        .collect();
    assert_eq!(path, vec!["root".to_string(), closet, toolbox, wrench]);
}

#[tokio::test]
async fn move_node_into_itself_is_rejected() {
    let (inv, _temp) = open_inventory().await;
    let box_node = create_named(&inv, "root", "Box").await;

    let err = inv.move_node(&box_node, &box_node).await.unwrap_err();
    assert!(
        matches!(err, visible_core::CoreError::Internal(_)),
        "{err:?}"
    );

    // The node is untouched — still a child of root.
    let unchanged = inv.get(&box_node).await.unwrap().unwrap();
    assert_eq!(unchanged.parent_id.as_deref(), Some("root"));
}

#[tokio::test]
async fn move_node_into_a_descendant_is_rejected() {
    let (inv, _temp) = open_inventory().await;

    // Home -> Garage -> Toolbox -> Drawer. Moving the garage under the drawer
    // (its own grandchild) would detach the branch, so it must be refused.
    let garage = create_named(&inv, "root", "Garage").await;
    let toolbox = create_named(&inv, &garage, "Toolbox").await;
    let drawer = create_named(&inv, &toolbox, "Drawer").await;

    let err = inv.move_node(&garage, &drawer).await.unwrap_err();
    assert!(
        matches!(err, visible_core::CoreError::Internal(_)),
        "{err:?}"
    );

    // The whole branch is intact: the garage still hangs off root and the drawer
    // still hangs off the toolbox.
    assert_eq!(
        inv.get(&garage)
            .await
            .unwrap()
            .unwrap()
            .parent_id
            .as_deref(),
        Some("root")
    );
    assert_eq!(
        inv.get(&drawer)
            .await
            .unwrap()
            .unwrap()
            .parent_id
            .as_deref(),
        Some(toolbox.as_str())
    );
}

#[tokio::test]
async fn move_root_is_rejected_and_leaves_the_tree_intact() {
    let (inv, _temp) = open_inventory().await;
    let room = create_named(&inv, "root", "Room").await;

    let err = inv.move_node("root", &room).await.unwrap_err();
    assert!(
        matches!(err, visible_core::CoreError::Internal(_)),
        "{err:?}"
    );

    // The root still has no parent; the room is still its child.
    assert_eq!(inv.root().await.unwrap().parent_id, None);
    assert_eq!(
        inv.get(&room).await.unwrap().unwrap().parent_id.as_deref(),
        Some("root")
    );
}

#[tokio::test]
async fn move_node_with_a_missing_node_or_parent_is_not_found() {
    let (inv, _temp) = open_inventory().await;
    let room = create_named(&inv, "root", "Room").await;

    // Missing moving node.
    let err = inv.move_node("nope", &room).await.unwrap_err();
    assert!(
        matches!(err, visible_core::CoreError::NotFound(_)),
        "{err:?}"
    );

    // Missing destination parent.
    let err = inv.move_node(&room, "nope").await.unwrap_err();
    assert!(
        matches!(err, visible_core::CoreError::NotFound(_)),
        "{err:?}"
    );
}

#[tokio::test]
async fn move_node_to_its_existing_parent_is_a_no_op() {
    let (inv, _temp) = open_inventory().await;

    // Two children of root; the first is already at position 0.
    let first = create_named(&inv, "root", "First").await;
    create_named(&inv, "root", "Second").await;
    assert_eq!(inv.get(&first).await.unwrap().unwrap().position, 0);

    // Moving it under the parent it already has changes nothing — position is not
    // churned to the end.
    inv.move_node(&first, "root").await.unwrap();
    let after = inv.get(&first).await.unwrap().unwrap();
    assert_eq!(after.parent_id.as_deref(), Some("root"));
    assert_eq!(after.position, 0);
}

#[tokio::test]
async fn search_returns_the_match_with_its_breadcrumb_and_ancestor_label() {
    let (inv, _temp) = open_inventory().await;

    // Home -> Living Room -> Shelf -> Vase
    let room = create_named(&inv, "root", "Living Room").await;
    let shelf = create_named(&inv, &room, "Shelf").await;
    let vase = create_named(&inv, &shelf, "Vase").await;

    let hits = inv.search("vase").await.unwrap();
    assert_eq!(hits.len(), 1);
    let hit = &hits[0];
    assert_eq!(hit.node.id, vase);
    assert_eq!(hit.node.name.as_deref(), Some("Vase"));

    // The breadcrumb is the full root->node path, so the UI has every ancestor id
    // to navigate by.
    let path_ids: Vec<&str> = hit.path.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(
        path_ids,
        vec!["root", room.as_str(), shelf.as_str(), vase.as_str()]
    );

    // The label is the ancestors only (root->parent), the matched node excluded.
    assert_eq!(hit.path_label, "Home › Living Room › Shelf");
}

#[tokio::test]
async fn search_is_case_insensitive_and_matches_substrings() {
    let (inv, _temp) = open_inventory().await;
    let id = create_named(&inv, "root", "Blue Toolbox").await;

    for query in ["toolbox", "TOOLBOX", "ToolBox", "oolbo"] {
        let hits = inv.search(query).await.unwrap();
        assert_eq!(hits.len(), 1, "query {query:?}");
        assert_eq!(hits[0].node.id, id, "query {query:?}");
    }
}

#[tokio::test]
async fn search_excludes_the_root_and_untitled_nodes() {
    let (inv, _temp) = open_inventory().await;

    // An untitled (photo-first, never renamed) child of root.
    inv.create_child_with_image("root", DUMMY_IMAGE.to_vec())
        .await
        .unwrap();
    // A titled child of root.
    let book = create_named(&inv, "root", "Book").await;

    // The root house is named "Home" but is never a search result — you are
    // always in it.
    assert!(inv.search("home").await.unwrap().is_empty());

    // Untitled nodes never match a text query; the titled sibling does. A query
    // that would match nothing-but-untitled returns nothing.
    let hits = inv.search("book").await.unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].node.id, book);

    // A direct child of root has only the root above it, so its ancestor label is
    // just the root's name.
    assert_eq!(hits[0].path_label, "Home");
}

#[tokio::test]
async fn search_renders_an_untitled_ancestor_in_the_label() {
    let (inv, _temp) = open_inventory().await;

    // Home -> (untitled box) -> Wrench. The box is photo-first, never renamed.
    let box_node = inv
        .create_child_with_image("root", DUMMY_IMAGE.to_vec())
        .await
        .unwrap();
    create_named(&inv, &box_node.id, "Wrench").await;

    let hits = inv.search("wrench").await.unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path_label, "Home › Untitled");
}

#[tokio::test]
async fn search_orders_matches_by_name_case_insensitively() {
    let (inv, _temp) = open_inventory().await;

    // All three contain "box"; their names differ in capitalization. A binary
    // sort would put "Big box" and "Tin box" (uppercase) before "apple box"
    // (lowercase); a case-insensitive sort reads naturally: apple, Big, Tin.
    create_named(&inv, "root", "Tin box").await;
    create_named(&inv, "root", "apple box").await;
    create_named(&inv, "root", "Big box").await;

    let names: Vec<String> = inv
        .search("box")
        .await
        .unwrap()
        .into_iter()
        .filter_map(|hit| hit.node.name)
        .collect();
    assert_eq!(names, vec!["apple box", "Big box", "Tin box"]);
}

#[tokio::test]
async fn search_with_an_empty_or_whitespace_query_returns_nothing() {
    let (inv, _temp) = open_inventory().await;
    create_named(&inv, "root", "Anything").await;

    assert!(inv.search("").await.unwrap().is_empty());
    assert!(inv.search("   ").await.unwrap().is_empty());
    assert!(inv.search("\t\n").await.unwrap().is_empty());
}

#[tokio::test]
async fn search_treats_like_wildcards_as_literal_characters() {
    let (inv, _temp) = open_inventory().await;

    // `%` and `_` are LIKE wildcards; a typed one must match the literal
    // character, not glob. "100%" must match "100% cotton" but not "100 cotton",
    // and "_" must match nothing (no name contains a literal underscore).
    let percent = create_named(&inv, "root", "100% cotton").await;
    create_named(&inv, "root", "100 cotton").await;

    let hits = inv.search("100%").await.unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].node.id, percent);

    assert!(inv.search("_").await.unwrap().is_empty());
}

/// Whether a `node_images` row exists for `image_id`. Images push to peers as
/// `node_images` INSERTs over coven's changeset channel (see
/// [`visible_core::blob_plan`]), so the presence of this row is what coven's blob
/// plan carries to an online peer — there is no image upload in the cloud outbox.
async fn node_image_exists(db: &Database, image_id: &str) -> bool {
    let image_id = image_id.to_string();
    db.call(move |conn| {
        conn.query_row(
            "SELECT 1 FROM node_images WHERE id = ?1",
            [image_id],
            |_| Ok(()),
        )
        .optional()
        .map(|row| row.is_some())
        .map_err(Into::into)
    })
    .await
    .unwrap()
}

/// The image ids of every `node_images` row owned by `node_id`.
async fn node_image_ids_for(db: &Database, node_id: &str) -> Vec<String> {
    let node_id = node_id.to_string();
    db.call(move |conn| {
        let mut stmt = conn.prepare("SELECT id FROM node_images WHERE node_id = ?1")?;
        let ids = stmt
            .query_map([node_id], |r| r.get::<_, String>(0))?
            .collect::<coven::rusqlite::Result<Vec<_>>>()?;
        Ok(ids)
    })
    .await
    .unwrap()
}

/// The tags of `node_id` read straight from `node_tags`, ordered, so a cascade
/// test can assert the rows are gone independently of the `tags` query path.
async fn node_tags_for(db: &Database, node_id: &str) -> Vec<String> {
    let node_id = node_id.to_string();
    db.call(move |conn| {
        let mut stmt = conn.prepare("SELECT tag FROM node_tags WHERE node_id = ?1 ORDER BY tag")?;
        let tags = stmt
            .query_map([node_id], |r| r.get::<_, String>(0))?
            .collect::<coven::rusqlite::Result<Vec<_>>>()?;
        Ok(tags)
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn create_child_with_image_records_a_node_image_row_and_no_outbox_upload() {
    let (inv, db, _temp) = open_inventory_with_db().await;

    let node = inv
        .create_child_with_image("root", b"photo".to_vec())
        .await
        .unwrap();
    let image_id = node.image_id.unwrap();

    // The image is carried by its node_images INSERT over the changeset channel,
    // not by a cloud-outbox upload.
    assert_eq!(node_image_ids_for(&db, &node.id).await, vec![image_id]);
    assert!(db.get_pending_cloud_uploads().await.unwrap().is_empty());
    assert!(db.get_pending_cloud_deletes().await.unwrap().is_empty());
}

#[tokio::test]
async fn set_image_replaces_the_node_image_row_and_enqueues_the_old_delete() {
    let (inv, db, _temp) = open_inventory_with_db().await;

    let node = inv
        .create_child_with_image("root", b"first".to_vec())
        .await
        .unwrap();
    let first_image = node.image_id.unwrap();

    inv.set_image(&node.id, b"second".to_vec()).await.unwrap();
    let second_image = inv.get(&node.id).await.unwrap().unwrap().image_id.unwrap();

    // The node now owns exactly the replacement image row (old row deleted, new
    // row inserted — never an UPDATE), so the new image rides a fresh INSERT to a
    // peer. No image goes through the outbox upload channel.
    assert_eq!(
        node_image_ids_for(&db, &node.id).await,
        vec![second_image.clone()]
    );
    assert!(node_image_exists(&db, &second_image).await);
    assert!(!node_image_exists(&db, &first_image).await);
    assert!(db.get_pending_cloud_uploads().await.unwrap().is_empty());

    // The replaced image's cloud blob is queued for deletion.
    let delete_keys: Vec<String> = db
        .get_pending_cloud_deletes()
        .await
        .unwrap()
        .into_iter()
        .map(|e| e.cloud_key)
        .collect();
    assert_eq!(
        delete_keys,
        vec![visible_core::node::image_cloud_key(&first_image)]
    );
}

#[tokio::test]
async fn set_attributes_round_trips_every_field() {
    let (inv, _temp) = open_inventory().await;
    let node = inv
        .create_child_with_image("root", DUMMY_IMAGE.to_vec())
        .await
        .unwrap();

    // A fresh node carries no attributes.
    let fresh = inv.get(&node.id).await.unwrap().unwrap();
    assert_eq!(fresh.quantity, None);
    assert_eq!(fresh.notes, None);
    assert_eq!(fresh.value_cents, None);
    assert_eq!(fresh.acquired_at, None);
    assert_eq!(fresh.serial, None);
    assert_eq!(fresh.barcode, None);

    inv.set_attributes(
        &node.id,
        Some(3),
        Some("Two left handed".into()),
        Some(4999),
        Some("2024-03-15".into()),
        Some("SN-12345".into()),
        Some("0123456789012".into()),
    )
    .await
    .unwrap();

    let loaded = inv.get(&node.id).await.unwrap().unwrap();
    assert_eq!(loaded.quantity, Some(3));
    assert_eq!(loaded.notes.as_deref(), Some("Two left handed"));
    assert_eq!(loaded.value_cents, Some(4999));
    assert_eq!(loaded.acquired_at.as_deref(), Some("2024-03-15"));
    assert_eq!(loaded.serial.as_deref(), Some("SN-12345"));
    assert_eq!(loaded.barcode.as_deref(), Some("0123456789012"));
}

#[tokio::test]
async fn set_attributes_clears_with_none_and_normalizes_blank_strings_to_none() {
    let (inv, _temp) = open_inventory().await;
    let node = inv
        .create_child_with_image("root", DUMMY_IMAGE.to_vec())
        .await
        .unwrap();

    // Set everything, then clear it: None for the numbers, blank/whitespace for
    // the text fields — a cleared text box is absence, stored as NULL not "".
    inv.set_attributes(
        &node.id,
        Some(2),
        Some("notes".into()),
        Some(100),
        Some("2024-01-01".into()),
        Some("serial".into()),
        Some("barcode".into()),
    )
    .await
    .unwrap();

    inv.set_attributes(
        &node.id,
        None,
        Some("".into()),
        None,
        Some("   ".into()),
        Some("\t\n".into()),
        None,
    )
    .await
    .unwrap();

    let cleared = inv.get(&node.id).await.unwrap().unwrap();
    assert_eq!(cleared.quantity, None);
    assert_eq!(cleared.notes, None, "blank notes normalize to NULL");
    assert_eq!(cleared.value_cents, None);
    assert_eq!(
        cleared.acquired_at, None,
        "whitespace date normalizes to NULL"
    );
    assert_eq!(cleared.serial, None, "whitespace serial normalizes to NULL");
    assert_eq!(cleared.barcode, None);
}

#[tokio::test]
async fn set_attributes_trims_present_text_values() {
    let (inv, _temp) = open_inventory().await;
    let node = inv
        .create_child_with_image("root", DUMMY_IMAGE.to_vec())
        .await
        .unwrap();

    inv.set_attributes(
        &node.id,
        None,
        Some("  spaced notes  ".into()),
        None,
        None,
        Some("  SN-1  ".into()),
        None,
    )
    .await
    .unwrap();

    let loaded = inv.get(&node.id).await.unwrap().unwrap();
    assert_eq!(loaded.notes.as_deref(), Some("spaced notes"));
    assert_eq!(loaded.serial.as_deref(), Some("SN-1"));
}

#[tokio::test]
async fn set_attributes_on_missing_node_is_not_found() {
    let (inv, _temp) = open_inventory().await;
    let err = inv
        .set_attributes("nope", Some(1), None, None, None, None, None)
        .await
        .unwrap_err();
    assert!(
        matches!(err, visible_core::CoreError::NotFound(_)),
        "{err:?}"
    );
}

#[tokio::test]
async fn add_tag_is_idempotent_and_trimmed() {
    let (inv, _temp) = open_inventory().await;
    let node = inv
        .create_child_with_image("root", DUMMY_IMAGE.to_vec())
        .await
        .unwrap();

    inv.add_tag(&node.id, "fragile".into()).await.unwrap();
    // The same tag with surrounding whitespace is the same tag — trimmed, then
    // ignored by the UNIQUE constraint.
    inv.add_tag(&node.id, "  fragile  ".into()).await.unwrap();
    inv.add_tag(&node.id, "fragile".into()).await.unwrap();

    assert_eq!(inv.detail(&node.id).await.unwrap().tags, vec!["fragile"]);
}

#[tokio::test]
async fn add_tag_skips_a_blank_tag() {
    let (inv, _temp) = open_inventory().await;
    let node = inv
        .create_child_with_image("root", DUMMY_IMAGE.to_vec())
        .await
        .unwrap();

    inv.add_tag(&node.id, "".into()).await.unwrap();
    inv.add_tag(&node.id, "   ".into()).await.unwrap();

    assert!(inv.detail(&node.id).await.unwrap().tags.is_empty());
}

#[tokio::test]
async fn remove_tag_drops_one_tag_and_ignores_an_absent_one() {
    let (inv, _temp) = open_inventory().await;
    let node = inv
        .create_child_with_image("root", DUMMY_IMAGE.to_vec())
        .await
        .unwrap();

    inv.add_tag(&node.id, "keep".into()).await.unwrap();
    inv.add_tag(&node.id, "drop".into()).await.unwrap();

    inv.remove_tag(&node.id, "drop".into()).await.unwrap();
    // Removing a tag the node doesn't have is a no-op, not an error.
    inv.remove_tag(&node.id, "never-had-it".into())
        .await
        .unwrap();

    assert_eq!(inv.detail(&node.id).await.unwrap().tags, vec!["keep"]);
}

#[tokio::test]
async fn detail_returns_the_node_and_its_tags() {
    let (inv, _temp) = open_inventory().await;
    let node = inv
        .create_child_with_image("root", DUMMY_IMAGE.to_vec())
        .await
        .unwrap();
    inv.rename(&node.id, "Drill".into()).await.unwrap();
    inv.set_attributes(
        &node.id,
        Some(1),
        Some("cordless".into()),
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();
    inv.add_tag(&node.id, "power".into()).await.unwrap();
    inv.add_tag(&node.id, "garage".into()).await.unwrap();

    let detail = inv.detail(&node.id).await.unwrap();
    assert_eq!(detail.node.id, node.id);
    assert_eq!(detail.node.name.as_deref(), Some("Drill"));
    assert_eq!(detail.node.quantity, Some(1));
    assert_eq!(detail.node.notes.as_deref(), Some("cordless"));
    // Tags come back alphabetically, the same order as `tags`.
    assert_eq!(detail.tags, vec!["garage", "power"]);
}

#[tokio::test]
async fn detail_on_missing_node_is_not_found() {
    let (inv, _temp) = open_inventory().await;
    let err = inv.detail("nope").await.unwrap_err();
    assert!(
        matches!(err, visible_core::CoreError::NotFound(_)),
        "{err:?}"
    );
}

#[tokio::test]
async fn deleting_a_node_cascades_its_tags() {
    let (inv, db, _temp) = open_inventory_with_db().await;

    let room = inv
        .create_child_with_image("root", DUMMY_IMAGE.to_vec())
        .await
        .unwrap();
    let thing = inv
        .create_child_with_image(&room.id, DUMMY_IMAGE.to_vec())
        .await
        .unwrap();
    inv.add_tag(&room.id, "room-tag".into()).await.unwrap();
    inv.add_tag(&thing.id, "thing-tag".into()).await.unwrap();

    assert_eq!(node_tags_for(&db, &room.id).await, vec!["room-tag"]);
    assert_eq!(node_tags_for(&db, &thing.id).await, vec!["thing-tag"]);

    inv.delete(&room.id).await.unwrap();

    // The subtree's node_tags rows cascade off the node DELETE.
    assert!(node_tags_for(&db, &room.id).await.is_empty());
    assert!(node_tags_for(&db, &thing.id).await.is_empty());
}

#[tokio::test]
async fn delete_enqueues_a_cloud_delete_for_every_image_in_the_subtree() {
    let (inv, db, _temp) = open_inventory_with_db().await;

    let room = inv
        .create_child_with_image("root", b"room".to_vec())
        .await
        .unwrap();
    let thing = inv
        .create_child_with_image(&room.id, b"thing".to_vec())
        .await
        .unwrap();
    let room_image = room.image_id.unwrap();
    let thing_image = thing.image_id.unwrap();

    inv.delete(&room.id).await.unwrap();

    // The subtree's node_images rows cascade off the node DELETE.
    assert!(!node_image_exists(&db, &room_image).await);
    assert!(!node_image_exists(&db, &thing_image).await);

    // Each image's cloud blob is queued for deletion (built with the production
    // cloud-key function, not a re-spelled one).
    let delete_keys: Vec<String> = db
        .get_pending_cloud_deletes()
        .await
        .unwrap()
        .into_iter()
        .map(|e| e.cloud_key)
        .collect();
    assert!(delete_keys.contains(&visible_core::node::image_cloud_key(&room_image)));
    assert!(delete_keys.contains(&visible_core::node::image_cloud_key(&thing_image)));
}
