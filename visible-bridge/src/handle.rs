//! The open-library handle the UI calls: each method translates one
//! [`Inventory`] call to and from the bridge types. Local SQLite reads and
//! writes are shallow, so each method blocks the calling thread on the runtime
//! until the async `Inventory` call resolves.

use visible_core::RunningApp;

use crate::types::{
    BridgeError, BridgeMember, BridgeMemberRole, BridgeNode, BridgeNodeDetail,
    BridgeOutboxSnapshot, BridgeS3Config, BridgeSearchResult, BridgeSyncStatus,
};

#[derive(uniffi::Object)]
pub struct AppHandle {
    pub(crate) app: RunningApp,
}

#[uniffi::export]
impl AppHandle {
    pub fn root_node(&self) -> Result<BridgeNode, BridgeError> {
        Ok(self.app.runtime.block_on(self.app.inventory.root())?.into())
    }

    pub fn children(&self, parent_id: String) -> Result<Vec<BridgeNode>, BridgeError> {
        let children = self
            .app
            .runtime
            .block_on(self.app.inventory.children(&parent_id))?;
        Ok(children.into_iter().map(BridgeNode::from).collect())
    }

    pub fn get_node(&self, id: String) -> Result<Option<BridgeNode>, BridgeError> {
        Ok(self
            .app
            .runtime
            .block_on(self.app.inventory.get(&id))?
            .map(BridgeNode::from))
    }

    pub fn node_path(&self, id: String) -> Result<Vec<BridgeNode>, BridgeError> {
        let path = self.app.runtime.block_on(self.app.inventory.path_to(&id))?;
        Ok(path.into_iter().map(BridgeNode::from).collect())
    }

    pub fn search(&self, query: String) -> Result<Vec<BridgeSearchResult>, BridgeError> {
        let hits = self
            .app
            .runtime
            .block_on(self.app.inventory.search(&query))?;
        Ok(hits.into_iter().map(BridgeSearchResult::from).collect())
    }

    pub fn create_node_with_image(
        &self,
        parent_id: String,
        bytes: Vec<u8>,
    ) -> Result<BridgeNode, BridgeError> {
        Ok(self
            .app
            .runtime
            .block_on(
                self.app
                    .inventory
                    .create_child_with_image(&parent_id, bytes),
            )?
            .into())
    }

    pub fn rename_node(&self, id: String, name: String) -> Result<(), BridgeError> {
        Ok(self
            .app
            .runtime
            .block_on(self.app.inventory.rename(&id, name))?)
    }

    pub fn delete_node(&self, id: String) -> Result<(), BridgeError> {
        Ok(self.app.runtime.block_on(self.app.inventory.delete(&id))?)
    }

    pub fn set_node_image(&self, id: String, bytes: Vec<u8>) -> Result<(), BridgeError> {
        Ok(self
            .app
            .runtime
            .block_on(self.app.inventory.set_image(&id, bytes))?)
    }

    /// A node with its editable attributes and tags, for the edit screen.
    pub fn node_detail(&self, id: String) -> Result<BridgeNodeDetail, BridgeError> {
        Ok(self
            .app
            .runtime
            .block_on(self.app.inventory.detail(&id))?
            .into())
    }

    /// Set a node's attributes in one write. Each field is optional; a blank text
    /// field maps to absence — core normalizes it to NULL.
    #[allow(clippy::too_many_arguments)]
    pub fn update_node_attributes(
        &self,
        id: String,
        quantity: Option<i64>,
        notes: Option<String>,
        value_cents: Option<i64>,
        acquired_at: Option<String>,
        serial: Option<String>,
        barcode: Option<String>,
    ) -> Result<(), BridgeError> {
        Ok(self
            .app
            .runtime
            .block_on(self.app.inventory.set_attributes(
                &id,
                quantity,
                notes,
                value_cents,
                acquired_at,
                serial,
                barcode,
            ))?)
    }

    pub fn add_node_tag(&self, id: String, tag: String) -> Result<(), BridgeError> {
        Ok(self
            .app
            .runtime
            .block_on(self.app.inventory.add_tag(&id, tag))?)
    }

    pub fn remove_node_tag(&self, id: String, tag: String) -> Result<(), BridgeError> {
        Ok(self
            .app
            .runtime
            .block_on(self.app.inventory.remove_tag(&id, tag))?)
    }

    pub fn image_path_if_exists(&self, image_id: String) -> Option<String> {
        self.app.inventory.image_path_if_exists(&image_id)
    }

    /// Connect an S3 cloud home: probe, persist credentials + config, mint the
    /// encryption key, and start sync. Probing the bucket and starting sync use a
    /// deep stack and block on the runtime; the apps call this off the main
    /// thread.
    pub fn save_s3_config(&self, config: BridgeS3Config) -> Result<(), BridgeError> {
        Ok(self
            .app
            .runtime
            .block_on(self.app.sync.save_s3_config(config.into()))?)
    }

    /// Disconnect the cloud provider, stopping sync and clearing its config and
    /// credentials. The encryption key is kept so a later reconnect stays
    /// readable.
    pub fn disconnect_sync(&self) -> Result<(), BridgeError> {
        Ok(self.app.sync.disconnect()?)
    }

    /// Request an immediate sync cycle (no-op when sync isn't connected). Backs
    /// the settings screen's "Sync now" action.
    pub fn trigger_sync(&self) {
        self.app.sync.trigger_sync();
    }

    /// Whether a provider is configured and whether the loop is running.
    pub fn sync_status(&self) -> BridgeSyncStatus {
        self.app.sync.sync_status().into()
    }

    /// The pending cloud-outbox delete count for the status line.
    pub fn outbox_snapshot(&self) -> Result<BridgeOutboxSnapshot, BridgeError> {
        Ok(self
            .app
            .runtime
            .block_on(self.app.sync.outbox_snapshot())?
            .into())
    }

    /// This device's identity code (its Ed25519 public key, hex), which the user
    /// sends to a library owner out of band so the owner can invite them.
    pub fn user_identity_code(&self) -> Result<String, BridgeError> {
        Ok(self
            .app
            .runtime
            .block_on(self.app.sync.user_identity_code())?)
    }

    /// Invite a member by their identity code, granting `role`, and return the
    /// invite code to send back. Requires a connected sync loop.
    pub fn invite_member(
        &self,
        identity_code: String,
        role: BridgeMemberRole,
    ) -> Result<String, BridgeError> {
        Ok(self
            .app
            .runtime
            .block_on(self.app.sync.invite_member(&identity_code, role.into()))?)
    }

    /// The members of this shared library (empty for a local-only library).
    pub fn members(&self) -> Result<Vec<BridgeMember>, BridgeError> {
        let members = self.app.runtime.block_on(self.app.sync.members())?;
        Ok(members.into_iter().map(BridgeMember::from).collect())
    }

    /// Remove a member, rotating the library key to lock them out. Requires a
    /// connected sync loop.
    pub fn remove_member(&self, pubkey: String) -> Result<(), BridgeError> {
        Ok(self
            .app
            .runtime
            .block_on(self.app.sync.remove_member(&pubkey))?)
    }

    /// This owner device's recovery code, which carries the library key so the
    /// owner can restore the library on another device. Requires a connected sync
    /// loop.
    pub fn restore_code(&self) -> Result<String, BridgeError> {
        Ok(self.app.runtime.block_on(self.app.sync.restore_code())?)
    }
}
