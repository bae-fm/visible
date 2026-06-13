//! The open-library handle the UI calls: each method translates one
//! [`Inventory`] call to and from the bridge types. Local SQLite reads and
//! writes are shallow, so each method blocks the calling thread on the runtime
//! until the async `Inventory` call resolves.

use visible_core::Inventory;

use crate::types::{BridgeError, BridgeNode};

#[derive(uniffi::Object)]
pub struct AppHandle {
    pub(crate) runtime: tokio::runtime::Runtime,
    pub(crate) inventory: Inventory,
}

#[uniffi::export]
impl AppHandle {
    pub fn root_node(&self) -> Result<BridgeNode, BridgeError> {
        Ok(self.runtime.block_on(self.inventory.root())?.into())
    }

    pub fn children(&self, parent_id: String) -> Result<Vec<BridgeNode>, BridgeError> {
        let children = self.runtime.block_on(self.inventory.children(&parent_id))?;
        Ok(children.into_iter().map(BridgeNode::from).collect())
    }

    pub fn get_node(&self, id: String) -> Result<Option<BridgeNode>, BridgeError> {
        Ok(self
            .runtime
            .block_on(self.inventory.get(&id))?
            .map(BridgeNode::from))
    }

    pub fn node_path(&self, id: String) -> Result<Vec<BridgeNode>, BridgeError> {
        let path = self.runtime.block_on(self.inventory.path_to(&id))?;
        Ok(path.into_iter().map(BridgeNode::from).collect())
    }

    pub fn create_node(&self, parent_id: String, name: String) -> Result<BridgeNode, BridgeError> {
        Ok(self
            .runtime
            .block_on(self.inventory.create_child(&parent_id, name))?
            .into())
    }

    pub fn rename_node(&self, id: String, name: String) -> Result<(), BridgeError> {
        Ok(self.runtime.block_on(self.inventory.rename(&id, name))?)
    }

    pub fn delete_node(&self, id: String) -> Result<(), BridgeError> {
        Ok(self.runtime.block_on(self.inventory.delete(&id))?)
    }

    pub fn set_node_image(&self, id: String, bytes: Vec<u8>) -> Result<(), BridgeError> {
        Ok(self
            .runtime
            .block_on(self.inventory.set_image(&id, bytes))?)
    }

    pub fn image_path_if_exists(&self, image_id: String) -> Option<String> {
        self.inventory.image_path_if_exists(&image_id)
    }
}
