//! Collaborative tasks: a shared to-do list for the home.
//!
//! A task is a title plus a done flag, living in the synced `tasks` table. The
//! table is a plain synced table (no blob), so every task rides coven's changeset
//! channel to the home's other members exactly like the node tree does — that is
//! what makes the list collaborative: a co-householder adds or checks off a task
//! and it shows up on everyone's device. There is no per-member ownership; the
//! list is shared, and any member can add, check off, rename, or remove a task.

use coven::clock::ClockRef;
use coven::id_provider::IdRef;
use coven::rusqlite::{params, Row};
use coven::{Database, UpdatedAtStamper};
use tracing::debug;

use crate::error::CoreError;

/// One task on the shared list. `_updated_at` is coven's last-writer-wins
/// register (not domain data the UI reads), so it is absent here — only the live
/// write path touches it, via the stamper. `created_at` is a stable wall-clock
/// timestamp used only to order the list; it never changes after creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub done: bool,
    pub created_at: String,
}

impl Task {
    /// Read a task from a row selecting [`TASK_COLUMNS`] in that order.
    fn from_row(row: &Row<'_>) -> coven::rusqlite::Result<Task> {
        Ok(Task {
            id: row.get(0)?,
            title: row.get(1)?,
            done: row.get(2)?,
            created_at: row.get(3)?,
        })
    }
}

/// The columns every task read selects, in the order [`Task::from_row`] expects.
const TASK_COLUMNS: &str = "id, title, done, created_at";

/// The live shared task list for one open library. Holds the coven database
/// handle, the register stamper bound into every write, the id source for new
/// tasks, and the wall clock that stamps a task's `created_at`.
pub struct Tasks {
    db: Database,
    stamper: UpdatedAtStamper,
    ids: IdRef,
    clock: ClockRef,
}

impl Tasks {
    pub fn new(db: Database, stamper: UpdatedAtStamper, ids: IdRef, clock: ClockRef) -> Self {
        Self {
            db,
            stamper,
            ids,
            clock,
        }
    }

    /// Every task, open ones first (so the list reads as "what's left to do",
    /// then "done"), and within each group the most recently added first.
    pub async fn list(&self) -> Result<Vec<Task>, CoreError> {
        self.db
            .call(|conn| {
                let mut stmt = conn.prepare(&format!(
                    "SELECT {TASK_COLUMNS} FROM tasks ORDER BY done ASC, created_at DESC"
                ))?;
                let tasks = stmt
                    .query_map([], Task::from_row)?
                    .collect::<coven::rusqlite::Result<Vec<_>>>()?;
                Ok(tasks)
            })
            .await
            .map_err(Into::into)
    }

    /// Add a new (not-done) task with the given title. The title is trimmed; a
    /// blank one is not a task, so it is rejected rather than stored as an empty
    /// row (the add control is disabled while the field is blank, so this guards
    /// against a value slipping through, not normal use).
    pub async fn create(&self, title: String) -> Result<Task, CoreError> {
        let title = title.trim().to_string();
        if title.is_empty() {
            return Err(CoreError::Internal(
                "cannot create a task with a blank title".into(),
            ));
        }
        let id = self.ids.new_id();
        let created_at = self.clock.now().to_rfc3339();
        let updated_at = self.stamper.stamp();
        let row_id = id.clone();
        let row_title = title.clone();
        let row_created = created_at.clone();
        self.db
            .call(move |conn| {
                conn.execute(
                    "INSERT INTO tasks (id, title, done, created_at, _updated_at) \
                     VALUES (?1, ?2, 0, ?3, ?4)",
                    params![row_id, row_title, row_created, updated_at],
                )
                .map_err(Into::into)
            })
            .await?;
        Ok(Task {
            id,
            title,
            done: false,
            created_at,
        })
    }

    /// Check a task off, or back on. NotFound if no task matched.
    pub async fn set_done(&self, id: &str, done: bool) -> Result<(), CoreError> {
        let updated_at = self.stamper.stamp();
        let id_owned = id.to_string();
        let affected = self
            .db
            .call(move |conn| {
                conn.execute(
                    "UPDATE tasks SET done = ?1, _updated_at = ?2 WHERE id = ?3",
                    params![done, updated_at, id_owned],
                )
                .map_err(Into::into)
            })
            .await?;
        if affected == 0 {
            return Err(CoreError::NotFound(format!("no task {id} to set done")));
        }
        Ok(())
    }

    /// Rename a task. The new title is trimmed; a blank one is rejected (a task
    /// always has a title). NotFound if no task matched.
    pub async fn rename(&self, id: &str, title: String) -> Result<(), CoreError> {
        let title = title.trim().to_string();
        if title.is_empty() {
            return Err(CoreError::Internal(
                "cannot rename a task to a blank title".into(),
            ));
        }
        let updated_at = self.stamper.stamp();
        let id_owned = id.to_string();
        let affected = self
            .db
            .call(move |conn| {
                conn.execute(
                    "UPDATE tasks SET title = ?1, _updated_at = ?2 WHERE id = ?3",
                    params![title, updated_at, id_owned],
                )
                .map_err(Into::into)
            })
            .await?;
        if affected == 0 {
            return Err(CoreError::NotFound(format!("no task {id} to rename")));
        }
        Ok(())
    }

    /// Remove a task from the shared list. The row delete is what carries the
    /// removal to the home's other members over coven's changeset channel. A task
    /// that doesn't exist is nothing to remove, so a DELETE affecting no rows is
    /// not an error — logged and treated as already gone (a co-householder may
    /// have removed it first, which is the expected collaborative race, not a
    /// fault).
    pub async fn delete(&self, id: &str) -> Result<(), CoreError> {
        let id_owned = id.to_string();
        let affected = self
            .db
            .call(move |conn| {
                conn.execute("DELETE FROM tasks WHERE id = ?1", params![id_owned])
                    .map_err(Into::into)
            })
            .await?;
        if affected == 0 {
            debug!(task_id = id, "no task to delete; already removed");
        }
        Ok(())
    }
}

/// The schema for the shared task list, run by [`crate::app::open_database`]
/// after coven's bookkeeping migration and the node schema. `tasks` is a plain
/// synced table (no blob): each row is one task, `done` stored as 0/1. It carries
/// no foreign key — tasks are a home-level list, not attached to a node.
pub const SCHEMA: &str = "\
CREATE TABLE IF NOT EXISTS tasks (
    id          TEXT PRIMARY KEY NOT NULL,
    title       TEXT NOT NULL,
    done        INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL,
    _updated_at TEXT NOT NULL
);
";
