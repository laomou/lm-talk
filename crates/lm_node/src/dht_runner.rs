use super::*;
use libp2p::{Multiaddr, PeerId};

pub(super) trait DhtTransport: Sync {
    fn send_dht_rpc(
        &self,
        peer: &SyncPeerConfig,
        request: &DhtRpcRequest,
    ) -> Result<DhtRpcResponse, Box<dyn std::error::Error>>;
}

#[derive(Debug, Clone, Copy)]
pub(super) struct HttpControlDhtTransport;

impl DhtTransport for HttpControlDhtTransport {
    fn send_dht_rpc(
        &self,
        peer: &SyncPeerConfig,
        request: &DhtRpcRequest,
    ) -> Result<DhtRpcResponse, Box<dyn std::error::Error>> {
        let body = serde_json::json!({ "request": request }).to_string();
        let response = http_control_request(peer, "POST", "/api/dht/rpc", &body)?;
        let response = serde_json::from_str(&response)?;
        validate_dht_rpc_response(request, response)
    }
}

pub(super) fn dht_rpc_request_id(request: &DhtRpcRequest) -> &str {
    match request {
        DhtRpcRequest::FindNode { request_id, .. }
        | DhtRpcRequest::FindValue { request_id, .. }
        | DhtRpcRequest::StoreRecord { request_id, .. } => request_id,
    }
}

pub(super) fn dht_rpc_response_id(response: &DhtRpcResponse) -> &str {
    match response {
        DhtRpcResponse::Nodes { request_id, .. }
        | DhtRpcResponse::Value { request_id, .. }
        | DhtRpcResponse::StoreResult { request_id, .. }
        | DhtRpcResponse::Error { request_id, .. } => request_id,
    }
}

pub(super) fn validate_dht_rpc_response(
    request: &DhtRpcRequest,
    response: DhtRpcResponse,
) -> Result<DhtRpcResponse, Box<dyn std::error::Error>> {
    let expected = dht_rpc_request_id(request);
    let actual = dht_rpc_response_id(&response);
    if actual != expected {
        return Err(format!(
            "dht rpc response request_id mismatch: expected {expected}, got {actual}"
        )
        .into());
    }
    if let DhtRpcResponse::Error { message, .. } = response {
        return Err(format!("dht rpc error response: {message}").into());
    }
    Ok(response)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DhtRunnerConfig {
    pub(crate) replication_factor: usize,
    pub(crate) routing_refresh_limit: usize,
    pub(crate) routing_refresh_max_targets: usize,
    pub(crate) transport: DhtTransportKind,
    pub(crate) peer_quarantine_consecutive_failures: u32,
}

impl Default for DhtRunnerConfig {
    fn default() -> Self {
        Self {
            replication_factor: 3,
            routing_refresh_limit: 8,
            routing_refresh_max_targets: 8,
            transport: DhtTransportKind::HttpControl,
            peer_quarantine_consecutive_failures: DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DhtTransportKind {
    HttpControl,
    Libp2p,
}

impl DhtTransportKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::HttpControl => "http-control",
            Self::Libp2p => "libp2p",
        }
    }
}

pub(super) fn parse_dht_transport_kind(value: &str) -> Result<DhtTransportKind, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "http" | "http-control" | "control" => Ok(DhtTransportKind::HttpControl),
        "libp2p" => Ok(DhtTransportKind::Libp2p),
        other => Err(format!(
            "unsupported dht transport {other:?}; expected http-control or libp2p"
        )),
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub(super) struct DhtReplicationRunStats {
    pub(crate) records: usize,
    pub(crate) attempts: usize,
    pub(crate) successes: usize,
    pub(crate) failures: usize,
    pub(crate) peers_quarantined: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub(super) struct DhtRoutingRefreshRunStats {
    pub(crate) targets: usize,
    pub(crate) attempts: usize,
    pub(crate) successes: usize,
    pub(crate) failures: usize,
    pub(crate) nodes_returned: usize,
    pub(crate) nodes_merged: usize,
    pub(crate) nodes_rejected_non_closer: usize,
    pub(crate) nodes_rejected_duplicate: usize,
    pub(crate) peers_quarantined: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub(super) struct DhtFindValueRunStats {
    pub(crate) attempts: usize,
    pub(crate) successes: usize,
    pub(crate) failures: usize,
    pub(crate) found_records: usize,
    pub(crate) invalid_found_records: usize,
    pub(crate) closer_records: usize,
    pub(crate) closer_nodes_returned: usize,
    pub(crate) closer_nodes_merged: usize,
    pub(crate) closer_nodes_rejected_non_closer: usize,
    pub(crate) closer_nodes_rejected_duplicate: usize,
    pub(crate) peers_quarantined: usize,
    pub(crate) quarantine_threshold: u32,
    pub(crate) query_rounds: usize,
    pub(crate) alpha: usize,
    pub(crate) exhausted: bool,
}

#[derive(Debug, Serialize)]
pub(super) struct DhtFindValueRunResponse {
    pub(crate) key: String,
    pub(crate) found: bool,
    pub(crate) record: Option<DhtRecord>,
    pub(crate) records: usize,
    pub(crate) stats: DhtFindValueRunStats,
}

#[derive(Debug, Serialize)]
pub(super) struct DhtReplicationRunResponse {
    pub(crate) peers: usize,
    pub(crate) records: usize,
    pub(crate) stats: DhtReplicationRunStats,
}

#[derive(Debug, Serialize)]
pub(super) struct DhtMaintenanceRunResponse {
    pub(crate) peers: usize,
    pub(crate) records: usize,
    pub(crate) routing_peers: usize,
    pub(crate) replication: DhtReplicationRunStats,
    pub(crate) routing_refresh: DhtRoutingRefreshRunStats,
}

#[derive(Debug, Serialize)]
pub(super) struct DhtRoutingRefreshRunResponse {
    pub(crate) peers: usize,
    pub(crate) routing_peers: usize,
    pub(crate) stats: DhtRoutingRefreshRunStats,
}

#[cfg(test)]
pub(super) fn run_dht_replication(
    node: &mut NativeNode,
    peers: &[SyncPeerConfig],
    replication_factor: usize,
) -> DhtReplicationRunStats {
    run_dht_replication_with_logger(
        node,
        peers,
        DhtRunnerConfig {
            replication_factor,
            ..Default::default()
        },
        None,
    )
}

pub(super) fn run_dht_replication_with_logger(
    node: &mut NativeNode,
    peers: &[SyncPeerConfig],
    config: DhtRunnerConfig,
    logger: Option<&ControlLogger>,
) -> DhtReplicationRunStats {
    match config.transport {
        DhtTransportKind::HttpControl => run_dht_replication_with_transport(
            node,
            peers,
            config.replication_factor,
            logger,
            &HttpControlDhtTransport,
        ),
        DhtTransportKind::Libp2p => run_dht_replication_with_transport(
            node,
            peers,
            config.replication_factor,
            logger,
            &Libp2pDhtTransport::default(),
        ),
    }
}

#[allow(dead_code)]
pub(super) fn dht_runner_peer_configs(
    node: &NativeNode,
    configured_peers: &[SyncPeerConfig],
    transport: DhtTransportKind,
    quarantine_threshold: u32,
) -> Vec<SyncPeerConfig> {
    dht_runner_peer_configs_with_quarantine_count(
        node,
        configured_peers,
        transport,
        quarantine_threshold,
    )
    .0
}

pub(super) fn dht_runner_peer_configs_with_quarantine_count(
    node: &NativeNode,
    configured_peers: &[SyncPeerConfig],
    transport: DhtTransportKind,
    quarantine_threshold: u32,
) -> (Vec<SyncPeerConfig>, usize) {
    let mut peers = configured_peers.to_vec();
    let mut seen = peers
        .iter()
        .map(|peer| (peer.url.clone(), peer.peer_id.clone()))
        .collect::<HashSet<_>>();
    for routing_peer in node.kademlia.all_peers() {
        let peer = match transport {
            DhtTransportKind::Libp2p => sync_peer_config_from_libp2p_routing_peer(&routing_peer),
            DhtTransportKind::HttpControl => sync_peer_config_from_http_routing_peer(&routing_peer),
        };
        let Some(peer) = peer else {
            continue;
        };
        if seen.insert((peer.url.clone(), peer.peer_id.clone())) {
            peers.push(peer);
        }
    }
    let before = peers.len();
    peers.retain(|peer| !dht_peer_is_quarantined(node, peer, quarantine_threshold));
    let quarantined = before.saturating_sub(peers.len());
    peers.sort_by_key(|peer| dht_peer_health_sort_key(node, peer));
    (peers, quarantined)
}

pub(super) fn dht_peer_is_quarantined(
    node: &NativeNode,
    peer: &SyncPeerConfig,
    threshold: u32,
) -> bool {
    node.sync_status
        .peers
        .get(&peer.url)
        .map(|status| sync_peer_is_dht_quarantined(status, current_unix_timestamp(), threshold))
        .unwrap_or(false)
}

pub(super) fn sync_peer_is_dht_quarantined(
    status: &NodeSyncPeerStatus,
    now: u64,
    threshold: u32,
) -> bool {
    threshold > 0
        && status.consecutive_failures >= threshold
        && status
            .next_attempt_at
            .map(|next| next > now)
            .unwrap_or(true)
}

pub(super) fn dht_peer_health_sort_key(
    node: &NativeNode,
    peer: &SyncPeerConfig,
) -> (u32, u64, u64) {
    node.sync_status
        .peers
        .get(&peer.url)
        .map(|status| {
            (
                status.consecutive_failures,
                status.failures,
                status.last_error_at.unwrap_or(0),
            )
        })
        .unwrap_or((0, 0, 0))
}

pub(super) fn sync_peer_config_from_http_routing_peer(
    peer: &RoutingPeer,
) -> Option<SyncPeerConfig> {
    let address = peer
        .announce
        .addresses
        .iter()
        .find(|address| address.starts_with("http://"))?;
    Some(SyncPeerConfig {
        url: address.trim_end_matches('/').to_string(),
        // Never propagate a configured control token to discovered peers.
        // A token is only safe for peers explicitly configured by the operator.
        token: None,
        peer_id: Some(peer.announce.peer_id.clone()),
    })
}

pub(super) fn sync_peer_config_from_libp2p_routing_peer(
    peer: &RoutingPeer,
) -> Option<SyncPeerConfig> {
    peer.announce.peer_id.parse::<PeerId>().ok()?;
    let address = peer
        .announce
        .addresses
        .iter()
        .find(|address| address.parse::<Multiaddr>().is_ok())?;
    Some(SyncPeerConfig {
        url: format!("libp2p://{address}"),
        token: None,
        peer_id: Some(peer.announce.peer_id.clone()),
    })
}

pub(super) fn record_dht_peer_success(node: &mut NativeNode, peer: &SyncPeerConfig) {
    node.sync_status.record_success(
        &peer.url,
        NodeMergeStats {
            peers: 0,
            mailbox_deliveries: 0,
            prekey_bundles: 0,
            signed_one_time_prekey_records: 0,
            dht_records: 0,
        },
    );
    node.sync_status
        .record_next_attempt(&peer.url, current_unix_timestamp());
}

pub(super) fn record_dht_peer_failure(
    node: &mut NativeNode,
    peer: &SyncPeerConfig,
    error: impl Into<String>,
) {
    node.sync_status.record_failure(&peer.url, error.into());
    let consecutive_failures = node
        .sync_status
        .peers
        .get(&peer.url)
        .map(|status| status.consecutive_failures)
        .unwrap_or(1);
    let delay_seconds = sync_backoff_delay_seconds(
        DHT_PEER_FAILURE_BACKOFF_BASE_SECONDS,
        DHT_PEER_FAILURE_BACKOFF_MAX_SECONDS,
        consecutive_failures,
    );
    node.sync_status.record_next_attempt(
        &peer.url,
        current_unix_timestamp().saturating_add(delay_seconds),
    );
}

pub(super) fn run_dht_replication_with_transport(
    node: &mut NativeNode,
    peers: &[SyncPeerConfig],
    replication_factor: usize,
    logger: Option<&ControlLogger>,
    transport: &dyn DhtTransport,
) -> DhtReplicationRunStats {
    if peers.is_empty() || replication_factor == 0 {
        return DhtReplicationRunStats::default();
    }
    let plan = node.plan_dht_replication(replication_factor);
    let mut stats = DhtReplicationRunStats {
        records: plan.records.len(),
        ..Default::default()
    };
    for (record_index, planned) in plan.records.into_iter().enumerate() {
        let selected_peers = replication_control_peers_for_plan(peers, &planned);
        for peer in selected_peers {
            stats.attempts = stats.attempts.saturating_add(1);
            let request = DhtRpcRequest::StoreRecord {
                request_id: format!(
                    "replicate-{}-{}-{}",
                    planned.record.key, plan.generated_at, record_index
                ),
                record: planned.record.clone(),
            };
            match transport.send_dht_rpc(peer, &request) {
                Ok(DhtRpcResponse::StoreResult { stored: true, .. }) => {
                    stats.successes = stats.successes.saturating_add(1);
                    record_dht_peer_success(node, peer);
                }
                Ok(response) => {
                    stats.failures = stats.failures.saturating_add(1);
                    record_dht_peer_failure(
                        node,
                        peer,
                        format!("unexpected DHT StoreRecord response: {response:?}"),
                    );
                    log_warn_or_stderr(
                        logger,
                        "dht.replication.unexpected_response",
                        format!("dht replication to {} returned {response:?}", peer.url),
                        serde_json::json!({
                            "peer": peer.url,
                            "response": format!("{response:?}"),
                        }),
                    );
                }
                Err(err) => {
                    stats.failures = stats.failures.saturating_add(1);
                    record_dht_peer_failure(node, peer, err.to_string());
                    log_error_or_stderr(
                        logger,
                        "dht.replication.error",
                        format!("dht replication to {} failed: {err}", peer.url),
                        serde_json::json!({
                            "peer": peer.url,
                            "error": err.to_string(),
                        }),
                    );
                }
            }
        }
    }
    stats
}

fn replication_control_peers_for_plan<'a>(
    peers: &'a [SyncPeerConfig],
    planned: &DhtRecordReplicationPlan,
) -> Vec<&'a SyncPeerConfig> {
    if !peers.iter().any(|peer| peer.peer_id.is_some()) {
        return peers.iter().collect();
    }
    let target_peer_ids = planned
        .target_nodes
        .iter()
        .map(|peer| peer.announce.peer_id.as_str())
        .collect::<Vec<_>>();
    peers
        .iter()
        .filter(|peer| {
            peer.peer_id
                .as_deref()
                .map(|peer_id| target_peer_ids.contains(&peer_id))
                .unwrap_or(false)
        })
        .collect()
}

pub(super) fn filter_routing_refresh_nodes(
    seed_peer: &SyncPeerConfig,
    target: lm_node::KademliaNodeId,
    nodes: Vec<RoutingPeer>,
    seen_returned_peer_ids: &mut HashSet<String>,
) -> (Vec<RoutingPeer>, usize, usize) {
    let mut out = Vec::new();
    let mut rejected_non_closer = 0usize;
    let mut rejected_duplicate = 0usize;
    for node in nodes {
        if routing_peer_is_seed_peer(seed_peer, &node)
            || !seen_returned_peer_ids.insert(node.announce.peer_id.clone())
        {
            rejected_duplicate = rejected_duplicate.saturating_add(1);
            continue;
        }
        if !routing_peer_makes_find_node_progress(seed_peer, &node, target) {
            rejected_non_closer = rejected_non_closer.saturating_add(1);
            continue;
        }
        out.push(node);
    }
    (out, rejected_non_closer, rejected_duplicate)
}

pub(super) fn routing_peer_makes_find_node_progress(
    seed_peer: &SyncPeerConfig,
    candidate: &RoutingPeer,
    target_node_id: lm_node::KademliaNodeId,
) -> bool {
    let Some(seed_peer_id) = seed_peer.peer_id.as_deref() else {
        return true;
    };
    let seed_node_id = lm_node::KademliaNodeId::from_peer_id(seed_peer_id);
    candidate.node_id.xor_distance(&target_node_id) < seed_node_id.xor_distance(&target_node_id)
}

pub(super) fn routing_peer_is_seed_peer(
    seed_peer: &SyncPeerConfig,
    candidate: &RoutingPeer,
) -> bool {
    candidate.announce.peer_id == seed_peer.peer_id.as_deref().unwrap_or_default()
        || sync_peer_config_from_routing_peer_for_seed(seed_peer, candidate)
            .map(|candidate_config| candidate_config.url == seed_peer.url)
            .unwrap_or(false)
}

#[cfg(test)]
pub(super) fn run_dht_routing_refresh(
    node: &mut NativeNode,
    peers: &[SyncPeerConfig],
    limit: usize,
    max_targets: usize,
) -> DhtRoutingRefreshRunStats {
    run_dht_routing_refresh_with_logger(
        node,
        peers,
        DhtRunnerConfig {
            routing_refresh_limit: limit,
            routing_refresh_max_targets: max_targets,
            ..Default::default()
        },
        None,
    )
}

pub(super) fn run_dht_routing_refresh_with_logger(
    node: &mut NativeNode,
    peers: &[SyncPeerConfig],
    config: DhtRunnerConfig,
    logger: Option<&ControlLogger>,
) -> DhtRoutingRefreshRunStats {
    match config.transport {
        DhtTransportKind::HttpControl => run_dht_routing_refresh_with_transport(
            node,
            peers,
            config.routing_refresh_limit,
            config.routing_refresh_max_targets,
            logger,
            &HttpControlDhtTransport,
        ),
        DhtTransportKind::Libp2p => run_dht_routing_refresh_with_transport(
            node,
            peers,
            config.routing_refresh_limit,
            config.routing_refresh_max_targets,
            logger,
            &Libp2pDhtTransport::default(),
        ),
    }
}

pub(super) fn run_dht_routing_refresh_with_transport(
    node: &mut NativeNode,
    peers: &[SyncPeerConfig],
    limit: usize,
    max_targets: usize,
    logger: Option<&ControlLogger>,
    transport: &dyn DhtTransport,
) -> DhtRoutingRefreshRunStats {
    if peers.is_empty() || max_targets == 0 {
        return DhtRoutingRefreshRunStats::default();
    }
    let plan = node.plan_dht_routing_refresh();
    let targets = plan
        .targets
        .into_iter()
        .take(max_targets)
        .collect::<Vec<_>>();
    let mut stats = DhtRoutingRefreshRunStats {
        targets: targets.len(),
        ..Default::default()
    };
    let mut seen_returned_peer_ids = HashSet::new();
    for (target_index, target) in targets.into_iter().enumerate() {
        for peer in peers {
            stats.attempts = stats.attempts.saturating_add(1);
            let request = DhtRpcRequest::FindNode {
                request_id: format!("refresh-{}-{}-{}", target, plan.generated_at, target_index),
                target,
                limit: limit.clamp(1, 64),
            };
            match transport.send_dht_rpc(peer, &request) {
                Ok(DhtRpcResponse::Nodes { nodes, .. }) => {
                    let nodes = nodes
                        .into_iter()
                        .take(dht_response_node_limit(limit))
                        .collect::<Vec<_>>();
                    let returned = nodes.len();
                    stats.nodes_returned = stats.nodes_returned.saturating_add(returned);
                    let (nodes, rejected_non_closer, rejected_duplicate) =
                        filter_routing_refresh_nodes(
                            peer,
                            target,
                            nodes,
                            &mut seen_returned_peer_ids,
                        );
                    stats.nodes_rejected_non_closer = stats
                        .nodes_rejected_non_closer
                        .saturating_add(rejected_non_closer);
                    stats.nodes_rejected_duplicate = stats
                        .nodes_rejected_duplicate
                        .saturating_add(rejected_duplicate);
                    let merged = node.merge_verified_routing_peers(nodes);
                    stats.nodes_merged = stats.nodes_merged.saturating_add(merged);
                    stats.successes = stats.successes.saturating_add(1);
                    if returned > 0
                        && rejected_non_closer.saturating_add(rejected_duplicate) > 0
                        && rejected_non_closer.saturating_add(rejected_duplicate) == returned
                    {
                        record_dht_peer_failure(
                            node,
                            peer,
                            "DHT FindNode response contained only non-progressing or duplicate nodes",
                        );
                    } else {
                        record_dht_peer_success(node, peer);
                    }
                }
                Ok(response) => {
                    stats.failures = stats.failures.saturating_add(1);
                    record_dht_peer_failure(
                        node,
                        peer,
                        format!("unexpected DHT FindNode response: {response:?}"),
                    );
                    log_warn_or_stderr(
                        logger,
                        "dht.routing_refresh.unexpected_response",
                        format!(
                            "dht routing refresh from {} returned {response:?}",
                            peer.url
                        ),
                        serde_json::json!({
                            "peer": peer.url,
                            "response": format!("{response:?}"),
                        }),
                    );
                }
                Err(err) => {
                    stats.failures = stats.failures.saturating_add(1);
                    record_dht_peer_failure(node, peer, err.to_string());
                    log_error_or_stderr(
                        logger,
                        "dht.routing_refresh.error",
                        format!("dht routing refresh from {} failed: {err}", peer.url),
                        serde_json::json!({
                            "peer": peer.url,
                            "error": err.to_string(),
                        }),
                    );
                }
            }
        }
    }
    stats
}

#[allow(dead_code)]
pub(super) fn dht_find_value_with_transport(
    node: &mut NativeNode,
    peers: &[SyncPeerConfig],
    key: lm_node::DhtRecordKey,
    limit: usize,
    max_peers: usize,
    alpha: usize,
    transport: &dyn DhtTransport,
) -> DhtFindValueRunStats {
    let alpha = alpha.clamp(1, 16);
    if peers.is_empty() || max_peers == 0 {
        return DhtFindValueRunStats {
            alpha,
            exhausted: true,
            ..Default::default()
        };
    }
    let mut stats = DhtFindValueRunStats {
        alpha,
        ..Default::default()
    };
    let mut queue = peers.iter().take(max_peers).cloned().collect::<Vec<_>>();
    let mut queued = queue
        .iter()
        .map(dht_query_peer_dedup_key)
        .collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    let mut index = 0usize;
    let mut found = false;
    while index < queue.len() && stats.attempts < max_peers && !found {
        let mut round_peers = Vec::new();
        while index < queue.len() && round_peers.len() < alpha && stats.attempts < max_peers {
            let peer = queue[index].clone();
            index += 1;
            let peer_key = (peer.url.clone(), peer.peer_id.clone());
            if !seen.insert(peer_key) {
                continue;
            }
            stats.attempts = stats.attempts.saturating_add(1);
            round_peers.push(peer);
        }
        if round_peers.is_empty() {
            continue;
        }
        stats.query_rounds = stats.query_rounds.saturating_add(1);
        let round_results = std::thread::scope(|scope| {
            let handles = round_peers
                .iter()
                .map(|peer| {
                    scope.spawn(move || {
                        let response = send_dht_find_value(peer, key, limit, transport)
                            .map_err(|err| err.to_string());
                        (peer.clone(), response)
                    })
                })
                .collect::<Vec<_>>();
            handles
                .into_iter()
                .map(|handle| {
                    handle.join().unwrap_or_else(|_| {
                        (
                            SyncPeerConfig {
                                url: "thread-panic".into(),
                                token: None,
                                peer_id: None,
                            },
                            Err("dht find-value worker panicked".into()),
                        )
                    })
                })
                .collect::<Vec<_>>()
        });
        for (peer, response) in round_results {
            match response {
                Ok(DhtRpcResponse::Value {
                    record,
                    closer_records,
                    closer_nodes,
                    ..
                }) => {
                    stats.successes = stats.successes.saturating_add(1);
                    let mut useful_response = false;
                    let mut rejected_found_record = false;
                    if let Some(record) = record {
                        if node.accept_dht_record_from_peer(record) {
                            stats.found_records = stats.found_records.saturating_add(1);
                            useful_response = true;
                            found = true;
                        } else {
                            stats.invalid_found_records =
                                stats.invalid_found_records.saturating_add(1);
                            rejected_found_record = true;
                        }
                    }
                    let before_closer_records = stats.closer_records;
                    let before_closer_nodes_returned = stats.closer_nodes_returned;
                    let before_closer_nodes_merged = stats.closer_nodes_merged;
                    let before_rejected_non_closer = stats.closer_nodes_rejected_non_closer;
                    let before_rejected_duplicate = stats.closer_nodes_rejected_duplicate;
                    if !found {
                        merge_find_value_closer_results(
                            node,
                            &mut queue,
                            &mut queued,
                            &seen,
                            &mut stats,
                            key,
                            limit,
                            max_peers,
                            &peer,
                            closer_records,
                            closer_nodes,
                        );
                    }
                    useful_response = useful_response
                        || stats.closer_records > before_closer_records
                        || stats.closer_nodes_merged > before_closer_nodes_merged;
                    let returned = stats
                        .closer_nodes_returned
                        .saturating_sub(before_closer_nodes_returned);
                    let rejected = stats
                        .closer_nodes_rejected_non_closer
                        .saturating_sub(before_rejected_non_closer)
                        .saturating_add(
                            stats
                                .closer_nodes_rejected_duplicate
                                .saturating_sub(before_rejected_duplicate),
                        );
                    if rejected_found_record {
                        record_dht_peer_failure(
                            node,
                            &peer,
                            "DHT FindValue response contained an invalid found record",
                        );
                    } else if returned > 0 && rejected == returned && !useful_response {
                        record_dht_peer_failure(
                            node,
                            &peer,
                            "DHT FindValue response contained only non-progressing or duplicate closer nodes",
                        );
                    } else {
                        record_dht_peer_success(node, &peer);
                    }
                    if found {
                        break;
                    }
                }
                Ok(response) => {
                    stats.failures = stats.failures.saturating_add(1);
                    record_dht_peer_failure(
                        node,
                        &peer,
                        format!("unexpected DHT FindValue response: {response:?}"),
                    );
                }
                Err(err) => {
                    stats.failures = stats.failures.saturating_add(1);
                    record_dht_peer_failure(node, &peer, err);
                }
            }
        }
    }
    stats.exhausted = stats.found_records == 0 && index >= queue.len();
    stats
}

#[allow(clippy::too_many_arguments)]
pub(super) fn merge_find_value_closer_results(
    node: &mut NativeNode,
    queue: &mut Vec<SyncPeerConfig>,
    queued: &mut HashSet<(String, Option<String>)>,
    seen: &HashSet<(String, Option<String>)>,
    stats: &mut DhtFindValueRunStats,
    key: lm_node::DhtRecordKey,
    limit: usize,
    max_peers: usize,
    seed_peer: &SyncPeerConfig,
    closer_records: Vec<DhtRecord>,
    closer_nodes: Vec<RoutingPeer>,
) {
    let closer_records = closer_records
        .into_iter()
        .take(dht_response_record_limit(limit))
        .collect::<Vec<_>>();
    stats.closer_records = stats
        .closer_records
        .saturating_add(node.merge_dht_records_from_peer(closer_records));
    let mut closer_nodes = closer_nodes
        .into_iter()
        .take(dht_response_node_limit(limit))
        .collect::<Vec<_>>();
    let target_node_id = key.to_node_id();
    stats.closer_nodes_returned = stats
        .closer_nodes_returned
        .saturating_add(closer_nodes.len());
    let before_progress_filter = closer_nodes.len();
    closer_nodes
        .retain(|peer| routing_peer_makes_find_value_progress(seed_peer, peer, target_node_id));
    stats.closer_nodes_rejected_non_closer = stats
        .closer_nodes_rejected_non_closer
        .saturating_add(before_progress_filter.saturating_sub(closer_nodes.len()));
    closer_nodes.sort_by_key(|peer| peer.node_id.xor_distance(&target_node_id));
    for closer_node in closer_nodes {
        let candidate = sync_peer_config_from_routing_peer_for_seed(seed_peer, &closer_node);
        let Some(candidate) = candidate else {
            let merged = node.merge_verified_routing_peers(vec![closer_node]);
            stats.closer_nodes_merged = stats.closer_nodes_merged.saturating_add(merged);
            continue;
        };
        let candidate_key = dht_query_peer_dedup_key(&candidate);
        if candidate_is_seed_peer(seed_peer, &candidate)
            || seen.contains(&candidate_key)
            || queued.contains(&candidate_key)
        {
            stats.closer_nodes_rejected_duplicate =
                stats.closer_nodes_rejected_duplicate.saturating_add(1);
            continue;
        }
        let merged = node.merge_verified_routing_peers(vec![closer_node]);
        stats.closer_nodes_merged = stats.closer_nodes_merged.saturating_add(merged);
        if merged > 0 && queue.len() < max_peers && queued.insert(candidate_key) {
            queue.push(candidate);
        }
    }
}

pub(super) fn dht_query_peer_dedup_key(peer: &SyncPeerConfig) -> (String, Option<String>) {
    (peer.url.clone(), peer.peer_id.clone())
}

pub(super) fn candidate_is_seed_peer(
    seed_peer: &SyncPeerConfig,
    candidate: &SyncPeerConfig,
) -> bool {
    candidate.url == seed_peer.url
        || candidate
            .peer_id
            .as_deref()
            .zip(seed_peer.peer_id.as_deref())
            .map(|(candidate, seed)| candidate == seed)
            .unwrap_or(false)
}

pub(super) fn routing_peer_makes_find_value_progress(
    seed_peer: &SyncPeerConfig,
    candidate: &RoutingPeer,
    target_node_id: lm_node::KademliaNodeId,
) -> bool {
    let Some(seed_peer_id) = seed_peer.peer_id.as_deref() else {
        // Configured HTTP control peers may not have a peer id. In that case
        // we cannot prove progress relative to the responder, so keep the
        // verified candidate and rely on max_peers/query budget bounds.
        return true;
    };
    let seed_node_id = lm_node::KademliaNodeId::from_peer_id(seed_peer_id);
    candidate.node_id.xor_distance(&target_node_id) < seed_node_id.xor_distance(&target_node_id)
}

pub(super) fn sync_peer_config_from_routing_peer_for_seed(
    seed_peer: &SyncPeerConfig,
    routing_peer: &RoutingPeer,
) -> Option<SyncPeerConfig> {
    if seed_peer.url.starts_with("libp2p://") {
        return sync_peer_config_from_libp2p_routing_peer(routing_peer);
    }
    let address = routing_peer.announce.addresses.iter().find(|address| {
        seed_peer
            .url
            .split_once("://")
            .map(|(scheme, _)| address.starts_with(&format!("{scheme}://")))
            .unwrap_or(false)
    })?;
    Some(SyncPeerConfig {
        url: address.clone(),
        token: seed_peer.token.clone(),
        peer_id: Some(routing_peer.announce.peer_id.clone()),
    })
}

#[allow(dead_code)]
pub(super) fn dht_response_record_limit(requested: usize) -> usize {
    requested.clamp(1, MAX_DHT_RESPONSE_RECORDS)
}

pub(super) fn dht_response_node_limit(requested: usize) -> usize {
    requested.clamp(1, MAX_DHT_RESPONSE_NODES)
}

#[allow(dead_code)]
pub(super) fn send_dht_find_value(
    peer: &SyncPeerConfig,
    key: lm_node::DhtRecordKey,
    limit: usize,
    transport: &dyn DhtTransport,
) -> Result<DhtRpcResponse, Box<dyn std::error::Error>> {
    transport.send_dht_rpc(
        peer,
        &DhtRpcRequest::FindValue {
            request_id: format!("find-value-{}", key),
            key,
            limit: limit.clamp(1, 64),
        },
    )
}

pub(super) fn log_warn_or_stderr(
    logger: Option<&ControlLogger>,
    event: &str,
    message: String,
    fields: serde_json::Value,
) {
    if let Some(logger) = logger {
        logger.warn(event, message, fields);
    } else {
        eprintln!("{message}");
    }
}

pub(super) fn log_error_or_stderr(
    logger: Option<&ControlLogger>,
    event: &str,
    message: String,
    fields: serde_json::Value,
) {
    if let Some(logger) = logger {
        logger.error(event, message, fields);
    } else {
        eprintln!("{message}");
    }
}

#[cfg(test)]
pub(super) fn send_dht_rpc(
    peer: &SyncPeerConfig,
    request: &DhtRpcRequest,
) -> Result<DhtRpcResponse, Box<dyn std::error::Error>> {
    HttpControlDhtTransport.send_dht_rpc(peer, request)
}
