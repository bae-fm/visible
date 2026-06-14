//! The cloud-sync service: connect an S3 cloud home, mint the per-library
//! encryption key on connect, build and run coven's [`SyncManager`], and report
//! status. Sibling to [`crate::node::Inventory`] in
//! [`crate::app::RunningApp`].
//!
//! coven owns everything hard — the sync loop, encryption, the cloud layout, the
//! membership model. This service is the host wiring: it holds the live config,
//! the keyring-backed [`KeyService`], the shared [`Database`], and the one
//! [`SyncManager`] (rebuilt across reconnects, never the database or stamper).

use std::sync::{Arc, RwLock};

use coven::clock::ClockRef;
use coven::config::{CloudProvider, Config};
use coven::database::Database;
use coven::encryption::EncryptionService;
use coven::keys::{CloudHomeCredentials, KeyService};
use coven::library_dir::LibraryDir;
use coven::storage::cloud::s3::S3CloudHome;
use coven::storage::cloud::CloudHome;
use coven::sync::sync_manager::{ConfigProvider, SyncManager};
use tracing::warn;

use crate::blob_plan::NodeBlobPlan;
use crate::error::CoreError;

/// The fields needed to connect an S3-compatible cloud home. `endpoint` and
/// `key_prefix` are optional (AWS S3 needs no endpoint; the prefix is for sharing
/// a bucket across libraries).
pub struct S3ConfigData {
    pub bucket: String,
    pub region: String,
    pub endpoint: Option<String>,
    pub key_prefix: Option<String>,
    pub access_key: String,
    pub secret_key: String,
}

/// Whether sync is set up and whether the loop is actually running.
pub struct SyncStatusInfo {
    /// A provider is selected with its settings + credentials present.
    pub configured: bool,
    /// The background sync loop is running and draining.
    pub ready: bool,
}

/// Counts of pending cloud-outbox work, for the settings screen's status line.
pub struct OutboxSnapshot {
    pub pending_uploads: u64,
    pub pending_deletes: u64,
}

/// The cloud-sync service for one open library.
///
/// Holds the live [`Config`] (updated on connect/disconnect and read fresh by
/// coven through the config provider), the keyring-backed [`KeyService`], the
/// shared [`Database`] and its wall clock, the library directory (for the blob
/// plan), and the one [`SyncManager`] once built. The manager slot is rebuilt on
/// reconnect; the database and stamper are never rebuilt.
pub struct Sync {
    /// The live config, shared with coven's config provider closure so a
    /// connect/disconnect mutation here is seen by the sync loop without
    /// rebuilding the manager. Held behind `Arc` because the provider outlives a
    /// borrow of `self`.
    config: Arc<RwLock<Config>>,
    key_service: KeyService,
    db: Database,
    clock: ClockRef,
    dir: LibraryDir,
    manager: RwLock<Option<Arc<SyncManager>>>,
}

impl Sync {
    pub fn new(
        config: Config,
        key_service: KeyService,
        db: Database,
        clock: ClockRef,
        dir: LibraryDir,
    ) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            key_service,
            db,
            clock,
            dir,
            manager: RwLock::new(None),
        }
    }

    /// Connect an S3 cloud home: probe the bucket with the proposed credentials
    /// *before* persisting anything (a typo or missing bucket fails fast at setup
    /// time, not via a delayed reconnect banner), then save the credentials and
    /// the cloud-home config and bring the sync loop up.
    pub async fn save_s3_config(&self, data: S3ConfigData) -> Result<(), CoreError> {
        // Probe first — build a throwaway client just to verify reachability.
        let probe_home = S3CloudHome::new(
            data.bucket.clone(),
            data.region.clone(),
            data.endpoint.clone(),
            data.access_key.clone(),
            data.secret_key.clone(),
            data.key_prefix.clone(),
        )
        .await?;
        probe_home.probe().await?;

        self.key_service
            .set_cloud_home_credentials(&CloudHomeCredentials::S3 {
                access_key: data.access_key,
                secret_key: data.secret_key,
            })?;

        // Connecting fills the whole cloud home as a unit. Empty optional fields
        // normalize to None so a blank form box isn't persisted as Some("").
        {
            let mut config = self.config.write().unwrap();
            config.cloud_home.provider = Some(CloudProvider::S3);
            config.cloud_home.s3_bucket = Some(data.bucket);
            config.cloud_home.s3_region = Some(data.region);
            config.cloud_home.s3_endpoint = data.endpoint.filter(|s| !s.is_empty());
            config.cloud_home.s3_key_prefix = data.key_prefix.filter(|s| !s.is_empty());
            config.save()?;
        }

        self.ensure_manager_and_start().await
    }

    /// Disconnect the cloud provider: stop the sync loop, drop the manager, reset
    /// the cloud-home config, and delete the cloud-home credentials. The
    /// encryption key is kept — a later reconnect reuses it so previously
    /// uploaded blobs stay readable.
    pub fn disconnect(&self) -> Result<(), CoreError> {
        if let Some(manager) = self.manager.read().unwrap().clone() {
            manager.stop_sync();
        }
        *self.manager.write().unwrap() = None;

        {
            let mut config = self.config.write().unwrap();
            config.cloud_home = Default::default();
            config.save()?;
        }

        // Deleting the credentials is best-effort: the config is already cleared,
        // so a lingering keyring entry can't reconnect on its own.
        if let Err(e) = self.key_service.delete_cloud_home_credentials() {
            warn!("failed to delete cloud home credentials on disconnect: {e}");
        }
        Ok(())
    }

    /// Resume sync at launch for a library that already has a provider configured
    /// and an unlocked encryption key on this device. Builds and starts the
    /// manager from the already-resolved encryption service; the encryption-key
    /// hint is already recorded (this is a returning user), so it isn't re-written
    /// here.
    pub async fn attach_and_start(&self, encryption: EncryptionService) {
        let manager = Arc::new(self.build_manager(encryption));
        manager.start_sync().await;
        *self.manager.write().unwrap() = Some(manager.clone());
        manager.trigger_sync();
    }

    /// Ensure a [`SyncManager`] exists (minting the encryption key on first
    /// connect) and start its sync loop. If a manager already exists, just
    /// (re)start it.
    async fn ensure_manager_and_start(&self) -> Result<(), CoreError> {
        // Clone the Arc out and drop the lock guard before awaiting — holding a
        // std lock across an await point would risk a deadlock with another task
        // taking the same lock.
        let existing = self.manager.read().unwrap().clone();
        if let Some(manager) = existing {
            manager.start_sync().await;
            manager.trigger_sync();
            return Ok(());
        }

        // Mint the per-library encryption key now, on first connect — not at
        // library create, so a local-only library stays crypto-free. Idempotent:
        // a retry after a failed start reuses the key already in the keyring.
        let key_hex = self.key_service.get_or_create_encryption_key()?;
        let encryption = EncryptionService::new(&key_hex)?;
        let fingerprint = encryption.fingerprint();

        let manager = Arc::new(self.build_manager(encryption));
        manager.start_sync().await;
        *self.manager.write().unwrap() = Some(manager.clone());

        // Record the encryption-key hint only once the loop is actually running.
        // start_sync silently bails when the cloud home is unreachable or sync
        // init returns None; writing the fingerprint then would tell the next
        // launch "this library has encryption set up" and skip the unlock prompt
        // while sync is still broken. Leaving it unwritten keeps the next attempt
        // a clean retry; the manager is kept so a reconnect can recover.
        if manager.is_sync_ready() {
            let mut config = self.config.write().unwrap();
            config.encryption_key_stored = true;
            config.encryption_key_fingerprint = Some(fingerprint);
            config.save()?;
        } else {
            let config = self.config.read().unwrap();
            warn!(
                provider = ?config.cloud_home.provider,
                s3_bucket = ?config.cloud_home.s3_bucket,
                "sync loop did not start after connect; encryption-key fingerprint not recorded \
                 — the next connect attempt will retry from a clean state"
            );
        }

        manager.trigger_sync();
        Ok(())
    }

    /// Build coven's [`SyncManager`] from an unlocked encryption service. The
    /// config provider clones the live in-memory config each call, so coven sees
    /// connect/disconnect without the manager being rebuilt. The manager takes
    /// the shared [`Database`] directly (visible holds coven's database, not a
    /// wrapper) and reads the register clock from it; the wall [`ClockRef`] drives
    /// `created_at`/expiry comparisons. No upload observer — visible has no
    /// per-file progress UI yet, and the outbox drains regardless.
    fn build_manager(&self, encryption: EncryptionService) -> SyncManager {
        // The provider clones the live config each call, so connect/disconnect is
        // reflected without rebuilding the manager. It shares this service's
        // config behind `Arc`, since the closure outlives a borrow of `self`.
        let provider_config = self.config.clone();
        let config_provider: ConfigProvider =
            Arc::new(move || provider_config.read().unwrap().clone());

        let blob_plan: Arc<dyn coven::blob::BlobPlan> =
            Arc::new(NodeBlobPlan::new(self.dir.clone()));

        SyncManager::new(
            config_provider,
            self.key_service.clone(),
            encryption,
            self.db.clone(),
            self.clock.clone(),
            blob_plan,
            None,
        )
    }

    /// Trigger an immediate sync cycle (no-op when no manager is built).
    pub fn trigger_sync(&self) {
        if let Some(manager) = self.manager.read().unwrap().as_ref() {
            manager.trigger_sync();
        }
    }

    /// Whether the background sync loop is running (no manager → false).
    pub fn is_sync_ready(&self) -> bool {
        self.manager
            .read()
            .unwrap()
            .as_ref()
            .is_some_and(|m| m.is_sync_ready())
    }

    /// Whether a provider is configured (settings + credentials present) and
    /// whether the loop is running.
    pub fn sync_status(&self) -> SyncStatusInfo {
        let configured = self.config.read().unwrap().sync_enabled(&self.key_service);
        SyncStatusInfo {
            configured,
            ready: self.is_sync_ready(),
        }
    }

    /// Counts of pending cloud-outbox uploads and deletes.
    pub async fn outbox_snapshot(&self) -> Result<OutboxSnapshot, CoreError> {
        let pending_uploads = self.db.get_pending_cloud_uploads().await?.len() as u64;
        let pending_deletes = self.db.get_pending_cloud_deletes().await?.len() as u64;
        Ok(OutboxSnapshot {
            pending_uploads,
            pending_deletes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::{TimeZone, Utc};
    use coven::clock::FixedClock;
    use coven::sync::session::SyncedTable;

    /// Build a real `Sync` over a real coven database on a temp dir, with the
    /// in-memory test keyring installed — the production service, not a
    /// reconstruction. No cloud home is connected (real S3 needs a bucket), so
    /// this covers the local-only status/outbox surface and disconnect.
    async fn open_sync() -> (Sync, tempfile::TempDir) {
        crate::config::install_test_keyring();

        let temp = tempfile::TempDir::new().unwrap();
        let dir = LibraryDir::new(temp.path().join("library"));
        std::fs::create_dir_all(&*dir).unwrap();

        let config = Config::with_defaults(
            "lib-sync".to_string(),
            "device".to_string(),
            dir.clone(),
            "Home".to_string(),
        );
        config.save().unwrap();

        let (db, _stamper) = Database::open(
            &dir.db_path(),
            vec![SyncedTable::new("nodes")],
            "device".to_string(),
            |conn| {
                conn.execute_batch(
                    "CREATE TABLE nodes (id TEXT PRIMARY KEY NOT NULL, image_id TEXT, \
                     _updated_at TEXT NOT NULL);",
                )
                .map_err(Into::into)
            },
        )
        .unwrap();

        let key_service = KeyService::new("lib-sync".to_string());
        let clock: ClockRef = Arc::new(FixedClock(
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
        ));
        (Sync::new(config, key_service, db, clock, dir), temp)
    }

    #[tokio::test]
    async fn fresh_library_is_not_configured_and_not_ready() {
        let (sync, _temp) = open_sync().await;
        let status = sync.sync_status();
        assert!(!status.configured, "no provider connected yet");
        assert!(!status.ready, "no sync loop running");
        // Triggering with no manager is a no-op, not a panic.
        sync.trigger_sync();
        assert!(!sync.is_sync_ready());
    }

    #[tokio::test]
    async fn outbox_snapshot_counts_pending_uploads_and_deletes() {
        let (sync, _temp) = open_sync().await;

        let snap = sync.outbox_snapshot().await.unwrap();
        assert_eq!(snap.pending_uploads, 0);
        assert_eq!(snap.pending_deletes, 0);

        // Enqueue one of each through coven's outbox API (the same API the node
        // write path uses) and confirm the snapshot reflects the counts.
        sync.db
            .enqueue_upload(
                "img-1",
                "images/aa/bb/img-1",
                Some("/tmp/img-1"),
                coven::blob::BlobScope::Master,
                "2024-01-01T00:00:00Z",
            )
            .await
            .unwrap();
        sync.db
            .enqueue_delete("images/cc/dd/img-2", "2024-01-01T00:00:00Z")
            .await
            .unwrap();

        let snap = sync.outbox_snapshot().await.unwrap();
        assert_eq!(snap.pending_uploads, 1);
        assert_eq!(snap.pending_deletes, 1);
    }

    #[tokio::test]
    async fn disconnect_without_a_provider_is_a_clean_no_op() {
        let (sync, _temp) = open_sync().await;
        // Nothing is connected; disconnect clears the (already empty) cloud home
        // and returns Ok rather than erroring on the missing manager/credentials.
        sync.disconnect().unwrap();
        assert!(!sync.sync_status().configured);
    }
}
