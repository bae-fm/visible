//! Tests the real [`Tasks`] service against a real coven [`Database`] on a temp
//! directory — the production unit, not a reconstruction.

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use coven::clock::{ClockRef, FixedClock};
use coven::id_provider::{IdRef, SequentialIdProvider};
use coven::library_dir::LibraryDir;
use tempfile::TempDir;
use visible_core::app::open_database;
use visible_core::{CoreError, Tasks};

/// Open a real database on a fresh temp library dir (which lays down the `tasks`
/// table) and return the live task list. Injects a deterministic id source and a
/// fixed clock so task ids are reproducible. The tempdir must outlive the service.
async fn open_tasks() -> (Tasks, TempDir) {
    let temp = TempDir::new().expect("temp dir");
    let dir = LibraryDir::new(temp.path().join("library"));
    std::fs::create_dir_all(&*dir).expect("create library dir");

    let (db, stamper) = open_database(&dir, "test-device".to_string()).expect("open database");

    let ids: IdRef = Arc::new(SequentialIdProvider::new("task"));
    let clock: ClockRef = Arc::new(FixedClock(
        Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
    ));
    (Tasks::new(db, stamper, ids, clock), temp)
}

#[tokio::test]
async fn created_tasks_appear_in_the_list_not_done() {
    let (tasks, _temp) = open_tasks().await;

    let made = tasks.create("Buy paper towels".to_string()).await.unwrap();
    assert_eq!(made.title, "Buy paper towels");
    assert!(!made.done);

    let list = tasks.list().await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, made.id);
    assert!(!list[0].done);
}

#[tokio::test]
async fn create_trims_the_title_and_rejects_a_blank_one() {
    let (tasks, _temp) = open_tasks().await;

    let made = tasks
        .create("  Water the plants  ".to_string())
        .await
        .unwrap();
    assert_eq!(made.title, "Water the plants");

    let blank = tasks.create("   ".to_string()).await;
    assert!(matches!(blank, Err(CoreError::Internal(_))));
    // The blank create stored nothing.
    assert_eq!(tasks.list().await.unwrap().len(), 1);
}

#[tokio::test]
async fn set_done_checks_a_task_off_and_back_on() {
    let (tasks, _temp) = open_tasks().await;
    let made = tasks.create("Fix the faucet".to_string()).await.unwrap();

    tasks.set_done(&made.id, true).await.unwrap();
    assert!(tasks.list().await.unwrap()[0].done);

    tasks.set_done(&made.id, false).await.unwrap();
    assert!(!tasks.list().await.unwrap()[0].done);
}

#[tokio::test]
async fn open_tasks_sort_before_done_ones() {
    let (tasks, _temp) = open_tasks().await;
    let first = tasks.create("first".to_string()).await.unwrap();
    let second = tasks.create("second".to_string()).await.unwrap();

    // Check the first one off; it must fall below the still-open second.
    tasks.set_done(&first.id, true).await.unwrap();

    let list = tasks.list().await.unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].id, second.id, "open task is first");
    assert!(!list[0].done);
    assert_eq!(list[1].id, first.id, "done task is last");
    assert!(list[1].done);
}

#[tokio::test]
async fn rename_changes_the_title_and_rejects_a_blank_one() {
    let (tasks, _temp) = open_tasks().await;
    let made = tasks.create("old".to_string()).await.unwrap();

    tasks.rename(&made.id, "  new  ".to_string()).await.unwrap();
    assert_eq!(tasks.list().await.unwrap()[0].title, "new");

    let blank = tasks.rename(&made.id, "  ".to_string()).await;
    assert!(matches!(blank, Err(CoreError::Internal(_))));
}

#[tokio::test]
async fn delete_removes_a_task_and_a_missing_one_is_not_an_error() {
    let (tasks, _temp) = open_tasks().await;
    let made = tasks.create("temporary".to_string()).await.unwrap();

    tasks.delete(&made.id).await.unwrap();
    assert!(tasks.list().await.unwrap().is_empty());

    // A co-householder may have removed it first; deleting an absent task is a
    // no-op, not a failure.
    tasks.delete(&made.id).await.unwrap();
}

#[tokio::test]
async fn set_done_and_rename_are_not_found_for_a_missing_task() {
    let (tasks, _temp) = open_tasks().await;
    assert!(matches!(
        tasks.set_done("ghost", true).await,
        Err(CoreError::NotFound(_))
    ));
    assert!(matches!(
        tasks.rename("ghost", "x".to_string()).await,
        Err(CoreError::NotFound(_))
    ));
}
