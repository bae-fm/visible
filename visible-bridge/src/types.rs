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

    /// A node with every field set, so a conversion test can assert each one
    /// passes through. `quantity` drives `quantity_badge`, so each test that
    /// cares overrides it.
    fn sample_node() -> Node {
        Node {
            id: "node-1".into(),
            parent_id: Some("parent-1".into()),
            name: Some("Lamp".into()),
            position: 3,
            image_id: Some("img-1".into()),
            quantity: None,
            notes: Some("a note".into()),
            value_cents: Some(1299),
            acquired_at: Some("2024-01-02".into()),
            serial: Some("SN-1".into()),
            barcode: Some("012345678905".into()),
        }
    }

    #[test]
    fn node_passes_its_fields_through_and_drops_position() {
        let bridge = BridgeNode::from(sample_node());
        assert_eq!(bridge.id, "node-1");
        assert_eq!(bridge.parent_id.as_deref(), Some("parent-1"));
        assert_eq!(bridge.name.as_deref(), Some("Lamp"));
        assert_eq!(bridge.image_id.as_deref(), Some("img-1"));
    }

    /// The root house is parentless and untitled photo-first children carry no
    /// name; both surface as `None` across the boundary.
    #[test]
    fn node_carries_absent_parent_and_name_as_none() {
        let node = Node {
            parent_id: None,
            name: None,
            image_id: None,
            ..sample_node()
        };
        let bridge = BridgeNode::from(node);
        assert!(bridge.parent_id.is_none());
        assert!(bridge.name.is_none());
        assert!(bridge.image_id.is_none());
    }

    /// The card shows `×N` only for a node that stands for more than one thing.
    /// A quantity of `None` or `1` is a single item with no badge; greater than
    /// one renders the count. The conversion calls [`Node::quantity_badge`], so
    /// this pins the boundary's view of that derivation across the three cases.
    #[test]
    fn node_quantity_badge_only_for_more_than_one() {
        let badge = |quantity| {
            BridgeNode::from(Node {
                quantity,
                ..sample_node()
            })
            .quantity_badge
        };
        assert_eq!(badge(None), None);
        assert_eq!(badge(Some(1)), None);
        assert_eq!(badge(Some(4)), Some("×4".into()));
    }

    #[test]
    fn node_detail_carries_the_node_attributes_and_tags() {
        let detail = NodeDetail {
            node: Node {
                quantity: Some(2),
                ..sample_node()
            },
            tags: vec!["fragile".into(), "blue".into()],
        };
        let bridge = BridgeNodeDetail::from(detail);
        assert_eq!(bridge.id, "node-1");
        assert_eq!(bridge.parent_id.as_deref(), Some("parent-1"));
        assert_eq!(bridge.name.as_deref(), Some("Lamp"));
        assert_eq!(bridge.image_id.as_deref(), Some("img-1"));
        assert_eq!(bridge.quantity, Some(2));
        assert_eq!(bridge.notes.as_deref(), Some("a note"));
        assert_eq!(bridge.value_cents, Some(1299));
        assert_eq!(bridge.acquired_at.as_deref(), Some("2024-01-02"));
        assert_eq!(bridge.serial.as_deref(), Some("SN-1"));
        assert_eq!(bridge.barcode.as_deref(), Some("012345678905"));
        assert_eq!(bridge.tags, vec!["fragile".to_string(), "blue".into()]);
    }

    /// A fresh node carries no attributes and no tags; every optional attribute
    /// is `None` and the tag list is empty across the boundary.
    #[test]
    fn node_detail_with_no_attributes_or_tags() {
        let detail = NodeDetail {
            node: Node {
                name: None,
                quantity: None,
                notes: None,
                value_cents: None,
                acquired_at: None,
                serial: None,
                barcode: None,
                ..sample_node()
            },
            tags: vec![],
        };
        let bridge = BridgeNodeDetail::from(detail);
        assert!(bridge.quantity.is_none());
        assert!(bridge.notes.is_none());
        assert!(bridge.value_cents.is_none());
        assert!(bridge.acquired_at.is_none());
        assert!(bridge.serial.is_none());
        assert!(bridge.barcode.is_none());
        assert!(bridge.tags.is_empty());
    }

    #[test]
    fn search_hit_carries_the_match_path_and_label() {
        let hit = SearchHit {
            node: sample_node(),
            path: vec![
                Node {
                    id: "root".into(),
                    parent_id: None,
                    name: Some("Home".into()),
                    ..sample_node()
                },
                sample_node(),
            ],
            path_label: "Home › Living Room".into(),
        };
        let bridge = BridgeSearchResult::from(hit);
        assert_eq!(bridge.node.id, "node-1");
        assert_eq!(bridge.path.len(), 2);
        assert_eq!(bridge.path[0].id, "root");
        assert!(bridge.path[0].parent_id.is_none());
        assert_eq!(bridge.path[1].id, "node-1");
        assert_eq!(bridge.path_label, "Home › Living Room");
    }

    #[test]
    fn member_carries_short_pubkey_role_and_is_self() {
        let member = Member {
            pubkey: "deadbeef".repeat(8),
            short_pubkey: "deadbeef…deadbeef".into(),
            role: MemberRole::Follower,
            is_self: true,
        };
        let bridge = BridgeMember::from(member);
        assert_eq!(bridge.pubkey, "deadbeef".repeat(8));
        assert_eq!(bridge.short_pubkey, "deadbeef…deadbeef");
        assert_eq!(bridge.role, BridgeMemberRole::Follower);
        assert!(bridge.is_self);
    }

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

    #[test]
    fn sync_status_carries_configured_and_ready() {
        let bridge = BridgeSyncStatus::from(SyncStatusInfo {
            configured: true,
            ready: false,
        });
        assert!(bridge.configured);
        assert!(!bridge.ready);
    }

    #[test]
    fn outbox_snapshot_carries_the_pending_delete_count() {
        let bridge = BridgeOutboxSnapshot::from(OutboxSnapshot { pending_deletes: 5 });
        assert_eq!(bridge.pending_deletes, 5);
    }

    /// The settings form's S3 fields map into core's config shape. The optional
    /// endpoint and key prefix pass through as-is — the form has already mapped a
    /// blank box to `None` before constructing the bridge config.
    #[test]
    fn s3_config_passes_required_and_optional_fields() {
        let config = BridgeS3Config {
            bucket: "my-bucket".into(),
            region: "us-east-1".into(),
            endpoint: Some("https://s3.example.com".into()),
            key_prefix: Some("home/".into()),
            access_key: "AKIA".into(),
            secret_key: "secret".into(),
        };
        let core = S3ConfigData::from(config);
        assert_eq!(core.bucket, "my-bucket");
        assert_eq!(core.region, "us-east-1");
        assert_eq!(core.endpoint.as_deref(), Some("https://s3.example.com"));
        assert_eq!(core.key_prefix.as_deref(), Some("home/"));
        assert_eq!(core.access_key, "AKIA");
        assert_eq!(core.secret_key, "secret");
    }

    /// AWS S3 needs no endpoint and a bucket not shared across libraries needs no
    /// prefix; both arrive as `None` and pass through absent.
    #[test]
    fn s3_config_without_endpoint_or_prefix() {
        let core = S3ConfigData::from(BridgeS3Config {
            bucket: "my-bucket".into(),
            region: "us-east-1".into(),
            endpoint: None,
            key_prefix: None,
            access_key: "AKIA".into(),
            secret_key: "secret".into(),
        });
        assert!(core.endpoint.is_none());
        assert!(core.key_prefix.is_none());
    }

    /// Each core error variant maps to the bridge error case the generated
    /// Swift/Kotlin switch on. `Io` has no bridge case of its own — it folds into
    /// `Internal` — so the two `Internal` sources are asserted separately. The
    /// message rides along on every variant.
    #[test]
    fn core_error_maps_each_variant_to_its_bridge_case() {
        assert!(matches!(
            BridgeError::from(CoreError::NotFound("missing".into())),
            BridgeError::NotFound { msg } if msg == "missing"
        ));
        assert!(matches!(
            BridgeError::from(CoreError::Database("db".into())),
            BridgeError::Database { msg } if msg == "db"
        ));
        assert!(matches!(
            BridgeError::from(CoreError::Config("cfg".into())),
            BridgeError::Config { msg } if msg == "cfg"
        ));
        assert!(matches!(
            BridgeError::from(CoreError::Keyring("key".into())),
            BridgeError::Keyring { msg } if msg == "key"
        ));
        assert!(matches!(
            BridgeError::from(CoreError::Sync("sync".into())),
            BridgeError::Sync { msg } if msg == "sync"
        ));
        assert!(matches!(
            BridgeError::from(CoreError::Internal("internal".into())),
            BridgeError::Internal { msg } if msg == "internal"
        ));
        // Io has no bridge case of its own; it folds into Internal.
        assert!(matches!(
            BridgeError::from(CoreError::Io("io".into())),
            BridgeError::Internal { msg } if msg == "io"
        ));
    }
}
