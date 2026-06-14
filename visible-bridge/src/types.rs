//! The uniffi boundary types: the records and error the generated Swift/Kotlin
//! see, and the conversions from visible-core's domain types. Type translation
//! only — no logic.

use visible_core::{
    CoreError, LibraryInfo, Member, MemberRole, Node, NodeDetail, OutboxSnapshot, S3ConfigData,
    SearchHit, SyncStatusInfo,
};

/// A node as the browse list consumes it. No `position` — the bridge returns
/// children already ordered, so the UI iterates in order rather than re-sorting.
/// `quantity_badge` is the only attribute the list needs: the card shows it over
/// the thumbnail when a node stands for more than one thing, so the list carries
/// the precomputed badge to avoid a per-card detail fetch. The rest of the
/// attributes load through [`BridgeNodeDetail`] on the edit screen.
#[derive(uniffi::Record)]
pub struct BridgeNode {
    pub id: String,
    pub parent_id: Option<String>,
    /// The node's title, or `None` while it is untitled. The UI renders an
    /// "Untitled" fallback for `None`.
    pub name: Option<String>,
    pub image_id: Option<String>,
    /// The "×N" count badge for a node that stands for more than one thing, or
    /// `None` for a single item. Precomputed in core (see [`Node::quantity_badge`])
    /// so the card renders the string directly.
    pub quantity_badge: Option<String>,
}

impl From<Node> for BridgeNode {
    fn from(node: Node) -> Self {
        Self {
            quantity_badge: node.quantity_badge(),
            id: node.id,
            parent_id: node.parent_id,
            name: node.name,
            image_id: node.image_id,
        }
    }
}

/// A node with all of its editable attributes and tags, for the edit screen.
/// The edit form seeds its fields from these and writes them back through
/// `update_node_attributes` / `add_node_tag` / `remove_node_tag`. Values stay in
/// their stored form — `value_cents` in cents, `acquired_at` as the ISO
/// `YYYY-MM-DD` string — and the form converts to its editable representation
/// (dollars, a date picker) on the model, not across this boundary.
#[derive(uniffi::Record)]
pub struct BridgeNodeDetail {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: Option<String>,
    pub image_id: Option<String>,
    pub quantity: Option<i64>,
    pub notes: Option<String>,
    pub value_cents: Option<i64>,
    pub acquired_at: Option<String>,
    pub serial: Option<String>,
    pub barcode: Option<String>,
    pub tags: Vec<String>,
}

impl From<NodeDetail> for BridgeNodeDetail {
    fn from(detail: NodeDetail) -> Self {
        let node = detail.node;
        Self {
            id: node.id,
            parent_id: node.parent_id,
            name: node.name,
            image_id: node.image_id,
            quantity: node.quantity,
            notes: node.notes,
            value_cents: node.value_cents,
            acquired_at: node.acquired_at,
            serial: node.serial,
            barcode: node.barcode,
            tags: detail.tags,
        }
    }
}

/// One search match for the search screen: the matching `node`, the ancestor
/// ids the screen navigates by (`path`, root→node inclusive), and `path_label`,
/// the core-built breadcrumb of the match's ancestors that the row shows as
/// secondary text.
#[derive(uniffi::Record)]
pub struct BridgeSearchResult {
    pub node: BridgeNode,
    pub path: Vec<BridgeNode>,
    pub path_label: String,
}

impl From<SearchHit> for BridgeSearchResult {
    fn from(hit: SearchHit) -> Self {
        Self {
            node: hit.node.into(),
            path: hit.path.into_iter().map(BridgeNode::from).collect(),
            path_label: hit.path_label,
        }
    }
}

/// A library for the picker.
#[derive(uniffi::Record)]
pub struct BridgeLibrary {
    pub id: String,
    pub name: String,
}

impl From<LibraryInfo> for BridgeLibrary {
    fn from(info: LibraryInfo) -> Self {
        Self {
            id: info.id,
            name: info.name,
        }
    }
}

/// A member of a shared library, for the members list. `short_pubkey` is the
/// row label core already truncated; `is_self` marks this device's own entry so
/// the UI can label it.
#[derive(uniffi::Record)]
pub struct BridgeMember {
    pub pubkey: String,
    pub short_pubkey: String,
    pub role: BridgeMemberRole,
    pub is_self: bool,
}

impl From<Member> for BridgeMember {
    fn from(m: Member) -> Self {
        Self {
            pubkey: m.pubkey,
            short_pubkey: m.short_pubkey,
            role: m.role.into(),
            is_self: m.is_self,
        }
    }
}

/// A member's role: `Owner` and `Member` may author changes; `Follower` is
/// read-only. The FFI mirror of visible-core's [`MemberRole`] — it converts both
/// ways: from the core type for the members list, and into it for `invite_member`
/// where the UI picks the role to grant.
#[derive(Debug, PartialEq, uniffi::Enum)]
pub enum BridgeMemberRole {
    Owner,
    Member,
    Follower,
}

impl From<MemberRole> for BridgeMemberRole {
    fn from(role: MemberRole) -> Self {
        match role {
            MemberRole::Owner => BridgeMemberRole::Owner,
            MemberRole::Member => BridgeMemberRole::Member,
            MemberRole::Follower => BridgeMemberRole::Follower,
        }
    }
}

impl From<BridgeMemberRole> for MemberRole {
    fn from(role: BridgeMemberRole) -> Self {
        match role {
            BridgeMemberRole::Owner => MemberRole::Owner,
            BridgeMemberRole::Member => MemberRole::Member,
            BridgeMemberRole::Follower => MemberRole::Follower,
        }
    }
}

/// The S3 connection fields the settings form collects. `endpoint`/`key_prefix`
/// are optional (AWS S3 needs no endpoint; the prefix is for sharing a bucket
/// across libraries): the form maps a blank or whitespace-only box to `None`
/// before sending, so visible-core receives the absence as `None`, never `""`.
#[derive(uniffi::Record)]
pub struct BridgeS3Config {
    pub bucket: String,
    pub region: String,
    pub endpoint: Option<String>,
    pub key_prefix: Option<String>,
    pub access_key: String,
    pub secret_key: String,
}

impl From<BridgeS3Config> for S3ConfigData {
    fn from(c: BridgeS3Config) -> Self {
        S3ConfigData {
            bucket: c.bucket,
            region: c.region,
            endpoint: c.endpoint,
            key_prefix: c.key_prefix,
            access_key: c.access_key,
            secret_key: c.secret_key,
        }
    }
}

/// Whether sync is configured and whether the loop is running, for the settings
/// status line.
#[derive(uniffi::Record)]
pub struct BridgeSyncStatus {
    pub configured: bool,
    pub ready: bool,
}

impl From<SyncStatusInfo> for BridgeSyncStatus {
    fn from(s: SyncStatusInfo) -> Self {
        Self {
            configured: s.configured,
            ready: s.ready,
        }
    }
}

/// The pending cloud-outbox delete count, for the settings status line.
/// visible's outbox carries deletes only — images upload inline on the changeset
/// channel, never through the outbox.
#[derive(uniffi::Record)]
pub struct BridgeOutboxSnapshot {
    pub pending_deletes: u64,
}

impl From<OutboxSnapshot> for BridgeOutboxSnapshot {
    fn from(s: OutboxSnapshot) -> Self {
        Self {
            pending_deletes: s.pending_deletes,
        }
    }
}

/// The error surface the generated Swift/Kotlin throw and switch on.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum BridgeError {
    #[error("not found: {msg}")]
    NotFound { msg: String },
    #[error("database error: {msg}")]
    Database { msg: String },
    #[error("config error: {msg}")]
    Config { msg: String },
    #[error("keyring error: {msg}")]
    Keyring { msg: String },
    #[error("sync error: {msg}")]
    Sync { msg: String },
    #[error("internal error: {msg}")]
    Internal { msg: String },
}

impl From<CoreError> for BridgeError {
    fn from(e: CoreError) -> Self {
        match e {
            CoreError::NotFound(msg) => BridgeError::NotFound { msg },
            CoreError::Database(msg) => BridgeError::Database { msg },
            CoreError::Config(msg) => BridgeError::Config { msg },
            CoreError::Keyring(msg) => BridgeError::Keyring { msg },
            CoreError::Sync(msg) => BridgeError::Sync { msg },
            CoreError::Io(msg) => BridgeError::Internal { msg },
            CoreError::Internal(msg) => BridgeError::Internal { msg },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each variant maps to its same-named counterpart in BOTH directions. The
    /// mapping is the FFI boundary the members list (core → bridge) and
    /// `invite_member` (bridge → core) cross separately, so a swapped pair would
    /// silently grant the wrong access — and a round-trip alone wouldn't catch it
    /// (a swap in both directions would still pass). Assert each direction
    /// against the expected variant instead.
    #[test]
    fn member_role_maps_to_the_same_variant_each_direction() {
        let pairs = [
            (MemberRole::Owner, BridgeMemberRole::Owner),
            (MemberRole::Member, BridgeMemberRole::Member),
            (MemberRole::Follower, BridgeMemberRole::Follower),
        ];
        for (core, bridge) in pairs {
            assert_eq!(BridgeMemberRole::from(core.clone()), bridge);
            assert_eq!(MemberRole::from(bridge), core);
        }
    }
}
