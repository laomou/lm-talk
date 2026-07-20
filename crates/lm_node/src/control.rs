use crate::{
    DEFAULT_MAX_DHT_RECORD_TTL_SECONDS, DEFAULT_MAX_DHT_RECORD_VALUE_BYTES,
    DEFAULT_MAX_DHT_RECORDS, DEFAULT_MAX_MAILBOX_ACK_ID_BYTES, DEFAULT_MAX_MAILBOX_ACK_IDS,
    DEFAULT_MAX_MAILBOX_TAKE_LIMIT, DhtRecord, DhtRecordKey, DhtRecordKind, DhtRecordRejectReason,
    DhtRpcRequest, KademliaNodeId, MailboxAckRejectReason, MailboxDelivery, MailboxDeliveryStatus,
    MailboxPushRejectReason, MailboxUserDeliverySummary, NativeNode, NodeMaintenanceStats,
    NodeStateSnapshot, NodeSyncPeerStatus, NodeSyncStatus, PublicPeerAnnounce,
    current_unix_timestamp, decode_identity_public_key_base64, dht_record_reject_reason, from_hex,
    prekey_low_one_time_prekeys, prekey_replenishment_required,
};
use lm_core::{LmError, MailboxMessage, PreKeyBundle, SignedOneTimePreKeyRecord, UserId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlRequest {
    pub method: String,
    pub path: String,
    pub body: String,
    pub headers: Vec<(String, String)>,
}

impl ControlRequest {
    pub fn header(&self, name: &str) -> Option<&str> {
        let name = name.to_ascii_lowercase();
        self.headers
            .iter()
            .find(|(header_name, _)| header_name.eq_ignore_ascii_case(&name))
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Debug, Serialize)]
struct ControlErrorBody<'a> {
    error_code: &'a str,
    message: String,
    recovery_hint: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlResponse {
    pub status: u16,
    pub content_type: String,
    pub body: String,
}

impl ControlResponse {
    pub fn json<T: Serialize>(status: u16, value: &T) -> Self {
        match serde_json::to_string(value) {
            Ok(body) => Self {
                status,
                content_type: "application/json".to_string(),
                body,
            },
            Err(err) => Self::text(500, format!("serialization error: {err}")),
        }
    }

    pub fn text(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            content_type: "text/plain; charset=utf-8".to_string(),
            body: body.into(),
        }
    }

    pub fn error(
        status: u16,
        error_code: &'static str,
        message: impl Into<String>,
        recovery_hint: &'static str,
    ) -> Self {
        Self::json(
            status,
            &ControlErrorBody {
                error_code,
                message: message.into(),
                recovery_hint,
            },
        )
    }

    pub fn to_http_string(&self) -> String {
        let reason = match self.status {
            200 => "OK",
            201 => "Created",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            413 => "Payload Too Large",
            429 => "Too Many Requests",
            500 => "Internal Server Error",
            _ => "OK",
        };
        format!(
            "HTTP/1.1 {} {}\r\ncontent-type: {}\r\naccess-control-allow-origin: *\r\naccess-control-allow-methods: GET,POST,OPTIONS\r\naccess-control-allow-headers: content-type\r\naccess-control-allow-private-network: true\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            self.status,
            reason,
            self.content_type,
            self.body.len(),
            self.body
        )
    }
}

#[derive(Debug, Deserialize)]
struct InsertPeerRequest {
    announce_text: String,
    identity_public_key: String,
}

#[derive(Debug, Deserialize)]
struct PushMailboxRequest {
    message_text: String,
    from_identity_public_key: String,
}

#[derive(Debug, Serialize)]
struct HealthResponse<'a> {
    status: &'a str,
    peer_id: &'a str,
    node_id: String,
    peers: usize,
    prekeys: usize,
    mailbox_deliveries: usize,
    mailbox_bytes: usize,
    mailbox_max_bytes: Option<u64>,
    mailbox_max_bytes_per_user: Option<u64>,
    mailbox_max_messages_per_user: Option<usize>,
    mailbox_take_limit: usize,
    mailbox_ack_max_ids: usize,
    mailbox_ack_id_max_bytes: usize,
    control_client_timeout_seconds: u64,
    control_peer_timeout_seconds: u64,
    control_peer_connect_timeout_seconds: u64,
    control_peer_response_max_bytes: usize,
    state_db_permissions_hardened: bool,
    libp2p_dht_rpc_request_max_bytes: u64,
    libp2p_dht_rpc_response_max_bytes: u64,
    libp2p_dht_rpc_max_concurrent_streams: usize,
    dht_peer_quarantine_consecutive_failures: u32,
    libp2p_dht_pending_incoming_max: u32,
    libp2p_dht_pending_outgoing_max: u32,
    libp2p_dht_established_incoming_max: u32,
    libp2p_dht_established_outgoing_max: u32,
    libp2p_dht_established_total_max: u32,
    libp2p_dht_established_per_peer_max: u32,
    dht_records: usize,
    dht_record_capacity: usize,
    dht_record_value_max_bytes: usize,
    dht_record_ttl_max_seconds: u64,
    maintenance: NodeMaintenanceStats,
    sync: NodeSyncStatus,
}

#[derive(Debug, Serialize)]
struct InsertPeerResponse {
    inserted: bool,
    peer_id: String,
    node_id: String,
    peers: usize,
}

#[derive(Debug, Serialize)]
struct ClosestPeersResponse {
    target: String,
    peers: Vec<PublicPeerAnnounce>,
}

#[derive(Debug, Serialize)]
struct MailboxPushResponse {
    stored: bool,
    delivery_id: String,
    to_user_id: String,
    pending: usize,
    pending_bytes: usize,
    max_bytes_per_user: Option<u64>,
}

#[derive(Debug, Serialize)]
struct MailboxTakeResponse {
    user_id: String,
    messages: Vec<MailboxDelivery>,
    returned: usize,
    pending: usize,
    pending_bytes: usize,
    max_bytes_per_user: Option<u64>,
    more: bool,
}

#[derive(Debug, Deserialize)]
struct MailboxAckRequest {
    user_id: String,
    delivery_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ResetSyncPeerHealthRequest {
    url: String,
}

#[derive(Debug, Serialize)]
struct ResetSyncPeerHealthResponse {
    url: String,
    reset: bool,
    status: Option<NodeSyncPeerStatus>,
}

#[derive(Debug, Serialize)]
struct MailboxAckResponse {
    user_id: String,
    removed: usize,
    pending: usize,
    pending_bytes: usize,
    max_bytes_per_user: Option<u64>,
}

#[derive(Debug, Serialize)]
struct MailboxStatusResponse {
    user_id: String,
    summary: MailboxUserDeliverySummary,
    max_bytes_per_user: Option<u64>,
    delivery: Option<MailboxDeliveryStatus>,
}

#[derive(Debug, Deserialize)]
struct PublishPreKeyRequest {
    prekey_bundle_text: String,
    #[serde(default)]
    signed_one_time_prekey_record_texts: Vec<String>,
}

#[derive(Debug, Serialize)]
struct PublishPreKeyResponse {
    stored: bool,
    user_id: String,
    prekey_bundles: usize,
    one_time_prekeys: usize,
    signed_one_time_prekey_records: usize,
    remaining_one_time_prekeys: usize,
    low_one_time_prekeys: bool,
    replenishment_required: bool,
    replenishment_actor: &'static str,
    node_generates_user_keys: bool,
}

#[derive(Debug, Serialize)]
struct GetPreKeyResponse {
    user_id: String,
    found: bool,
    prekey_bundle_text: Option<String>,
    selected_one_time_prekey_id: Option<u32>,
    selected_signed_one_time_prekey_record_text: Option<String>,
    consumed_one_time_prekey_ids: Vec<u32>,
    remaining_one_time_prekeys: Option<usize>,
    signed_one_time_prekey_records: Option<usize>,
    low_one_time_prekeys: bool,
    replenishment_required: bool,
    replenishment_actor: &'static str,
    node_generates_user_keys: bool,
}

#[derive(Debug, Serialize)]
struct PreKeyStatusResponse {
    user_id: String,
    found: bool,
    consumed_one_time_prekey_ids: Vec<u32>,
    remaining_one_time_prekeys: Option<usize>,
    signed_one_time_prekey_records: Option<usize>,
    low_one_time_prekeys: bool,
    replenishment_required: bool,
    replenishment_actor: &'static str,
    node_generates_user_keys: bool,
}

#[derive(Debug, Deserialize)]
struct StoreDhtRecordRequest {
    record: DhtRecord,
}

#[derive(Debug, Deserialize)]
struct DhtRpcControlRequest {
    request: DhtRpcRequest,
}

#[derive(Debug, Serialize)]
struct StoreDhtRecordResponse {
    stored: bool,
    inserted: bool,
    key: String,
    records: usize,
}

#[derive(Debug, Serialize)]
struct GetDhtRecordResponse {
    key: String,
    found: bool,
    record: Option<DhtRecord>,
}

#[derive(Debug, Serialize)]
struct ClosestDhtRecordsResponse {
    target: String,
    records: Vec<DhtRecord>,
}

#[derive(Debug, Serialize)]
struct DhtKeyResponse {
    kind: DhtRecordKind,
    value: String,
    key: String,
}

#[derive(Debug, Deserialize)]
struct ImportSnapshotRequest {
    snapshot: NodeStateSnapshot,
}

#[derive(Debug, Serialize)]
struct ImportSnapshotResponse {
    imported: bool,
    peers: usize,
    mailbox_deliveries: usize,
    prekey_bundles: usize,
    signed_one_time_prekey_records: usize,
    dht_records: usize,
}

impl NativeNode {
    pub fn handle_control_request(&mut self, request: ControlRequest) -> ControlResponse {
        self.prune_expired_records();
        match (request.method.as_str(), path_without_query(&request.path)) {
            ("OPTIONS", _) => ControlResponse::text(200, ""),
            ("GET", "/api/health") => ControlResponse::json(
                200,
                &HealthResponse {
                    status: "ok",
                    peer_id: &self.config.peer_id,
                    node_id: self.kademlia.local_id().to_hex(),
                    peers: self.kademlia.len(),
                    prekeys: self.prekeys.len(),
                    mailbox_deliveries: self.mailbox.total_pending(),
                    mailbox_bytes: self.mailbox.total_bytes(),
                    mailbox_max_bytes: self.config.max_mailbox_bytes,
                    mailbox_max_bytes_per_user: self.config.max_mailbox_bytes_per_user,
                    mailbox_max_messages_per_user: self.config.max_mailbox_messages_per_user,
                    mailbox_take_limit: DEFAULT_MAX_MAILBOX_TAKE_LIMIT,
                    mailbox_ack_max_ids: DEFAULT_MAX_MAILBOX_ACK_IDS,
                    mailbox_ack_id_max_bytes: DEFAULT_MAX_MAILBOX_ACK_ID_BYTES,
                    control_client_timeout_seconds: 10,
                    control_peer_timeout_seconds: 10,
                    control_peer_connect_timeout_seconds: 10,
                    control_peer_response_max_bytes: 8 * 1024 * 1024,
                    state_db_permissions_hardened: true,
                    libp2p_dht_rpc_request_max_bytes: 1024 * 1024,
                    libp2p_dht_rpc_response_max_bytes: 8 * 1024 * 1024,
                    libp2p_dht_rpc_max_concurrent_streams: 32,
                    dht_peer_quarantine_consecutive_failures: self
                        .config
                        .dht_peer_quarantine_consecutive_failures,
                    libp2p_dht_pending_incoming_max: 64,
                    libp2p_dht_pending_outgoing_max: 64,
                    libp2p_dht_established_incoming_max: 128,
                    libp2p_dht_established_outgoing_max: 128,
                    libp2p_dht_established_total_max: 256,
                    libp2p_dht_established_per_peer_max: 4,
                    dht_records: self.dht_records.len(),
                    dht_record_capacity: DEFAULT_MAX_DHT_RECORDS,
                    dht_record_value_max_bytes: DEFAULT_MAX_DHT_RECORD_VALUE_BYTES,
                    dht_record_ttl_max_seconds: DEFAULT_MAX_DHT_RECORD_TTL_SECONDS,
                    maintenance: self.maintenance.clone(),
                    sync: self.sync_status.clone(),
                },
            ),
            ("POST", "/api/announce") => self.handle_control_announce(&request.body),
            ("GET", "/api/peers/closest") => self.handle_control_closest(&request.path),
            ("POST", "/api/mailbox/push") => self.handle_control_mailbox_push(&request.body),
            ("GET", "/api/mailbox/take") => self.handle_control_mailbox_take(&request.path),
            ("GET", "/api/mailbox/status") => self.handle_control_mailbox_status(&request.path),
            ("POST", "/api/mailbox/ack") => self.handle_control_mailbox_ack(&request.body),
            ("POST", "/api/prekey/publish") => self.handle_control_prekey_publish(&request.body),
            ("GET", "/api/prekey/get") => self.handle_control_prekey_get(&request.path),
            ("GET", "/api/prekey/status") => self.handle_control_prekey_status(&request.path),
            ("POST", "/api/dht/record") => self.handle_control_dht_record_store(&request.body),
            ("GET", "/api/dht/key") => self.handle_control_dht_key(&request.path),
            ("GET", "/api/dht/record") => self.handle_control_dht_record_get(&request.path),
            ("GET", "/api/dht/closest") => self.handle_control_dht_closest(&request.path),
            ("POST", "/api/dht/rpc") => self.handle_control_dht_rpc(&request.body),
            ("GET", "/api/dht/replication-plan") => {
                self.handle_control_dht_replication_plan(&request.path)
            }
            ("GET", "/api/dht/routing-refresh-plan") => {
                self.handle_control_dht_routing_refresh_plan()
            }
            ("GET", "/api/sync/snapshot") => ControlResponse::json(200, &self.to_state_snapshot()),
            ("GET", "/api/sync/status") => ControlResponse::json(200, &self.sync_status),
            ("POST", "/api/sync/peer/reset") => self.handle_control_sync_peer_reset(&request.body),
            ("POST", "/api/sync/import") => self.handle_control_sync_import(&request.body),
            (
                _,
                "/api/health"
                | "/api/announce"
                | "/api/peers/closest"
                | "/api/mailbox/push"
                | "/api/mailbox/take"
                | "/api/mailbox/status"
                | "/api/mailbox/ack"
                | "/api/prekey/publish"
                | "/api/prekey/get"
                | "/api/prekey/status"
                | "/api/dht/key"
                | "/api/dht/record"
                | "/api/dht/closest"
                | "/api/dht/rpc"
                | "/api/dht/find-value"
                | "/api/dht/maintenance"
                | "/api/dht/replicate"
                | "/api/dht/replication-plan"
                | "/api/dht/routing-refresh-plan"
                | "/api/sync/snapshot"
                | "/api/sync/status"
                | "/api/sync/peer/reset"
                | "/api/sync/import",
            ) => ControlResponse::error(
                405,
                "METHOD_NOT_ALLOWED",
                "method not allowed",
                "请检查 HTTP 方法是否匹配该 API。",
            ),
            _ => ControlResponse::error(
                404,
                "API_NOT_FOUND",
                "not found",
                "请确认 lm_node 版本和 API 路径。",
            ),
        }
    }

    fn handle_control_announce(&mut self, body: &str) -> ControlResponse {
        let req: InsertPeerRequest = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, format!("invalid json: {e}")),
        };
        let announce = match PublicPeerAnnounce::from_export_text(req.announce_text.trim()) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, e.to_string()),
        };
        let public_key = match decode_identity_public_key_base64(&req.identity_public_key) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, e.to_string()),
        };
        if let Err(e) = self
            .routing_table
            .insert_verified(announce.clone(), &public_key)
        {
            return ControlResponse::text(400, e.to_string());
        }
        if let Err(e) = self.kademlia.insert_verified(announce.clone(), &public_key) {
            return ControlResponse::text(400, e.to_string());
        }
        ControlResponse::json(
            201,
            &InsertPeerResponse {
                inserted: true,
                peer_id: announce.peer_id.clone(),
                node_id: KademliaNodeId::from_peer_id(&announce.peer_id).to_hex(),
                peers: self.kademlia.len(),
            },
        )
    }

    fn handle_control_closest(&self, path: &str) -> ControlResponse {
        let query = query_params(path);
        let Some(target) = query.get("target") else {
            return ControlResponse::text(400, "missing target");
        };
        let limit = query
            .get("limit")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(8)
            .clamp(1, 64);
        let peers = self
            .kademlia
            .closest(KademliaNodeId::from_peer_id(target), limit)
            .into_iter()
            .map(|p| p.announce)
            .collect();
        ControlResponse::json(
            200,
            &ClosestPeersResponse {
                target: target.to_string(),
                peers,
            },
        )
    }

    fn handle_control_mailbox_push(&mut self, body: &str) -> ControlResponse {
        let req: PushMailboxRequest = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(e) => {
                self.record_mailbox_push_reject(MailboxPushRejectReason::InvalidJson);
                return ControlResponse::text(400, format!("invalid json: {e}"));
            }
        };
        let message = match MailboxMessage::from_export_text(req.message_text.trim()) {
            Ok(v) => v,
            Err(e) => {
                self.record_mailbox_push_reject(MailboxPushRejectReason::InvalidMessageFormat);
                return ControlResponse::text(400, e.to_string());
            }
        };
        let public_key = match decode_identity_public_key_base64(&req.from_identity_public_key) {
            Ok(v) => v,
            Err(e) => {
                self.record_mailbox_push_reject(MailboxPushRejectReason::InvalidIdentityPublicKey);
                return ControlResponse::text(400, e.to_string());
            }
        };
        if let Err(e) = message.verify(&public_key) {
            self.record_mailbox_push_reject(MailboxPushRejectReason::from(e.clone()));
            return ControlResponse::text(400, e.to_string());
        }
        let now = current_unix_timestamp();
        let global_rate_limit = self.config.mailbox_global_rate_limit();
        let sender_rate_limit = self.config.mailbox_sender_rate_limit();
        if !self
            .mailbox_global_rate_limiter
            .allows(now, global_rate_limit)
        {
            self.record_mailbox_push_reject(MailboxPushRejectReason::GlobalRateLimited);
            return ControlResponse::error(
                429,
                "MAILBOX_RATE_LIMITED",
                "mailbox global rate limit exceeded",
                "请求过于频繁，请稍后重试；必要时调整节点 mailbox 全局限流。",
            );
        }
        if !self
            .mailbox_sender_rate_limiter
            .allows(&message.from_user_id, now, sender_rate_limit)
        {
            self.record_mailbox_push_reject(MailboxPushRejectReason::SenderRateLimited);
            return ControlResponse::error(
                429,
                "MAILBOX_RATE_LIMITED",
                "mailbox sender rate limit exceeded",
                "该发送者请求过于频繁，请稍后重试；必要时调整节点按发送者限流。",
            );
        }
        let delivery_id = match self.mailbox.push_verified_with_limits(
            message.clone(),
            &public_key,
            self.config.max_mailbox_bytes,
            self.config.max_mailbox_bytes_per_user,
            self.config.max_mailbox_messages_per_user,
            self.config.max_message_ttl_seconds,
        ) {
            Ok(delivery_id) => delivery_id,
            Err(e) => {
                self.record_mailbox_push_reject(MailboxPushRejectReason::from(e.clone()));
                if e == LmError::PayloadTooLarge {
                    return ControlResponse::error(
                        413,
                        "MAILBOX_QUOTA_EXCEEDED",
                        e.to_string(),
                        "Mailbox 内容超过节点大小或配额限制，请缩小内容、清理收件箱或调整节点配额。",
                    );
                }
                return ControlResponse::text(400, e.to_string());
            }
        };
        self.mailbox_global_rate_limiter
            .record(now, global_rate_limit);
        self.mailbox_sender_rate_limiter
            .record(&message.from_user_id, now, sender_rate_limit);
        ControlResponse::json(
            201,
            &MailboxPushResponse {
                stored: true,
                delivery_id,
                to_user_id: message.to_user_id.to_string(),
                pending: self.mailbox.pending_for(&message.to_user_id),
                pending_bytes: self.mailbox.bytes_for(&message.to_user_id),
                max_bytes_per_user: self.config.max_mailbox_bytes_per_user,
            },
        )
    }

    fn handle_control_mailbox_take(&mut self, path: &str) -> ControlResponse {
        let query = query_params(path);
        let Some(user_id) = query.get("user_id") else {
            return ControlResponse::text(400, "missing user_id");
        };
        let user_id = match UserId::from_raw(user_id.to_string()) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, e.to_string()),
        };
        let requested_limit = query
            .get("limit")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(DEFAULT_MAX_MAILBOX_TAKE_LIMIT);
        let limit = requested_limit.clamp(1, DEFAULT_MAX_MAILBOX_TAKE_LIMIT);
        let messages = self.mailbox.take_for_limited(&user_id, limit);
        let returned = messages.len();
        let pending = self.mailbox.pending_for(&user_id);
        ControlResponse::json(
            200,
            &MailboxTakeResponse {
                user_id: user_id.to_string(),
                messages,
                returned,
                pending,
                pending_bytes: self.mailbox.bytes_for(&user_id),
                max_bytes_per_user: self.config.max_mailbox_bytes_per_user,
                more: pending > returned,
            },
        )
    }

    fn handle_control_mailbox_status(&mut self, path: &str) -> ControlResponse {
        let query = query_params(path);
        let Some(user_id) = query.get("user_id") else {
            return ControlResponse::text(400, "missing user_id");
        };
        let user_id = match UserId::from_raw(user_id.to_string()) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, e.to_string()),
        };
        let delivery = match query.get("delivery_id") {
            Some(delivery_id) => {
                if delivery_id.trim().is_empty() {
                    return ControlResponse::text(400, "mailbox delivery id is empty");
                }
                if delivery_id.len() > DEFAULT_MAX_MAILBOX_ACK_ID_BYTES {
                    return ControlResponse::text(413, "mailbox delivery id too large");
                }
                Some(self.mailbox.delivery_status(&user_id, delivery_id))
            }
            None => None,
        };
        ControlResponse::json(
            200,
            &MailboxStatusResponse {
                user_id: user_id.to_string(),
                summary: self.mailbox.delivery_summary_for(&user_id),
                max_bytes_per_user: self.config.max_mailbox_bytes_per_user,
                delivery,
            },
        )
    }

    fn handle_control_mailbox_ack(&mut self, body: &str) -> ControlResponse {
        let req: MailboxAckRequest = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(e) => {
                self.record_mailbox_ack_reject(MailboxAckRejectReason::InvalidJson);
                return ControlResponse::text(400, format!("invalid json: {e}"));
            }
        };
        let user_id = match UserId::from_raw(req.user_id) {
            Ok(v) => v,
            Err(e) => {
                self.record_mailbox_ack_reject(MailboxAckRejectReason::InvalidUserId);
                return ControlResponse::text(400, e.to_string());
            }
        };
        if req.delivery_ids.len() > DEFAULT_MAX_MAILBOX_ACK_IDS {
            self.record_mailbox_ack_reject(MailboxAckRejectReason::TooManyIds);
            return ControlResponse::text(413, "too many mailbox ack delivery ids");
        }
        if req.delivery_ids.iter().any(|id| id.trim().is_empty()) {
            self.record_mailbox_ack_reject(MailboxAckRejectReason::EmptyId);
            return ControlResponse::text(400, "mailbox ack delivery id is empty");
        }
        if req
            .delivery_ids
            .iter()
            .any(|id| id.len() > DEFAULT_MAX_MAILBOX_ACK_ID_BYTES)
        {
            self.record_mailbox_ack_reject(MailboxAckRejectReason::IdTooLarge);
            return ControlResponse::text(413, "mailbox ack delivery id too large");
        }
        let removed = self.mailbox.ack(&user_id, &req.delivery_ids);
        ControlResponse::json(
            200,
            &MailboxAckResponse {
                user_id: user_id.to_string(),
                removed,
                pending: self.mailbox.pending_for(&user_id),
                pending_bytes: self.mailbox.bytes_for(&user_id),
                max_bytes_per_user: self.config.max_mailbox_bytes_per_user,
            },
        )
    }

    fn handle_control_prekey_publish(&mut self, body: &str) -> ControlResponse {
        let req: PublishPreKeyRequest = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, format!("invalid json: {e}")),
        };
        let bundle = match PreKeyBundle::from_export_text(req.prekey_bundle_text.trim()) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, e.to_string()),
        };
        let signed_one_time_prekey_records = match req
            .signed_one_time_prekey_record_texts
            .iter()
            .map(|text| SignedOneTimePreKeyRecord::from_export_text(text.trim()))
            .collect::<lm_core::Result<Vec<_>>>()
        {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, e.to_string()),
        };
        let user_id = bundle.user_id.clone();
        let one_time_prekeys = if signed_one_time_prekey_records.is_empty() {
            bundle.one_time_prekeys.len()
        } else {
            signed_one_time_prekey_records.len()
        };
        if let Err(e) = self
            .prekeys
            .publish_verified_with_signed_one_time_prekey_records(
                bundle,
                signed_one_time_prekey_records,
            )
        {
            return ControlResponse::text(400, e.to_string());
        }
        let signed_one_time_prekey_records = self
            .prekeys
            .signed_one_time_prekey_records_for(&user_id)
            .len();
        let remaining = self
            .prekeys
            .remaining_one_time_prekeys_for(&user_id)
            .unwrap_or(0);
        let remaining_status = Some(remaining);
        let low_one_time_prekeys = prekey_low_one_time_prekeys(remaining_status);
        let replenishment_required = prekey_replenishment_required(remaining_status);
        ControlResponse::json(
            201,
            &PublishPreKeyResponse {
                stored: true,
                user_id: user_id.to_string(),
                prekey_bundles: self.prekeys.len(),
                one_time_prekeys,
                signed_one_time_prekey_records,
                remaining_one_time_prekeys: remaining,
                low_one_time_prekeys,
                replenishment_required,
                replenishment_actor: "client",
                node_generates_user_keys: false,
            },
        )
    }

    fn handle_control_prekey_get(&mut self, path: &str) -> ControlResponse {
        let query = query_params(path);
        let Some(user_id) = query.get("user_id") else {
            return ControlResponse::text(400, "missing user_id");
        };
        let user_id = match UserId::from_raw(user_id.to_string()) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, e.to_string()),
        };
        let consume = query
            .get("consume")
            .map(|v| matches!(v.as_str(), "1" | "true" | "yes"))
            .unwrap_or(false);
        let selected = self
            .prekeys
            .take_for_with_selected_one_time_prekey_material(&user_id, consume);
        let (bundle_text, selected_one_time_prekey_id, selected_record_text) = match selected {
            Some((bundle, selected_id, selected_record)) => {
                let bundle_text = match bundle.to_export_text() {
                    Ok(text) => Some(text),
                    Err(e) => return ControlResponse::text(400, e.to_string()),
                };
                let selected_record_text = match selected_record {
                    Some(record) => match record.to_export_text() {
                        Ok(text) => Some(text),
                        Err(e) => return ControlResponse::text(400, e.to_string()),
                    },
                    None => None,
                };
                (bundle_text, selected_id, selected_record_text)
            }
            None => (None, None, None),
        };
        let remaining = self.prekeys.remaining_one_time_prekeys_for(&user_id);
        let signed_one_time_prekey_records = self.prekeys.published_one_time_prekeys_for(&user_id);
        let low_one_time_prekeys = prekey_low_one_time_prekeys(remaining);
        let replenishment_required = prekey_replenishment_required(remaining);
        ControlResponse::json(
            200,
            &GetPreKeyResponse {
                user_id: user_id.to_string(),
                found: bundle_text.is_some(),
                prekey_bundle_text: bundle_text,
                selected_one_time_prekey_id,
                selected_signed_one_time_prekey_record_text: selected_record_text,
                consumed_one_time_prekey_ids: self.prekeys.consumed_for(&user_id),
                remaining_one_time_prekeys: remaining,
                signed_one_time_prekey_records,
                low_one_time_prekeys,
                replenishment_required,
                replenishment_actor: "client",
                node_generates_user_keys: false,
            },
        )
    }

    fn handle_control_prekey_status(&mut self, path: &str) -> ControlResponse {
        let query = query_params(path);
        let Some(user_id) = query.get("user_id") else {
            return ControlResponse::text(400, "missing user_id");
        };
        let user_id = match UserId::from_raw(user_id.to_string()) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, e.to_string()),
        };
        self.prekeys.prune_expired(current_unix_timestamp());
        let remaining = self.prekeys.remaining_one_time_prekeys_for(&user_id);
        let signed_one_time_prekey_records = self.prekeys.published_one_time_prekeys_for(&user_id);
        let low_one_time_prekeys = prekey_low_one_time_prekeys(remaining);
        let replenishment_required = prekey_replenishment_required(remaining);
        ControlResponse::json(
            200,
            &PreKeyStatusResponse {
                user_id: user_id.to_string(),
                found: remaining.is_some(),
                consumed_one_time_prekey_ids: self.prekeys.consumed_for(&user_id),
                remaining_one_time_prekeys: remaining,
                signed_one_time_prekey_records,
                low_one_time_prekeys,
                replenishment_required,
                replenishment_actor: "client",
                node_generates_user_keys: false,
            },
        )
    }

    fn handle_control_dht_replication_plan(&mut self, path: &str) -> ControlResponse {
        let query = query_params(path);
        let replication_factor = query
            .get("replication_factor")
            .or_else(|| query.get("factor"))
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(8)
            .clamp(1, 64);
        ControlResponse::json(200, &self.plan_dht_replication(replication_factor))
    }

    fn handle_control_dht_routing_refresh_plan(&self) -> ControlResponse {
        ControlResponse::json(200, &self.plan_dht_routing_refresh())
    }

    fn handle_control_dht_rpc(&mut self, body: &str) -> ControlResponse {
        let req: DhtRpcControlRequest = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, format!("invalid json: {e}")),
        };
        ControlResponse::json(200, &self.handle_dht_rpc(req.request))
    }

    fn handle_control_dht_record_store(&mut self, body: &str) -> ControlResponse {
        let req: StoreDhtRecordRequest = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(e) => {
                self.record_dht_record_reject(DhtRecordRejectReason::InvalidJson);
                return ControlResponse::text(400, format!("invalid json: {e}"));
            }
        };
        let now = current_unix_timestamp();
        if let Some(reason) = dht_record_reject_reason(&req.record, now) {
            self.record_dht_record_reject(reason);
            return match reason {
                DhtRecordRejectReason::PayloadTooLarge => ControlResponse::error(
                    413,
                    "DHT_RECORD_TOO_LARGE",
                    "dht record value too large",
                    "DHT record 超过节点大小限制，请缩小记录内容。",
                ),
                DhtRecordRejectReason::Expired => ControlResponse::error(
                    400,
                    "DHT_RECORD_EXPIRED",
                    "dht record expired",
                    "请重新生成并发布未过期的 DHT record。",
                ),
                DhtRecordRejectReason::TtlTooLong => ControlResponse::error(
                    400,
                    "DHT_RECORD_TTL_TOO_LONG",
                    "dht record ttl too long",
                    "请降低 DHT record TTL 后重试。",
                ),
                _ => ControlResponse::error(
                    400,
                    "DHT_RECORD_INVALID",
                    "invalid dht record",
                    "请检查 DHT record 的 kind、key、签名和 value。",
                ),
            };
        }
        let key = req.record.key.to_hex();
        let inserted = self.dht_records.store(req.record);
        ControlResponse::json(
            201,
            &StoreDhtRecordResponse {
                stored: true,
                inserted,
                key,
                records: self.dht_records.len(),
            },
        )
    }

    fn handle_control_dht_key(&mut self, path: &str) -> ControlResponse {
        let query = query_params(path);
        let Some(kind) = query.get("kind") else {
            return ControlResponse::text(400, "missing kind");
        };
        let Some(value) = query.get("value") else {
            return ControlResponse::text(400, "missing value");
        };
        let normalized_kind = kind.trim().to_ascii_lowercase();
        let value = value.trim();
        if value.is_empty() {
            return ControlResponse::text(400, "missing value");
        }
        let (kind, key) = match normalized_kind.as_str() {
            "public-peer" | "public_peer" | "peer" => (
                DhtRecordKind::PublicPeer,
                DhtRecordKey::for_public_peer(value),
            ),
            "prekey" | "pre-key" => {
                let user_id = match UserId::from_raw(value.to_string()) {
                    Ok(user_id) => user_id,
                    Err(err) => return ControlResponse::text(400, err.to_string()),
                };
                (DhtRecordKind::PreKey, DhtRecordKey::for_prekey(&user_id))
            }
            "contact-card" | "contact_card" | "contact" => {
                let user_id = match UserId::from_raw(value.to_string()) {
                    Ok(user_id) => user_id,
                    Err(err) => return ControlResponse::text(400, err.to_string()),
                };
                (
                    DhtRecordKind::ContactCard,
                    DhtRecordKey::for_contact_card(&user_id),
                )
            }
            "mailbox-hint" | "mailbox_hint" | "mailbox" => {
                let user_id = match UserId::from_raw(value.to_string()) {
                    Ok(user_id) => user_id,
                    Err(err) => return ControlResponse::text(400, err.to_string()),
                };
                (
                    DhtRecordKind::MailboxHint,
                    DhtRecordKey::for_mailbox_hint(&user_id),
                )
            }
            _ => {
                return ControlResponse::text(
                    400,
                    "unsupported dht key kind; expected public-peer, prekey, mailbox-hint, or contact-card",
                );
            }
        };
        ControlResponse::json(
            200,
            &DhtKeyResponse {
                kind,
                value: value.to_string(),
                key: key.to_hex(),
            },
        )
    }

    fn handle_control_dht_record_get(&mut self, path: &str) -> ControlResponse {
        let query = query_params(path);
        let Some(key) = query.get("key") else {
            return ControlResponse::text(400, "missing key");
        };
        let key = match DhtRecordKey::from_hex(key) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, e.to_string()),
        };
        let record = self.dht_records.find_value(&key);
        ControlResponse::json(
            200,
            &GetDhtRecordResponse {
                key: key.to_hex(),
                found: record.is_some(),
                record,
            },
        )
    }

    fn handle_control_dht_closest(&mut self, path: &str) -> ControlResponse {
        let query = query_params(path);
        let Some(target) = query.get("target") else {
            return ControlResponse::text(400, "missing target");
        };
        let target = match DhtRecordKey::from_hex(target) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, e.to_string()),
        };
        let limit = query
            .get("limit")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(8)
            .clamp(1, 64);
        let records = self.dht_records.closest_records(target, limit);
        ControlResponse::json(
            200,
            &ClosestDhtRecordsResponse {
                target: target.to_hex(),
                records,
            },
        )
    }

    fn handle_control_sync_peer_reset(&mut self, body: &str) -> ControlResponse {
        let req: ResetSyncPeerHealthRequest = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, format!("invalid json: {e}")),
        };
        let url = req.url.trim().trim_end_matches('/').to_string();
        if url.is_empty() {
            return ControlResponse::text(400, "missing sync peer url");
        }
        let reset = self.sync_status.reset_peer_health(&url);
        ControlResponse::json(
            200,
            &ResetSyncPeerHealthResponse {
                status: self.sync_status.peers.get(&url).cloned(),
                url,
                reset,
            },
        )
    }

    fn handle_control_sync_import(&mut self, body: &str) -> ControlResponse {
        let req: ImportSnapshotRequest = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, format!("invalid json: {e}")),
        };
        let stats = self.merge_snapshot(req.snapshot);
        ControlResponse::json(
            200,
            &ImportSnapshotResponse {
                imported: true,
                peers: stats.peers,
                mailbox_deliveries: stats.mailbox_deliveries,
                prekey_bundles: stats.prekey_bundles,
                signed_one_time_prekey_records: stats.signed_one_time_prekey_records,
                dht_records: stats.dht_records,
            },
        )
    }
}

pub(crate) fn path_without_query(path: &str) -> &str {
    path.split_once('?').map(|(h, _)| h).unwrap_or(path)
}

pub(crate) fn query_params(path: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Some((_, query)) = path.split_once('?') else {
        return out;
    };
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        out.insert(percent_decode(key), percent_decode(value));
    }
    out
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut idx = 0;
    while idx < bytes.len() {
        match bytes[idx] {
            b'+' => {
                out.push(b' ');
                idx += 1;
            }
            b'%' if idx + 2 < bytes.len() => {
                if let (Some(hi), Some(lo)) = (from_hex(bytes[idx + 1]), from_hex(bytes[idx + 2])) {
                    out.push((hi << 4) | lo);
                    idx += 3;
                } else {
                    out.push(bytes[idx]);
                    idx += 1;
                }
            }
            byte => {
                out.push(byte);
                idx += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}
