use super::*;

#[derive(Debug, Clone, Default)]
pub(super) struct ControlSecurityConfig {
    pub(crate) token: Option<String>,
    pub(crate) previous_tokens: Vec<String>,
    pub(crate) cors_allow_origins: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RateLimitConfig {
    pub(crate) window_seconds: u64,
    pub(crate) max_requests: u32,
}

impl RateLimitConfig {
    pub(super) fn is_enabled(self) -> bool {
        self.window_seconds > 0 && self.max_requests > 0
    }
}

#[derive(Debug, Clone)]
pub(super) struct RateLimitEntry {
    pub(crate) window_started_at: Instant,
    pub(crate) count: u32,
}

#[derive(Debug, Default)]
pub(super) struct RateLimiter {
    pub(crate) entries: HashMap<IpAddr, RateLimitEntry>,
}

impl RateLimiter {
    pub(super) fn check(&mut self, ip: IpAddr, now: Instant, config: RateLimitConfig) -> bool {
        if !config.is_enabled() {
            return true;
        }
        let window = Duration::from_secs(config.window_seconds);
        let entry = self.entries.entry(ip).or_insert(RateLimitEntry {
            window_started_at: now,
            count: 0,
        });
        if now.duration_since(entry.window_started_at) >= window {
            entry.window_started_at = now;
            entry.count = 0;
        }
        if entry.count >= config.max_requests {
            return false;
        }
        entry.count = entry.count.saturating_add(1);
        true
    }

    pub(super) fn prune(&mut self, now: Instant, config: RateLimitConfig) {
        if !config.is_enabled() {
            self.entries.clear();
            return;
        }
        let ttl = Duration::from_secs(config.window_seconds.saturating_mul(2).max(1));
        self.entries
            .retain(|_, entry| now.duration_since(entry.window_started_at) < ttl);
    }
}

impl ControlSecurityConfig {
    pub(super) fn has_bearer_tokens(&self) -> bool {
        self.token.is_some() || !self.previous_tokens.is_empty()
    }

    pub(super) fn is_loopback_only(&self) -> bool {
        !self.has_bearer_tokens()
    }

    pub(super) fn token_matches(&self, value: &str) -> bool {
        self.token
            .as_deref()
            .into_iter()
            .chain(self.previous_tokens.iter().map(String::as_str))
            .any(|token| constant_time_eq(value.as_bytes(), token.as_bytes()))
    }

    pub(super) fn allows_origin(&self, origin: Option<&str>) -> bool {
        if self.cors_allow_origins.is_empty() {
            return true;
        }
        let Some(origin) = origin else {
            return true;
        };
        self.cors_allow_origins
            .iter()
            .any(|allowed| allowed == "*" || allowed == origin)
    }

    pub(super) fn access_control_origin(&self, request_origin: Option<&str>) -> String {
        if self.cors_allow_origins.is_empty() || self.cors_allow_origins.iter().any(|v| v == "*") {
            "*".to_string()
        } else {
            request_origin.unwrap_or("null").to_string()
        }
    }
}

pub(super) fn status_for_request_error(error: &str) -> u16 {
    if error.contains("request body too large") {
        413
    } else if error.contains("request header too large")
        || error.contains("request method too large")
        || error.contains("request path too large")
    {
        431
    } else {
        400
    }
}

pub(super) fn status_reason(status: u16) -> &'static str {
    match status {
        400 => "Bad Request",
        413 => "Payload Too Large",
        431 => "Request Header Fields Too Large",
        _ => "Bad Request",
    }
}

pub(super) fn control_error_http_response(status: u16, body: &str) -> String {
    format!(
        "HTTP/1.1 {} {}\r\ncontent-type: text/plain; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        status,
        status_reason(status),
        body.len(),
        body
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn serve_control(
    bind: &str,
    node: &mut NativeNode,
    state_file: Option<&str>,
    state_db: Option<&str>,
    sync_peers: Vec<SyncPeerConfig>,
    sync_interval_seconds: u64,
    sync_max_backoff_seconds: u64,
    dht_runner: DhtRunnerConfig,
    security: ControlSecurityConfig,
    rate_limit: RateLimitConfig,
    logger: ControlLogger,
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(bind)?;
    listener.set_nonblocking(true)?;
    let mut rate_limiter = RateLimiter::default();
    let mut runtime_stats = ControlRuntimeStats::new(current_unix_timestamp());
    let mut last_rate_limit_prune = Instant::now();
    let dht_configured_peers = sync_peers.clone();
    let mut sync_peers = if sync_interval_seconds == 0 {
        Vec::new()
    } else {
        sync_peers
            .into_iter()
            .map(|config| SyncPeerRuntime {
                config,
                next_attempt_at: Instant::now(),
                consecutive_failures: 0,
            })
            .collect::<Vec<_>>()
    };
    logger.info(
        "control.start",
        format!("LM Talk control plane listening on http://{bind}"),
        serde_json::Value::Null,
    );
    logger.info(
        "control.endpoints",
        "endpoints: GET /health, GET /control/stats, GET /control/metrics, POST /announce, GET /peers/closest, POST /mailbox/push, GET /mailbox/take, GET /mailbox/status, POST /mailbox/ack, POST /prekey/publish, GET /prekey/get, GET /prekey/status, GET /dht/key, POST/GET /dht/record, GET /dht/closest, POST /dht/rpc, GET /dht/find-value, GET /dht/maintenance, GET /dht/replicate, GET /dht/routing-refresh, GET /dht/replication-plan, GET /dht/routing-refresh-plan, GET /sync/snapshot, GET /sync/status, POST /sync/peer/reset, POST /sync/import"
            .to_string(),
        serde_json::Value::Null,
    );
    if security.has_bearer_tokens() {
        logger.info(
            "control.security",
            "control security: bearer token required",
            serde_json::json!({"auth": "bearer", "previous_token_count": security.previous_tokens.len()}),
        );
    } else {
        logger.warn(
            "control.security",
            "control security: no token configured; loopback clients only",
            serde_json::json!({"auth": "loopback_only"}),
        );
    }
    if !security.cors_allow_origins.is_empty() {
        logger.info(
            "control.cors",
            format!(
                "CORS allow origins: {}",
                security.cors_allow_origins.join(",")
            ),
            serde_json::json!({"origins": security.cors_allow_origins.clone()}),
        );
    }
    if rate_limit.is_enabled() {
        logger.info(
            "control.rate_limit",
            format!(
                "control rate limit: {} requests / {}s per client IP",
                rate_limit.max_requests, rate_limit.window_seconds
            ),
            serde_json::json!({
                "window_seconds": rate_limit.window_seconds,
                "max_requests": rate_limit.max_requests,
            }),
        );
    } else {
        logger.info(
            "control.rate_limit",
            "control rate limit: disabled",
            serde_json::json!({"enabled": false}),
        );
    }
    if let Some(config) = node.config.mailbox_global_rate_limit() {
        logger.info(
            "mailbox.global_rate_limit",
            format!(
                "mailbox global rate limit: {} messages / {}s",
                config.max_messages, config.window_seconds
            ),
            serde_json::json!({
                "window_seconds": config.window_seconds,
                "max_messages": config.max_messages,
            }),
        );
    } else {
        logger.info(
            "mailbox.global_rate_limit",
            "mailbox global rate limit: disabled",
            serde_json::json!({"enabled": false}),
        );
    }
    if let Some(config) = node.config.mailbox_sender_rate_limit() {
        logger.info(
            "mailbox.sender_rate_limit",
            format!(
                "mailbox sender rate limit: {} messages / {}s per sender",
                config.max_messages, config.window_seconds
            ),
            serde_json::json!({
                "window_seconds": config.window_seconds,
                "max_messages": config.max_messages,
            }),
        );
    } else {
        logger.info(
            "mailbox.sender_rate_limit",
            "mailbox sender rate limit: disabled",
            serde_json::json!({"enabled": false}),
        );
    }
    logger.info(
        "mailbox.storage_quota",
        "mailbox storage quotas",
        serde_json::json!({
            "max_mailbox_bytes": node.config.max_mailbox_bytes,
            "max_mailbox_bytes_per_user": node.config.max_mailbox_bytes_per_user,
            "max_mailbox_messages_per_user": node.config.max_mailbox_messages_per_user,
        }),
    );
    if !sync_peers.is_empty() && sync_interval_seconds > 0 {
        let peer_urls = sync_peers
            .iter()
            .map(|peer| peer.config.url.as_str())
            .collect::<Vec<_>>()
            .join(",");
        logger.info(
            "sync.config",
            format!("auto snapshot sync: every {sync_interval_seconds}s from {peer_urls}"),
            serde_json::json!({
                "interval_seconds": sync_interval_seconds,
                "peers": sync_peers.iter().map(|peer| peer.config.url.as_str()).collect::<Vec<_>>(),
            }),
        );
        logger.info(
            "dht.config",
            format!(
                "dht runners: replication_factor={} routing_refresh_limit={} routing_refresh_max_targets={} transport={}",
                dht_runner.replication_factor,
                dht_runner.routing_refresh_limit,
                dht_runner.routing_refresh_max_targets,
                dht_runner.transport.as_str()
            ),
            serde_json::json!({
                "replication_factor": dht_runner.replication_factor,
                "routing_refresh_limit": dht_runner.routing_refresh_limit,
                "routing_refresh_max_targets": dht_runner.routing_refresh_max_targets,
                "transport": dht_runner.transport.as_str(),
            }),
        );
    }
    loop {
        let now = Instant::now();
        let mut sync_ran = false;
        let mut max_sync_schedule_delay = Duration::ZERO;
        for peer in &mut sync_peers {
            if now >= peer.next_attempt_at {
                let delay = now.duration_since(peer.next_attempt_at);
                max_sync_schedule_delay = max_sync_schedule_delay.max(delay);
                runtime_stats.record_sync_schedule_delay(delay);
                run_snapshot_sync(
                    node,
                    peer,
                    sync_interval_seconds,
                    sync_max_backoff_seconds,
                    &logger,
                );
                sync_ran = true;
            }
        }
        if sync_ran {
            let peer_configs = sync_peers
                .iter()
                .map(|peer| peer.config.clone())
                .collect::<Vec<_>>();
            let (dht_peer_configs, dht_peers_quarantined) =
                dht_runner_peer_configs_with_quarantine_count(
                    node,
                    &peer_configs,
                    dht_runner.transport,
                    dht_runner.peer_quarantine_consecutive_failures,
                );
            runtime_stats.record_dht_replication_schedule_delay(max_sync_schedule_delay);
            let mut replication =
                run_dht_replication_with_logger(node, &dht_peer_configs, dht_runner, Some(&logger));
            replication.peers_quarantined = dht_peers_quarantined;
            runtime_stats.record_dht_replication_run(replication, current_unix_timestamp());
            if replication.attempts > 0 {
                logger.info(
                    "dht.replication.run",
                    format!(
                        "dht replication: records={} attempts={} successes={} failures={}",
                        replication.records,
                        replication.attempts,
                        replication.successes,
                        replication.failures
                    ),
                    serde_json::json!({
                        "records": replication.records,
                        "attempts": replication.attempts,
                        "successes": replication.successes,
                        "failures": replication.failures,
                        "peers_quarantined": replication.peers_quarantined,
                    }),
                );
            }
            runtime_stats.record_dht_routing_refresh_schedule_delay(max_sync_schedule_delay);
            let mut refresh = run_dht_routing_refresh_with_logger(
                node,
                &dht_peer_configs,
                dht_runner,
                Some(&logger),
            );
            refresh.peers_quarantined = dht_peers_quarantined;
            runtime_stats.record_dht_routing_refresh_run(refresh, current_unix_timestamp());
            if refresh.attempts > 0 {
                logger.info(
                    "dht.routing_refresh.run",
                    format!(
                        "dht routing refresh: targets={} attempts={} successes={} failures={} nodes_returned={} nodes_merged={}",
                        refresh.targets,
                        refresh.attempts,
                        refresh.successes,
                        refresh.failures,
                        refresh.nodes_returned,
                        refresh.nodes_merged
                    ),
                    serde_json::json!({
                        "targets": refresh.targets,
                        "attempts": refresh.attempts,
                        "successes": refresh.successes,
                        "failures": refresh.failures,
                        "nodes_returned": refresh.nodes_returned,
                        "nodes_merged": refresh.nodes_merged,
                        "peers_quarantined": refresh.peers_quarantined,
                    }),
                );
            }
            if let Some(path) = state_file
                && let Err(err) = save_node_state(path, node)
            {
                logger.error(
                    "state_file.save_error",
                    format!("state save error: {err}"),
                    serde_json::json!({"path": path, "error": err.to_string()}),
                );
            }
            if let Some(path) = state_db
                && let Err(err) = save_node_state_db(path, node)
            {
                logger.error(
                    "state_db.save_error",
                    format!("state db save error: {err}"),
                    serde_json::json!({"path": path, "error": err.to_string()}),
                );
            }
        }
        if now.duration_since(last_rate_limit_prune) >= Duration::from_secs(60) {
            rate_limiter.prune(now, rate_limit);
            last_rate_limit_prune = now;
        }
        match listener.accept() {
            Ok((mut stream, _addr)) => {
                if let Err(err) = handle_stream(
                    &mut stream,
                    node,
                    &security,
                    &mut rate_limiter,
                    rate_limit,
                    &mut runtime_stats,
                    state_file,
                    state_db,
                    &dht_configured_peers,
                    dht_runner,
                    &logger,
                ) {
                    let status = status_for_request_error(&err.to_string());
                    runtime_stats.record_response("<bad-request>", status, Duration::ZERO);
                    let body = format!("request error: {err}");
                    logger.warn(
                        "control.request_error",
                        body.clone(),
                        serde_json::json!({"error": err.to_string(), "status": status}),
                    );
                    let response = control_error_http_response(status, &body);
                    let _ = stream.write_all(response.as_bytes());
                } else if let Some(path) = state_file
                    && let Err(err) = save_node_state(path, node)
                {
                    logger.error(
                        "state_file.save_error",
                        format!("state save error: {err}"),
                        serde_json::json!({"path": path, "error": err.to_string()}),
                    );
                }
                if let Some(path) = state_db
                    && let Err(err) = save_node_state_db(path, node)
                {
                    logger.error(
                        "state_db.save_error",
                        format!("state db save error: {err}"),
                        serde_json::json!({"path": path, "error": err.to_string()}),
                    );
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(err) => logger.error(
                "control.connection_error",
                format!("connection error: {err}"),
                serde_json::json!({"error": err.to_string()}),
            ),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_stream(
    stream: &mut TcpStream,
    node: &mut NativeNode,
    security: &ControlSecurityConfig,
    rate_limiter: &mut RateLimiter,
    rate_limit: RateLimitConfig,
    runtime_stats: &mut ControlRuntimeStats,
    state_file: Option<&str>,
    state_db: Option<&str>,
    dht_configured_peers: &[SyncPeerConfig],
    dht_runner: DhtRunnerConfig,
    logger: &ControlLogger,
) -> Result<(), Box<dyn std::error::Error>> {
    configure_control_client_stream(stream)?;
    let peer_addr = stream.peer_addr().ok();
    let request = read_http_request(stream)?;
    let started_at = Instant::now();
    let endpoint = control_endpoint_key(&request);
    let method = request.method.clone();
    let path = request.path.clone();
    let request_body_bytes = request.body.len();
    let origin = request.header("origin").map(str::to_string);
    let response = if !security.allows_origin(origin.as_deref()) {
        ControlHttpResponse::text(403, "cors origin not allowed")
    } else if !request_is_within_rate_limit(&request, peer_addr.as_ref(), rate_limiter, rate_limit)
    {
        ControlHttpResponse::text(429, "rate limit exceeded")
    } else if request.method == "OPTIONS" {
        ControlHttpResponse::from_control(node.handle_control_request(request))
    } else if !request_is_authorized(&request, security, peer_addr.as_ref()) {
        ControlHttpResponse::text(401, "unauthorized")
    } else if request.method == "GET" && request.path.starts_with("/control/stats") {
        node.prune_expired_records();
        ControlHttpResponse::json(
            200,
            &ControlStatsResponse {
                runtime: runtime_stats,
                maintenance: node.maintenance_stats().clone(),
                state_db: state_db_stats_opt(state_db),
                state_file: state_file_stats_opt(state_file),
            },
        )
    } else if request.method == "GET" && request.path.starts_with("/control/metrics") {
        node.prune_expired_records();
        ControlHttpResponse::openmetrics(
            200,
            runtime_stats.to_openmetrics(
                node.maintenance_stats(),
                state_db_stats_opt(state_db).as_ref(),
                state_file_stats_opt(state_file).as_ref(),
                Some(&node.sync_status),
                dht_runner.peer_quarantine_consecutive_failures,
            ),
        )
    } else if request.method == "GET" && request.path.starts_with("/dht/find-value") {
        handle_control_dht_find_value_run(
            node,
            dht_configured_peers,
            dht_runner,
            &request.path,
            Some(runtime_stats),
        )
    } else if request.method == "GET" && request.path.starts_with("/dht/maintenance") {
        handle_control_dht_maintenance_run(
            node,
            dht_configured_peers,
            dht_runner,
            &request.path,
            Some(runtime_stats),
        )
    } else if request.method == "GET" && request.path.starts_with("/dht/replicate") {
        handle_control_dht_replication_run(
            node,
            dht_configured_peers,
            dht_runner,
            &request.path,
            Some(runtime_stats),
        )
    } else if request.method == "GET" && request.path.starts_with("/dht/routing-refresh") {
        handle_control_dht_routing_refresh_run(
            node,
            dht_configured_peers,
            dht_runner,
            &request.path,
            Some(runtime_stats),
        )
    } else {
        ControlHttpResponse::from_control(node.handle_control_request(request))
    };
    let duration = started_at.elapsed();
    runtime_stats.record_response(&endpoint, response.status, duration);
    runtime_stats.record_sync_snapshot_bytes(
        &endpoint,
        response.status,
        request_body_bytes,
        response.body.len(),
    );
    logger.info(
        "control.request",
        format!(
            "control request: {} {} status={} duration_micros={}",
            method,
            path,
            response.status,
            duration.as_micros()
        ),
        serde_json::json!({
            "method": method,
            "path": path,
            "endpoint": endpoint,
            "status": response.status,
            "duration_micros": duration.as_micros(),
            "request_body_bytes": request_body_bytes,
            "response_body_bytes": response.body.len(),
            "remote_addr": peer_addr.map(|addr| addr.to_string()),
        }),
    );
    let allow_origin = security.access_control_origin(origin.as_deref());
    stream.write_all(response.to_http_string(&allow_origin).as_bytes())?;
    Ok(())
}

pub(super) fn handle_control_dht_maintenance_run(
    node: &mut NativeNode,
    dht_configured_peers: &[SyncPeerConfig],
    dht_runner: DhtRunnerConfig,
    path: &str,
    mut runtime_stats: Option<&mut ControlRuntimeStats>,
) -> ControlHttpResponse {
    let replication_factor = query_param_value(path, "factor")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(dht_runner.replication_factor)
        .clamp(1, 64);
    let refresh_limit = query_param_value(path, "limit")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(dht_runner.routing_refresh_limit)
        .clamp(1, MAX_DHT_RESPONSE_NODES);
    let max_targets = query_param_value(path, "max_targets")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(dht_runner.routing_refresh_max_targets)
        .clamp(1, 256);
    let (peers, peers_quarantined) = dht_runner_peer_configs_with_quarantine_count(
        node,
        dht_configured_peers,
        dht_runner.transport,
        dht_runner.peer_quarantine_consecutive_failures,
    );
    let replication_config = DhtRunnerConfig {
        replication_factor,
        ..dht_runner
    };
    let mut replication = run_dht_replication_with_logger(node, &peers, replication_config, None);
    replication.peers_quarantined = peers_quarantined;
    if let Some(runtime_stats) = runtime_stats.as_deref_mut() {
        runtime_stats.record_dht_replication_run(replication, current_unix_timestamp());
    }
    let refresh_config = DhtRunnerConfig {
        routing_refresh_limit: refresh_limit,
        routing_refresh_max_targets: max_targets,
        ..dht_runner
    };
    let mut routing_refresh =
        run_dht_routing_refresh_with_logger(node, &peers, refresh_config, None);
    routing_refresh.peers_quarantined = peers_quarantined;
    if let Some(runtime_stats) = runtime_stats {
        runtime_stats.record_dht_routing_refresh_run(routing_refresh, current_unix_timestamp());
    }
    ControlHttpResponse::json(
        200,
        &DhtMaintenanceRunResponse {
            peers: peers.len(),
            records: node.dht_records.len(),
            routing_peers: node.kademlia.len(),
            replication,
            routing_refresh,
        },
    )
}

pub(super) fn handle_control_dht_replication_run(
    node: &mut NativeNode,
    dht_configured_peers: &[SyncPeerConfig],
    dht_runner: DhtRunnerConfig,
    path: &str,
    runtime_stats: Option<&mut ControlRuntimeStats>,
) -> ControlHttpResponse {
    let replication_factor = query_param_value(path, "factor")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(dht_runner.replication_factor)
        .clamp(1, 64);
    let (peers, peers_quarantined) = dht_runner_peer_configs_with_quarantine_count(
        node,
        dht_configured_peers,
        dht_runner.transport,
        dht_runner.peer_quarantine_consecutive_failures,
    );
    let config = DhtRunnerConfig {
        replication_factor,
        ..dht_runner
    };
    let mut stats = run_dht_replication_with_logger(node, &peers, config, None);
    stats.peers_quarantined = peers_quarantined;
    if let Some(runtime_stats) = runtime_stats {
        runtime_stats.record_dht_replication_run(stats, current_unix_timestamp());
    }
    ControlHttpResponse::json(
        200,
        &DhtReplicationRunResponse {
            peers: peers.len(),
            records: node.dht_records.len(),
            stats,
        },
    )
}

pub(super) fn handle_control_dht_routing_refresh_run(
    node: &mut NativeNode,
    dht_configured_peers: &[SyncPeerConfig],
    dht_runner: DhtRunnerConfig,
    path: &str,
    runtime_stats: Option<&mut ControlRuntimeStats>,
) -> ControlHttpResponse {
    let limit = query_param_value(path, "limit")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(dht_runner.routing_refresh_limit)
        .clamp(1, MAX_DHT_RESPONSE_NODES);
    let max_targets = query_param_value(path, "max_targets")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(dht_runner.routing_refresh_max_targets)
        .clamp(1, 256);
    let (peers, peers_quarantined) = dht_runner_peer_configs_with_quarantine_count(
        node,
        dht_configured_peers,
        dht_runner.transport,
        dht_runner.peer_quarantine_consecutive_failures,
    );
    let config = DhtRunnerConfig {
        routing_refresh_limit: limit,
        routing_refresh_max_targets: max_targets,
        ..dht_runner
    };
    let mut stats = run_dht_routing_refresh_with_logger(node, &peers, config, None);
    stats.peers_quarantined = peers_quarantined;
    if let Some(runtime_stats) = runtime_stats {
        runtime_stats.record_dht_routing_refresh_run(stats, current_unix_timestamp());
    }
    ControlHttpResponse::json(
        200,
        &DhtRoutingRefreshRunResponse {
            peers: peers.len(),
            routing_peers: node.kademlia.len(),
            stats,
        },
    )
}

pub(super) fn handle_control_dht_find_value_run(
    node: &mut NativeNode,
    dht_configured_peers: &[SyncPeerConfig],
    dht_runner: DhtRunnerConfig,
    path: &str,
    runtime_stats: Option<&mut ControlRuntimeStats>,
) -> ControlHttpResponse {
    let key = match dht_record_key_from_find_value_query(path) {
        Ok(key) => key,
        Err(err) => return ControlHttpResponse::text(400, err),
    };
    let limit = query_param_value(path, "limit")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(dht_runner.routing_refresh_limit)
        .clamp(1, MAX_DHT_RESPONSE_RECORDS);
    let max_peers = query_param_value(path, "max_peers")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(dht_runner.routing_refresh_max_targets)
        .clamp(1, 64);
    let alpha = query_param_value(path, "alpha")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(3)
        .clamp(1, 16);
    let (peers, peers_quarantined) = dht_runner_peer_configs_with_quarantine_count(
        node,
        dht_configured_peers,
        dht_runner.transport,
        dht_runner.peer_quarantine_consecutive_failures,
    );
    let mut stats = match dht_runner.transport {
        DhtTransportKind::HttpControl => dht_find_value_with_transport(
            node,
            &peers,
            key,
            limit,
            max_peers,
            alpha,
            &HttpControlDhtTransport,
        ),
        DhtTransportKind::Libp2p => dht_find_value_with_transport(
            node,
            &peers,
            key,
            limit,
            max_peers,
            alpha,
            &Libp2pDhtTransport::default(),
        ),
    };
    stats.peers_quarantined = peers_quarantined;
    stats.quarantine_threshold = dht_runner.peer_quarantine_consecutive_failures;
    if let Some(runtime_stats) = runtime_stats {
        runtime_stats.record_dht_find_value_run(stats, current_unix_timestamp());
    }
    let record = node.dht_records.find_value(&key);
    let found = record.is_some();
    ControlHttpResponse::json(
        200,
        &DhtFindValueRunResponse {
            key: key.to_hex(),
            found,
            record,
            records: node.dht_records.len(),
            stats,
        },
    )
}

pub(super) fn dht_record_key_from_find_value_query(
    path: &str,
) -> Result<lm_node::DhtRecordKey, String> {
    if let Some(key_hex) = query_param_value(path, "key") {
        return lm_node::DhtRecordKey::from_hex(&key_hex).map_err(|err| err.to_string());
    }
    let kind = query_param_value(path, "kind").ok_or_else(|| "missing key or kind".to_string())?;
    let value = query_param_value(path, "value").ok_or_else(|| "missing value".to_string())?;
    let value = value.trim();
    if value.is_empty() {
        return Err("missing value".into());
    }
    match kind.trim().to_ascii_lowercase().as_str() {
        "public-peer" | "public_peer" | "peer" => Ok(lm_node::DhtRecordKey::for_public_peer(value)),
        "prekey" | "pre-key" => {
            let user_id = UserId::from_raw(value.to_string()).map_err(|err| err.to_string())?;
            Ok(lm_node::DhtRecordKey::for_prekey(&user_id))
        }
        "mailbox-hint" | "mailbox_hint" | "mailbox" => {
            let user_id = UserId::from_raw(value.to_string()).map_err(|err| err.to_string())?;
            Ok(lm_node::DhtRecordKey::for_mailbox_hint(&user_id))
        }
        _ => Err("unsupported dht key kind; expected public-peer, prekey, or mailbox-hint".into()),
    }
}

pub(super) fn query_param_value(path: &str, name: &str) -> Option<String> {
    let (_, query) = path.split_once('?')?;
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        if key == name {
            return Some(percent_decode_query_component(value));
        }
    }
    None
}

pub(super) fn percent_decode_query_component(value: &str) -> String {
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

pub(super) fn from_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

pub(super) fn control_endpoint_key(request: &ControlRequest) -> String {
    let path = request
        .path
        .split_once('?')
        .map(|(path, _)| path)
        .unwrap_or(&request.path);
    format!("{} {}", request.method, path)
}

pub(super) fn request_is_within_rate_limit(
    request: &ControlRequest,
    peer_addr: Option<&std::net::SocketAddr>,
    rate_limiter: &mut RateLimiter,
    rate_limit: RateLimitConfig,
) -> bool {
    if request.method == "GET" && request.path.starts_with("/health") {
        return true;
    }
    let Some(peer_addr) = peer_addr else {
        return true;
    };
    rate_limiter.check(peer_addr.ip(), Instant::now(), rate_limit)
}

pub(super) fn request_is_authorized(
    request: &ControlRequest,
    security: &ControlSecurityConfig,
    peer_addr: Option<&std::net::SocketAddr>,
) -> bool {
    if request.method == "GET" && request.path.starts_with("/health") {
        return true;
    }
    if security.has_bearer_tokens() {
        return request
            .header("authorization")
            .and_then(|value| value.strip_prefix("Bearer "))
            .map(|value| security.token_matches(value))
            .unwrap_or(false);
    }
    security.is_loopback_only()
        && peer_addr
            .map(|addr| addr.ip().is_loopback())
            .unwrap_or(false)
}

pub(super) fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (left, right) in a.iter().zip(b.iter()) {
        diff |= left ^ right;
    }
    diff == 0
}

pub(super) fn read_http_request(
    stream: &mut TcpStream,
) -> Result<ControlRequest, Box<dyn std::error::Error>> {
    let mut buffer = Vec::new();
    let mut temp = [0u8; 4096];
    let header_end;
    loop {
        let n = stream.read(&mut temp)?;
        if n == 0 {
            return Err("connection closed before headers".into());
        }
        buffer.extend_from_slice(&temp[..n]);
        if let Some(pos) = find_header_end(&buffer) {
            header_end = pos;
            break;
        }
        if buffer.len() > MAX_CONTROL_REQUEST_HEADER_BYTES {
            return Err("request header too large".into());
        }
    }
    let headers = String::from_utf8_lossy(&buffer[..header_end]).into_owned();
    let mut lines = headers.lines();
    let request_line = lines.next().ok_or("missing request line")?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or("missing method")?.to_string();
    let path = parts.next().ok_or("missing path")?.to_string();
    let version = parts.next().ok_or("missing http version")?;
    if parts.next().is_some() {
        return Err("invalid request line".into());
    }
    if !version.starts_with("HTTP/1.") {
        return Err("unsupported http version".into());
    }
    if method.len() > MAX_CONTROL_REQUEST_METHOD_BYTES {
        return Err("request method too large".into());
    }
    if path.len() > MAX_CONTROL_REQUEST_PATH_BYTES {
        return Err("request path too large".into());
    }
    let content_length = parse_content_length_and_validate_headers(&headers)?;
    if content_length > MAX_CONTROL_REQUEST_BODY_BYTES {
        return Err("request body too large".into());
    }
    let body_start = header_end + 4;
    while buffer.len() < body_start + content_length {
        let n = stream.read(&mut temp)?;
        if n == 0 {
            return Err("connection closed before body".into());
        }
        buffer.extend_from_slice(&temp[..n]);
    }
    let body = String::from_utf8(buffer[body_start..body_start + content_length].to_vec())?;
    Ok(ControlRequest {
        method,
        path,
        body,
        headers: parse_headers(&headers),
    })
}

pub(super) fn parse_headers(headers: &str) -> Vec<(String, String)> {
    headers
        .lines()
        .skip(1)
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_string()))
        .collect()
}

pub(super) fn parse_content_length_and_validate_headers(
    headers: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut content_length: Option<usize> = None;
    for line in headers.lines().skip(1) {
        if line.len() > MAX_CONTROL_REQUEST_HEADER_LINE_BYTES {
            return Err("request header too large".into());
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err("invalid header line".into());
        };
        if name.trim().is_empty() {
            return Err("invalid header name".into());
        }
        if name.eq_ignore_ascii_case("transfer-encoding") && !value.trim().is_empty() {
            return Err("unsupported transfer-encoding".into());
        }
        if name.eq_ignore_ascii_case("content-length") {
            let parsed = value.trim().parse::<usize>()?;
            if let Some(previous) = content_length {
                if previous != parsed {
                    return Err("conflicting content-length".into());
                }
            } else {
                content_length = Some(parsed);
            }
        }
    }
    Ok(content_length.unwrap_or(0))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ControlHttpResponse {
    pub(crate) status: u16,
    pub(crate) content_type: String,
    pub(crate) body: String,
}

impl ControlHttpResponse {
    pub(super) fn json<T: Serialize>(status: u16, value: &T) -> Self {
        match serde_json::to_string_pretty(value) {
            Ok(body) => Self {
                status,
                content_type: "application/json; charset=utf-8".to_string(),
                body,
            },
            Err(err) => Self::text(500, format!("serialization error: {err}")),
        }
    }

    pub(super) fn openmetrics(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            content_type: "application/openmetrics-text; version=1.0.0; charset=utf-8".to_string(),
            body: body.into(),
        }
    }

    pub(super) fn from_control(response: lm_node::ControlResponse) -> Self {
        Self {
            status: response.status,
            content_type: response.content_type,
            body: response.body,
        }
    }

    pub(super) fn text(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            content_type: "text/plain; charset=utf-8".to_string(),
            body: body.into(),
        }
    }

    pub(super) fn to_http_string(&self, access_control_origin: &str) -> String {
        let reason = match self.status {
            200 => "OK",
            201 => "Created",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            429 => "Too Many Requests",
            500 => "Internal Server Error",
            _ => "OK",
        };
        format!(
            "HTTP/1.1 {} {}\r\ncontent-type: {}\r\ncache-control: no-store\r\nx-content-type-options: nosniff\r\nreferrer-policy: no-referrer\r\npermissions-policy: camera=(), microphone=(), geolocation=(), payment=(), usb=()\r\ncontent-security-policy: default-src 'none'; frame-ancestors 'none'; base-uri 'none'\r\naccess-control-allow-origin: {}\r\naccess-control-allow-methods: GET,POST,OPTIONS\r\naccess-control-allow-headers: content-type,authorization\r\naccess-control-allow-private-network: true\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            self.status,
            reason,
            self.content_type,
            access_control_origin,
            self.body.len(),
            self.body
        )
    }
}

pub(super) fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}
