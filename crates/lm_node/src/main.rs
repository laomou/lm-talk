use lm_core::PublicPeerAnnounce;
use lm_node::{
    ControlRequest, NativeNode, NodeConfig, NodeMaintenanceStats, NodeStateSnapshot,
    decode_identity_public_key_base64, parse_capabilities_csv, restore_identity_from_backup_text,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env, fs,
    fs::File,
    io::{Read, Write},
    net::{IpAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    process,
    time::{Duration, Instant},
};

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        print_help();
        return Ok(());
    }
    let cmd = args.remove(0);

    match cmd.as_str() {
        "announce" => {
            let backup_file = required_arg(&args, "--backup-file")?;
            let passphrase = required_arg(&args, "--passphrase")?;
            let peer_id = optional_arg(&args, "--peer-id")?.unwrap_or("lm-node".into());
            let addresses = optional_arg(&args, "--addr")?
                .map(|value| value.split(',').map(str::to_string).collect())
                .unwrap_or_else(|| vec!["/ip4/0.0.0.0/tcp/4001".to_string()]);
            let capabilities = optional_arg(&args, "--cap")?
                .map(|value| parse_capabilities_csv(&value))
                .transpose()?
                .unwrap_or_else(|| NodeConfig::default().capabilities);
            let backup_text = fs::read_to_string(backup_file)?;
            let identity = restore_identity_from_backup_text(backup_text.trim(), &passphrase)?;
            let node = NativeNode::new(NodeConfig {
                peer_id,
                addresses,
                capabilities,
                ..Default::default()
            });
            println!("{}", node.local_announce(&identity)?.to_export_text()?);
        }
        "inspect-public" => {
            let text_file = required_arg(&args, "--text-file")?;
            let public_key = required_arg(&args, "--identity-public-key")?;
            let text = fs::read_to_string(text_file)?;
            let announce = PublicPeerAnnounce::from_export_text(text.trim())?;
            let pk = decode_identity_public_key_base64(&public_key)?;
            announce.verify(&pk)?;
            println!("{}", serde_json::to_string_pretty(&announce)?);
        }
        "run" => {
            let peer_id = optional_arg(&args, "--peer-id")?.unwrap_or("lm-node-dev".into());
            let addr = optional_arg(&args, "--addr")?.unwrap_or("/ip4/0.0.0.0/tcp/4001".into());
            let node = NativeNode::new(NodeConfig {
                peer_id,
                addresses: vec![addr],
                ..Default::default()
            });
            println!("LM Talk native node scaffold");
            println!("peer_id={}", node.config.peer_id);
            println!("node_id={}", node.kademlia.local_id());
            println!("addresses={}", node.config.addresses.join(","));
            println!("capabilities={:?}", node.config.capabilities);
            println!("k_bucket_size={}", lm_node::DEFAULT_K_BUCKET_SIZE);
            println!("status=transport-not-yet-enabled");
        }
        "distance" => {
            let a = required_arg(&args, "--a")?;
            let b = required_arg(&args, "--b")?;
            let a_id = lm_node::KademliaNodeId::from_peer_id(&a);
            let b_id = lm_node::KademliaNodeId::from_peer_id(&b);
            let distance = a_id.xor_distance(&b_id);
            println!("a_id={a_id}");
            println!("b_id={b_id}");
            println!("distance={}", distance.to_hex());
            println!("bucket_index={:?}", a_id.bucket_index(&b_id));
        }
        "serve-control" => {
            let config_file = optional_arg(&args, "--config-file")?
                .or_else(|| env::var("LM_NODE_CONFIG_FILE").ok());
            let file_config = match &config_file {
                Some(path) => ServeControlConfigFile::load(path)?,
                None => ServeControlConfigFile::default(),
            };
            let bind = optional_arg(&args, "--bind")?
                .or(file_config.bind)
                .unwrap_or("127.0.0.1:8787".into());
            let peer_id = optional_arg(&args, "--peer-id")?
                .or(file_config.peer_id)
                .unwrap_or("lm-node-dev".into());
            let state_file = optional_arg(&args, "--state-file")?.or(file_config.state_file);
            let sync_peer_token_direct = optional_arg(&args, "--sync-peer-token")?
                .or_else(|| env::var("LM_NODE_SYNC_PEER_TOKEN").ok());
            let sync_peer_token_from_file = optional_secret_file_arg(
                &args,
                "--sync-peer-token-file",
                "LM_NODE_SYNC_PEER_TOKEN_FILE",
                None,
            )?;
            let sync_peer_token = choose_secret(sync_peer_token_direct, sync_peer_token_from_file);
            let mut sync_peers = Vec::new();
            for peer in file_config.sync_peers.unwrap_or_default() {
                let token_from_file = peer
                    .token_file
                    .as_deref()
                    .map(read_secret_file)
                    .transpose()?;
                sync_peers.push(SyncPeerConfig {
                    url: peer.url,
                    token: choose_secret(peer.token, token_from_file),
                });
            }
            if let Some(sync_peer_urls) = optional_arg(&args, "--sync-peer")? {
                sync_peers = parse_csv(&sync_peer_urls)
                    .into_iter()
                    .map(|url| SyncPeerConfig {
                        url,
                        token: sync_peer_token.clone(),
                    })
                    .collect();
            } else if let Some(token) = sync_peer_token {
                for peer in &mut sync_peers {
                    if peer.token.is_none() {
                        peer.token = Some(token.clone());
                    }
                }
            }
            let sync_interval_seconds = optional_arg(&args, "--sync-interval-seconds")?
                .map(|value| value.parse::<u64>())
                .transpose()?
                .or(file_config.sync_interval_seconds)
                .unwrap_or(0);
            let sync_max_backoff_seconds = optional_arg(&args, "--sync-max-backoff-seconds")?
                .map(|value| value.parse::<u64>())
                .transpose()?
                .or(file_config.sync_max_backoff_seconds)
                .unwrap_or(300);
            let rate_limit_window_seconds = optional_arg(&args, "--rate-limit-window-seconds")?
                .or_else(|| env::var("LM_NODE_RATE_LIMIT_WINDOW_SECONDS").ok())
                .map(|value| value.parse::<u64>())
                .transpose()?
                .or(file_config.rate_limit_window_seconds)
                .unwrap_or(60);
            let rate_limit_max_requests = optional_arg(&args, "--rate-limit-max-requests")?
                .or_else(|| env::var("LM_NODE_RATE_LIMIT_MAX_REQUESTS").ok())
                .map(|value| value.parse::<u32>())
                .transpose()?
                .or(file_config.rate_limit_max_requests)
                .unwrap_or(600);
            let control_token_direct = optional_arg(&args, "--control-token")?
                .or_else(|| env::var("LM_NODE_CONTROL_TOKEN").ok())
                .or(file_config.control_token);
            let control_token_from_file = optional_secret_file_arg(
                &args,
                "--control-token-file",
                "LM_NODE_CONTROL_TOKEN_FILE",
                file_config.control_token_file,
            )?;
            let token = choose_secret(control_token_direct, control_token_from_file);
            let cors_allow_origins = optional_arg(&args, "--cors-allow-origin")?
                .or_else(|| env::var("LM_NODE_CORS_ALLOW_ORIGIN").ok())
                .map(|value| parse_csv(&value))
                .or(file_config.cors_allow_origins)
                .unwrap_or_default();
            let security = ControlSecurityConfig {
                token,
                cors_allow_origins,
            };
            let mut node = if let Some(path) = &state_file {
                load_node_state(path).unwrap_or_else(|_| {
                    NativeNode::new(NodeConfig {
                        peer_id: peer_id.clone(),
                        ..Default::default()
                    })
                })
            } else {
                NativeNode::new(NodeConfig {
                    peer_id,
                    ..Default::default()
                })
            };
            serve_control(
                &bind,
                &mut node,
                state_file.as_deref(),
                sync_peers,
                sync_interval_seconds,
                sync_max_backoff_seconds,
                security,
                RateLimitConfig {
                    window_seconds: rate_limit_window_seconds,
                    max_requests: rate_limit_max_requests,
                },
            )?;
        }
        "help" | "--help" | "-h" => print_help(),
        _ => {
            print_help();
            return Err(format!("unknown command: {cmd}").into());
        }
    }
    Ok(())
}

fn required_arg(args: &[String], name: &str) -> Result<String, String> {
    optional_arg(args, name)?.ok_or_else(|| format!("missing {name}"))
}

fn optional_arg(args: &[String], name: &str) -> Result<Option<String>, String> {
    let Some(index) = args.iter().position(|arg| arg == name) else {
        return Ok(None);
    };
    args.get(index + 1)
        .cloned()
        .map(Some)
        .ok_or_else(|| format!("missing value for {name}"))
}

fn parse_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

fn read_secret_file(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let value = fs::read_to_string(path)?.trim().to_string();
    if value.is_empty() {
        Err(format!("secret file is empty: {path}").into())
    } else {
        Ok(value)
    }
}

fn optional_secret_file_arg(
    args: &[String],
    arg_name: &str,
    env_name: &str,
    config_value: Option<String>,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let path = optional_arg(args, arg_name)?
        .or_else(|| env::var(env_name).ok())
        .or(config_value);
    path.map(|path| read_secret_file(&path)).transpose()
}

fn choose_secret(direct: Option<String>, file_value: Option<String>) -> Option<String> {
    direct.or(file_value)
}

#[derive(Debug, Clone, Default)]
struct ControlSecurityConfig {
    token: Option<String>,
    cors_allow_origins: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SyncPeerConfig {
    url: String,
    token: Option<String>,
}

#[derive(Debug, Clone)]
struct SyncPeerRuntime {
    config: SyncPeerConfig,
    next_attempt_at: Instant,
    consecutive_failures: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RateLimitConfig {
    window_seconds: u64,
    max_requests: u32,
}

impl RateLimitConfig {
    fn is_enabled(self) -> bool {
        self.window_seconds > 0 && self.max_requests > 0
    }
}

#[derive(Debug, Clone)]
struct RateLimitEntry {
    window_started_at: Instant,
    count: u32,
}

#[derive(Debug, Serialize)]
struct ControlStatsResponse<'a> {
    #[serde(flatten)]
    runtime: &'a ControlRuntimeStats,
    maintenance: NodeMaintenanceStats,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct ControlRuntimeStats {
    started_at: u64,
    requests_total: u64,
    responses_2xx: u64,
    responses_4xx: u64,
    responses_5xx: u64,
    cors_rejected: u64,
    unauthorized: u64,
    rate_limited: u64,
    bad_requests: u64,
    sync_snapshot_exports: u64,
    sync_snapshot_export_bytes: u64,
    sync_snapshot_imports: u64,
    sync_snapshot_import_bytes: u64,
    endpoints: HashMap<String, ControlEndpointStats>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
struct ControlEndpointStats {
    requests: u64,
    responses_2xx: u64,
    responses_4xx: u64,
    responses_5xx: u64,
    total_duration_micros: u128,
    max_duration_micros: u128,
    last_status: Option<u16>,
}

impl ControlRuntimeStats {
    fn new(started_at: u64) -> Self {
        Self {
            started_at,
            requests_total: 0,
            responses_2xx: 0,
            responses_4xx: 0,
            responses_5xx: 0,
            cors_rejected: 0,
            unauthorized: 0,
            rate_limited: 0,
            bad_requests: 0,
            sync_snapshot_exports: 0,
            sync_snapshot_export_bytes: 0,
            sync_snapshot_imports: 0,
            sync_snapshot_import_bytes: 0,
            endpoints: HashMap::new(),
        }
    }

    fn to_openmetrics(&self, maintenance: &NodeMaintenanceStats) -> String {
        let mut out = String::new();
        push_metric_help(
            &mut out,
            "lm_node_control_started_at",
            "Unix timestamp when the control runtime started.",
        );
        push_metric_type(&mut out, "lm_node_control_started_at", "gauge");
        push_metric_value(&mut out, "lm_node_control_started_at", self.started_at);
        push_metric_help(
            &mut out,
            "lm_node_control_requests_total",
            "Total control HTTP responses by status class and security outcome.",
        );
        push_metric_type(&mut out, "lm_node_control_requests_total", "counter");
        push_metric_value(
            &mut out,
            "lm_node_control_requests_total",
            self.requests_total,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_control_responses_total",
            "class",
            "2xx",
            self.responses_2xx,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_control_responses_total",
            "class",
            "4xx",
            self.responses_4xx,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_control_responses_total",
            "class",
            "5xx",
            self.responses_5xx,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_control_security_events_total",
            "event",
            "bad_request",
            self.bad_requests,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_control_security_events_total",
            "event",
            "unauthorized",
            self.unauthorized,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_control_security_events_total",
            "event",
            "cors_rejected",
            self.cors_rejected,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_control_security_events_total",
            "event",
            "rate_limited",
            self.rate_limited,
        );
        push_metric_help(
            &mut out,
            "lm_node_control_sync_snapshots_total",
            "Successful snapshot sync import/export operations through the control plane.",
        );
        push_metric_type(&mut out, "lm_node_control_sync_snapshots_total", "counter");
        push_labeled_metric_value(
            &mut out,
            "lm_node_control_sync_snapshots_total",
            "direction",
            "export",
            self.sync_snapshot_exports,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_control_sync_snapshots_total",
            "direction",
            "import",
            self.sync_snapshot_imports,
        );
        push_metric_help(
            &mut out,
            "lm_node_control_sync_snapshot_bytes_total",
            "Snapshot sync import/export payload bytes through the control plane.",
        );
        push_metric_type(
            &mut out,
            "lm_node_control_sync_snapshot_bytes_total",
            "counter",
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_control_sync_snapshot_bytes_total",
            "direction",
            "export",
            self.sync_snapshot_export_bytes,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_control_sync_snapshot_bytes_total",
            "direction",
            "import",
            self.sync_snapshot_import_bytes,
        );
        push_metric_help(
            &mut out,
            "lm_node_maintenance_prune_runs_total",
            "Total node expired-record prune runs.",
        );
        push_metric_type(&mut out, "lm_node_maintenance_prune_runs_total", "counter");
        push_metric_value(
            &mut out,
            "lm_node_maintenance_prune_runs_total",
            maintenance.prune_runs,
        );
        push_metric_help(
            &mut out,
            "lm_node_maintenance_expired_records_total",
            "Expired records removed by node maintenance prune jobs.",
        );
        push_metric_type(
            &mut out,
            "lm_node_maintenance_expired_records_total",
            "counter",
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_maintenance_expired_records_total",
            "kind",
            "mailbox_delivery",
            maintenance.mailbox_expired_deliveries,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_maintenance_expired_records_total",
            "kind",
            "prekey_bundle",
            maintenance.prekey_expired_bundles,
        );
        if let Some(last_pruned_at) = maintenance.last_pruned_at {
            push_metric_help(
                &mut out,
                "lm_node_maintenance_last_pruned_at",
                "Unix timestamp of the last expired-record prune run.",
            );
            push_metric_type(&mut out, "lm_node_maintenance_last_pruned_at", "gauge");
            push_metric_value(
                &mut out,
                "lm_node_maintenance_last_pruned_at",
                last_pruned_at,
            );
        }
        push_metric_help(
            &mut out,
            "lm_node_control_endpoint_requests_total",
            "Control HTTP responses by endpoint and status class.",
        );
        push_metric_type(
            &mut out,
            "lm_node_control_endpoint_requests_total",
            "counter",
        );
        push_metric_help(
            &mut out,
            "lm_node_control_endpoint_duration_micros_total",
            "Total control HTTP handler duration in microseconds by endpoint.",
        );
        push_metric_type(
            &mut out,
            "lm_node_control_endpoint_duration_micros_total",
            "counter",
        );
        push_metric_help(
            &mut out,
            "lm_node_control_endpoint_duration_micros_max",
            "Maximum observed control HTTP handler duration in microseconds by endpoint.",
        );
        push_metric_type(
            &mut out,
            "lm_node_control_endpoint_duration_micros_max",
            "gauge",
        );
        let mut endpoints = self.endpoints.iter().collect::<Vec<_>>();
        endpoints.sort_by(|(left, _), (right, _)| left.cmp(right));
        for (endpoint, stats) in endpoints {
            push_endpoint_metric_value(
                &mut out,
                "lm_node_control_endpoint_requests_total",
                endpoint,
                "all",
                stats.requests,
            );
            push_endpoint_metric_value(
                &mut out,
                "lm_node_control_endpoint_requests_total",
                endpoint,
                "2xx",
                stats.responses_2xx,
            );
            push_endpoint_metric_value(
                &mut out,
                "lm_node_control_endpoint_requests_total",
                endpoint,
                "4xx",
                stats.responses_4xx,
            );
            push_endpoint_metric_value(
                &mut out,
                "lm_node_control_endpoint_requests_total",
                endpoint,
                "5xx",
                stats.responses_5xx,
            );
            push_labeled_metric_value(
                &mut out,
                "lm_node_control_endpoint_duration_micros_total",
                "endpoint",
                endpoint,
                stats.total_duration_micros,
            );
            push_labeled_metric_value(
                &mut out,
                "lm_node_control_endpoint_duration_micros_max",
                "endpoint",
                endpoint,
                stats.max_duration_micros,
            );
            if let Some(status) = stats.last_status {
                push_labeled_metric_value(
                    &mut out,
                    "lm_node_control_endpoint_last_status",
                    "endpoint",
                    endpoint,
                    status,
                );
            }
        }
        out.push_str("# EOF\n");
        out
    }

    fn record_sync_snapshot_bytes(
        &mut self,
        endpoint: &str,
        status: u16,
        request_body_bytes: usize,
        response_body_bytes: usize,
    ) {
        if !(200..=299).contains(&status) {
            return;
        }
        match endpoint {
            "GET /sync/snapshot" => {
                self.sync_snapshot_exports = self.sync_snapshot_exports.saturating_add(1);
                self.sync_snapshot_export_bytes = self
                    .sync_snapshot_export_bytes
                    .saturating_add(response_body_bytes as u64);
            }
            "POST /sync/import" => {
                self.sync_snapshot_imports = self.sync_snapshot_imports.saturating_add(1);
                self.sync_snapshot_import_bytes = self
                    .sync_snapshot_import_bytes
                    .saturating_add(request_body_bytes as u64);
            }
            _ => {}
        }
    }

    fn record_response(&mut self, endpoint: &str, status: u16, duration: Duration) {
        self.requests_total = self.requests_total.saturating_add(1);
        match status {
            200..=299 => self.responses_2xx = self.responses_2xx.saturating_add(1),
            400..=499 => self.responses_4xx = self.responses_4xx.saturating_add(1),
            500..=599 => self.responses_5xx = self.responses_5xx.saturating_add(1),
            _ => {}
        }
        match status {
            400 => self.bad_requests = self.bad_requests.saturating_add(1),
            401 => self.unauthorized = self.unauthorized.saturating_add(1),
            403 => self.cors_rejected = self.cors_rejected.saturating_add(1),
            429 => self.rate_limited = self.rate_limited.saturating_add(1),
            _ => {}
        }
        self.endpoints
            .entry(endpoint.to_string())
            .or_default()
            .record(status, duration);
    }
}

fn push_metric_help(out: &mut String, name: &str, help: &str) {
    out.push_str("# HELP ");
    out.push_str(name);
    out.push(' ');
    out.push_str(help);
    out.push('\n');
}

fn push_metric_type(out: &mut String, name: &str, kind: &str) {
    out.push_str("# TYPE ");
    out.push_str(name);
    out.push(' ');
    out.push_str(kind);
    out.push('\n');
}

fn push_metric_value(out: &mut String, name: &str, value: impl std::fmt::Display) {
    out.push_str(name);
    out.push(' ');
    out.push_str(&value.to_string());
    out.push('\n');
}

fn push_labeled_metric_value(
    out: &mut String,
    name: &str,
    label_name: &str,
    label_value: &str,
    value: impl std::fmt::Display,
) {
    out.push_str(name);
    out.push('{');
    out.push_str(label_name);
    out.push_str("=\"");
    out.push_str(&escape_openmetrics_label(label_value));
    out.push_str("\"} ");
    out.push_str(&value.to_string());
    out.push('\n');
}

fn push_endpoint_metric_value(
    out: &mut String,
    name: &str,
    endpoint: &str,
    class: &str,
    value: impl std::fmt::Display,
) {
    out.push_str(name);
    out.push_str("{endpoint=\"");
    out.push_str(&escape_openmetrics_label(endpoint));
    out.push_str("\",class=\"");
    out.push_str(&escape_openmetrics_label(class));
    out.push_str("\"} ");
    out.push_str(&value.to_string());
    out.push('\n');
}

fn escape_openmetrics_label(value: &str) -> String {
    value
        .chars()
        .flat_map(|ch| match ch {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            _ => vec![ch],
        })
        .collect()
}

impl ControlEndpointStats {
    fn record(&mut self, status: u16, duration: Duration) {
        self.requests = self.requests.saturating_add(1);
        match status {
            200..=299 => self.responses_2xx = self.responses_2xx.saturating_add(1),
            400..=499 => self.responses_4xx = self.responses_4xx.saturating_add(1),
            500..=599 => self.responses_5xx = self.responses_5xx.saturating_add(1),
            _ => {}
        }
        let micros = duration.as_micros();
        self.total_duration_micros = self.total_duration_micros.saturating_add(micros);
        self.max_duration_micros = self.max_duration_micros.max(micros);
        self.last_status = Some(status);
    }
}

#[derive(Debug, Default)]
struct RateLimiter {
    entries: HashMap<IpAddr, RateLimitEntry>,
}

impl RateLimiter {
    fn check(&mut self, ip: IpAddr, now: Instant, config: RateLimitConfig) -> bool {
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

    fn prune(&mut self, now: Instant, config: RateLimitConfig) {
        if !config.is_enabled() {
            self.entries.clear();
            return;
        }
        let ttl = Duration::from_secs(config.window_seconds.saturating_mul(2).max(1));
        self.entries
            .retain(|_, entry| now.duration_since(entry.window_started_at) < ttl);
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
struct ServeControlConfigFile {
    bind: Option<String>,
    peer_id: Option<String>,
    state_file: Option<String>,
    control_token: Option<String>,
    control_token_file: Option<String>,
    cors_allow_origins: Option<Vec<String>>,
    sync_peers: Option<Vec<SyncPeerConfigFile>>,
    sync_interval_seconds: Option<u64>,
    sync_max_backoff_seconds: Option<u64>,
    rate_limit_window_seconds: Option<u64>,
    rate_limit_max_requests: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
struct SyncPeerConfigFile {
    url: String,
    token: Option<String>,
    token_file: Option<String>,
}

impl ServeControlConfigFile {
    fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let text = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&text)?)
    }
}

impl ControlSecurityConfig {
    fn is_loopback_only(&self) -> bool {
        self.token.is_none()
    }

    fn allows_origin(&self, origin: Option<&str>) -> bool {
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

    fn access_control_origin(&self, request_origin: Option<&str>) -> String {
        if self.cors_allow_origins.is_empty() || self.cors_allow_origins.iter().any(|v| v == "*") {
            "*".to_string()
        } else {
            request_origin.unwrap_or("null").to_string()
        }
    }
}

fn load_node_state(path: &str) -> Result<NativeNode, Box<dyn std::error::Error>> {
    let text = fs::read_to_string(path)?;
    let snapshot: NodeStateSnapshot = serde_json::from_str(&text)?;
    Ok(NativeNode::from_state_snapshot(snapshot))
}

fn save_node_state(path: &str, node: &NativeNode) -> Result<(), Box<dyn std::error::Error>> {
    let text = serde_json::to_string_pretty(&node.to_state_snapshot())?;
    atomic_write_text(Path::new(path), &text)?;
    Ok(())
}

fn atomic_write_text(path: &Path, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let tmp_path = atomic_temp_path(path);
    {
        let mut file = File::create(&tmp_path)?;
        file.write_all(text.as_bytes())?;
        file.write_all(b"\n")?;
        file.sync_all()?;
    }
    fs::rename(&tmp_path, path)?;
    sync_parent_dir(path);
    Ok(())
}

fn atomic_temp_path(path: &Path) -> PathBuf {
    let mut file_name = path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_else(|| "lm-node-state".into());
    file_name.push(format!(
        ".tmp.{}.{}",
        process::id(),
        current_unix_timestamp()
    ));
    path.with_file_name(file_name)
}

fn sync_parent_dir(path: &Path) {
    #[cfg(unix)]
    {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            if let Ok(dir) = File::open(parent) {
                let _ = dir.sync_all();
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

fn serve_control(
    bind: &str,
    node: &mut NativeNode,
    state_file: Option<&str>,
    sync_peers: Vec<SyncPeerConfig>,
    sync_interval_seconds: u64,
    sync_max_backoff_seconds: u64,
    security: ControlSecurityConfig,
    rate_limit: RateLimitConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(bind)?;
    listener.set_nonblocking(true)?;
    let mut rate_limiter = RateLimiter::default();
    let mut runtime_stats = ControlRuntimeStats::new(current_unix_timestamp());
    let mut last_rate_limit_prune = Instant::now();
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
    println!("LM Talk control plane listening on http://{bind}");
    println!(
        "endpoints: GET /health, GET /control/stats, GET /control/metrics, POST /announce, GET /peers/closest, POST /mailbox/push, GET /mailbox/take, POST /mailbox/ack, POST /prekey/publish, GET /prekey/get, POST/GET /dht/record, GET /dht/closest, POST /dht/rpc, GET /sync/snapshot, GET /sync/status, POST /sync/import"
    );
    if security.token.is_some() {
        println!("control security: bearer token required");
    } else {
        println!("control security: no token configured; loopback clients only");
    }
    if !security.cors_allow_origins.is_empty() {
        println!(
            "CORS allow origins: {}",
            security.cors_allow_origins.join(",")
        );
    }
    if rate_limit.is_enabled() {
        println!(
            "control rate limit: {} requests / {}s per client IP",
            rate_limit.max_requests, rate_limit.window_seconds
        );
    } else {
        println!("control rate limit: disabled");
    }
    if !sync_peers.is_empty() && sync_interval_seconds > 0 {
        println!(
            "auto snapshot sync: every {sync_interval_seconds}s from {}",
            sync_peers
                .iter()
                .map(|peer| peer.config.url.as_str())
                .collect::<Vec<_>>()
                .join(",")
        );
    }
    loop {
        let now = Instant::now();
        let mut sync_ran = false;
        for peer in &mut sync_peers {
            if now >= peer.next_attempt_at {
                run_snapshot_sync(node, peer, sync_interval_seconds, sync_max_backoff_seconds);
                sync_ran = true;
            }
        }
        if sync_ran {
            if let Some(path) = state_file {
                if let Err(err) = save_node_state(path, node) {
                    eprintln!("state save error: {err}");
                }
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
                ) {
                    runtime_stats.record_response("<bad-request>", 400, Duration::ZERO);
                    let body = format!("request error: {err}");
                    let response = format!(
                        "HTTP/1.1 400 Bad Request\r\ncontent-type: text/plain; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(response.as_bytes());
                } else if let Some(path) = state_file {
                    if let Err(err) = save_node_state(path, node) {
                        eprintln!("state save error: {err}");
                    }
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(err) => eprintln!("connection error: {err}"),
        }
    }
}

fn run_snapshot_sync(
    node: &mut NativeNode,
    peer: &mut SyncPeerRuntime,
    base_interval_seconds: u64,
    max_backoff_seconds: u64,
) {
    let delay_seconds;
    match fetch_snapshot(&peer.config) {
        Ok(snapshot) => {
            let stats = node.merge_snapshot(snapshot);
            node.sync_status.record_success(&peer.config.url, stats);
            peer.consecutive_failures = 0;
            delay_seconds = base_interval_seconds.max(1);
            println!(
                "snapshot sync from {}: peers={} mailbox_deliveries={} prekey_bundles={}",
                peer.config.url, stats.peers, stats.mailbox_deliveries, stats.prekey_bundles
            );
        }
        Err(err) => {
            let error = err.to_string();
            node.sync_status
                .record_failure(&peer.config.url, error.clone());
            peer.consecutive_failures = peer.consecutive_failures.saturating_add(1);
            delay_seconds = sync_backoff_delay_seconds(
                base_interval_seconds.max(1),
                max_backoff_seconds.max(1),
                peer.consecutive_failures,
            );
            eprintln!("snapshot sync from {} failed: {error}", peer.config.url);
        }
    }
    peer.next_attempt_at = Instant::now() + Duration::from_secs(delay_seconds);
    node.sync_status.record_next_attempt(
        &peer.config.url,
        current_unix_timestamp().saturating_add(delay_seconds),
    );
}

fn sync_backoff_delay_seconds(base: u64, max: u64, consecutive_failures: u32) -> u64 {
    if consecutive_failures == 0 {
        return base.max(1).min(max.max(1));
    }
    let exponent = consecutive_failures.saturating_sub(1).min(20);
    base.max(1).saturating_mul(1u64 << exponent).min(max.max(1))
}

fn current_unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn fetch_snapshot(peer: &SyncPeerConfig) -> Result<NodeStateSnapshot, Box<dyn std::error::Error>> {
    let normalized = peer.url.trim().trim_end_matches('/');
    let without_scheme = normalized
        .strip_prefix("http://")
        .ok_or("only http:// sync peers are supported")?;
    let (host_port, path_prefix) = without_scheme
        .split_once('/')
        .map(|(host, path)| (host, format!("/{path}")))
        .unwrap_or((without_scheme, String::new()));
    let path = format!("{path_prefix}/sync/snapshot");
    let mut stream = TcpStream::connect(host_port)?;
    let auth_header = peer
        .token
        .as_ref()
        .map(|token| format!("authorization: Bearer {token}\r\n"))
        .unwrap_or_default();
    let request = format!(
        "GET {path} HTTP/1.1\r\nhost: {host_port}\r\n{auth_header}connection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes())?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    let response = String::from_utf8(response)?;
    let (headers, body) = response
        .split_once("\r\n\r\n")
        .ok_or("invalid http response")?;
    let status_line = headers.lines().next().ok_or("missing status line")?;
    if !status_line.contains(" 200 ") {
        return Err(format!("sync peer returned {status_line}").into());
    }
    Ok(serde_json::from_str(body)?)
}

fn handle_stream(
    stream: &mut TcpStream,
    node: &mut NativeNode,
    security: &ControlSecurityConfig,
    rate_limiter: &mut RateLimiter,
    rate_limit: RateLimitConfig,
    runtime_stats: &mut ControlRuntimeStats,
) -> Result<(), Box<dyn std::error::Error>> {
    let peer_addr = stream.peer_addr().ok();
    let request = read_http_request(stream)?;
    let started_at = Instant::now();
    let endpoint = control_endpoint_key(&request);
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
            },
        )
    } else if request.method == "GET" && request.path.starts_with("/control/metrics") {
        node.prune_expired_records();
        ControlHttpResponse::openmetrics(
            200,
            &runtime_stats.to_openmetrics(node.maintenance_stats()),
        )
    } else {
        ControlHttpResponse::from_control(node.handle_control_request(request))
    };
    runtime_stats.record_response(&endpoint, response.status, started_at.elapsed());
    runtime_stats.record_sync_snapshot_bytes(
        &endpoint,
        response.status,
        request_body_bytes,
        response.body.len(),
    );
    let allow_origin = security.access_control_origin(origin.as_deref());
    stream.write_all(response.to_http_string(&allow_origin).as_bytes())?;
    Ok(())
}

fn control_endpoint_key(request: &ControlRequest) -> String {
    let path = request
        .path
        .split_once('?')
        .map(|(path, _)| path)
        .unwrap_or(&request.path);
    format!("{} {}", request.method, path)
}

fn request_is_within_rate_limit(
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

fn request_is_authorized(
    request: &ControlRequest,
    security: &ControlSecurityConfig,
    peer_addr: Option<&std::net::SocketAddr>,
) -> bool {
    if request.method == "GET" && request.path.starts_with("/health") {
        return true;
    }
    if let Some(token) = &security.token {
        return request
            .header("authorization")
            .and_then(|value| value.strip_prefix("Bearer "))
            .map(|value| constant_time_eq(value.as_bytes(), token.as_bytes()))
            .unwrap_or(false);
    }
    security.is_loopback_only()
        && peer_addr
            .map(|addr| addr.ip().is_loopback())
            .unwrap_or(false)
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (left, right) in a.iter().zip(b.iter()) {
        diff |= left ^ right;
    }
    diff == 0
}

fn read_http_request(stream: &mut TcpStream) -> Result<ControlRequest, Box<dyn std::error::Error>> {
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
        if buffer.len() > 1024 * 1024 {
            return Err("request header too large".into());
        }
    }
    let headers = String::from_utf8_lossy(&buffer[..header_end]).into_owned();
    let mut lines = headers.lines();
    let request_line = lines.next().ok_or("missing request line")?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or("missing method")?.to_string();
    let path = parts.next().ok_or("missing path")?.to_string();
    let mut content_length = 0usize;
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse()?;
            }
        }
    }
    if content_length > 4 * 1024 * 1024 {
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

fn parse_headers(headers: &str) -> Vec<(String, String)> {
    headers
        .lines()
        .skip(1)
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_string()))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ControlHttpResponse {
    status: u16,
    content_type: String,
    body: String,
}

impl ControlHttpResponse {
    fn json<T: Serialize>(status: u16, value: &T) -> Self {
        match serde_json::to_string_pretty(value) {
            Ok(body) => Self {
                status,
                content_type: "application/json; charset=utf-8".to_string(),
                body,
            },
            Err(err) => Self::text(500, format!("serialization error: {err}")),
        }
    }

    fn openmetrics(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            content_type: "application/openmetrics-text; version=1.0.0; charset=utf-8".to_string(),
            body: body.into(),
        }
    }

    fn from_control(response: lm_node::ControlResponse) -> Self {
        Self {
            status: response.status,
            content_type: response.content_type,
            body: response.body,
        }
    }

    fn text(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            content_type: "text/plain; charset=utf-8".to_string(),
            body: body.into(),
        }
    }

    fn to_http_string(&self, access_control_origin: &str) -> String {
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
            "HTTP/1.1 {} {}\r\ncontent-type: {}\r\naccess-control-allow-origin: {}\r\naccess-control-allow-methods: GET,POST,OPTIONS\r\naccess-control-allow-headers: content-type,authorization\r\naccess-control-allow-private-network: true\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            self.status,
            reason,
            self.content_type,
            access_control_origin,
            self.body.len(),
            self.body
        )
    }
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn print_help() {
    eprintln!(
        "LM Talk node scaffold\n\n\
Commands:\n  \
announce --backup-file <file> --passphrase <text> [--peer-id <id>] [--addr <multiaddr,csv>] [--cap <bootstrap,dht,relay,mailbox>]\n  \
inspect-public --text-file <file> --identity-public-key <base64>\n  \
run [--peer-id <id>] [--addr <multiaddr>]\n  \
serve-control [--config-file <json>] [--bind <host:port>] [--peer-id <id>] [--state-file <file>] [--sync-peer <url,csv>] [--sync-interval-seconds <n>] [--rate-limit-window-seconds <n>] [--rate-limit-max-requests <n>]\n"
    );
}

#[cfg(test)]
mod tests {
    use super::{
        ControlRuntimeStats, NodeMaintenanceStats, RateLimitConfig, RateLimiter,
        ServeControlConfigFile, atomic_write_text, current_unix_timestamp, read_secret_file,
        sync_backoff_delay_seconds,
    };

    #[test]
    fn sync_backoff_is_exponential_and_capped() {
        assert_eq!(sync_backoff_delay_seconds(10, 300, 0), 10);
        assert_eq!(sync_backoff_delay_seconds(10, 300, 1), 10);
        assert_eq!(sync_backoff_delay_seconds(10, 300, 2), 20);
        assert_eq!(sync_backoff_delay_seconds(10, 300, 3), 40);
        assert_eq!(sync_backoff_delay_seconds(10, 30, 4), 30);
    }

    #[test]
    fn control_runtime_stats_counts_status_classes_and_security_events() {
        let mut stats = ControlRuntimeStats::new(123);
        stats.record_response("GET /health", 200, std::time::Duration::from_micros(10));
        stats.record_response("GET /health", 201, std::time::Duration::from_micros(20));
        stats.record_response(
            "POST /mailbox/push",
            400,
            std::time::Duration::from_micros(5),
        );
        stats.record_response("GET /sync/status", 401, std::time::Duration::from_micros(1));
        stats.record_response("GET /sync/status", 403, std::time::Duration::from_micros(2));
        stats.record_response("GET /sync/status", 429, std::time::Duration::from_micros(3));
        stats.record_response(
            "GET /control/stats",
            500,
            std::time::Duration::from_micros(4),
        );

        assert_eq!(stats.started_at, 123);
        assert_eq!(stats.requests_total, 7);
        assert_eq!(stats.responses_2xx, 2);
        assert_eq!(stats.responses_4xx, 4);
        assert_eq!(stats.responses_5xx, 1);
        assert_eq!(stats.bad_requests, 1);
        assert_eq!(stats.unauthorized, 1);
        assert_eq!(stats.cors_rejected, 1);
        assert_eq!(stats.rate_limited, 1);
        stats.record_sync_snapshot_bytes("GET /sync/snapshot", 200, 0, 321);
        stats.record_sync_snapshot_bytes("POST /sync/import", 200, 123, 10);
        stats.record_sync_snapshot_bytes("POST /sync/import", 400, 999, 10);
        assert_eq!(stats.sync_snapshot_exports, 1);
        assert_eq!(stats.sync_snapshot_export_bytes, 321);
        assert_eq!(stats.sync_snapshot_imports, 1);
        assert_eq!(stats.sync_snapshot_import_bytes, 123);
        let health = stats.endpoints.get("GET /health").unwrap();
        assert_eq!(health.requests, 2);
        assert_eq!(health.responses_2xx, 2);
        assert_eq!(health.total_duration_micros, 30);
        assert_eq!(health.max_duration_micros, 20);
        assert_eq!(health.last_status, Some(201));
        let sync = stats.endpoints.get("GET /sync/status").unwrap();
        assert_eq!(sync.requests, 3);
        assert_eq!(sync.responses_4xx, 3);
        let metrics = stats.to_openmetrics(&NodeMaintenanceStats {
            prune_runs: 2,
            mailbox_expired_deliveries: 3,
            prekey_expired_bundles: 4,
            last_pruned_at: Some(1234),
        });
        assert!(metrics.contains("# TYPE lm_node_control_requests_total counter"));
        assert!(metrics.contains("lm_node_control_requests_total 7"));
        assert!(
            metrics.contains("lm_node_control_security_events_total{event=\"rate_limited\"} 1")
        );
        assert!(metrics.contains(
            "lm_node_control_endpoint_requests_total{endpoint=\"GET /sync/status\",class=\"4xx\"} 3"
        ));
        assert!(metrics.contains("lm_node_maintenance_prune_runs_total 2"));
        assert!(
            metrics
                .contains("lm_node_maintenance_expired_records_total{kind=\"mailbox_delivery\"} 3")
        );
        assert!(metrics.ends_with("# EOF\n"));
    }

    #[test]
    fn rate_limiter_enforces_window_and_resets() {
        let mut limiter = RateLimiter::default();
        let config = RateLimitConfig {
            window_seconds: 10,
            max_requests: 2,
        };
        let ip = "127.0.0.1".parse().unwrap();
        let now = std::time::Instant::now();
        assert!(limiter.check(ip, now, config));
        assert!(limiter.check(ip, now, config));
        assert!(!limiter.check(ip, now, config));
        assert!(limiter.check(ip, now + std::time::Duration::from_secs(10), config));
    }

    #[test]
    fn atomic_write_text_replaces_existing_file_and_cleans_temp() {
        let dir = std::env::temp_dir().join(format!(
            "lm-node-atomic-save-test-{}-{}",
            std::process::id(),
            current_unix_timestamp()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        std::fs::write(&path, "old").unwrap();

        atomic_write_text(&path, "new-state").unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new-state\n");
        let leftovers = std::fs::read_dir(&dir).unwrap().count();
        assert_eq!(leftovers, 1);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn secret_file_loader_trims_and_rejects_empty_files() {
        let path = std::env::temp_dir().join(format!(
            "lm-node-secret-test-{}-{}.txt",
            std::process::id(),
            current_unix_timestamp()
        ));
        std::fs::write(&path, "  secret-value\n").unwrap();
        assert_eq!(
            read_secret_file(path.to_str().unwrap()).unwrap(),
            "secret-value"
        );
        std::fs::write(&path, "  \n").unwrap();
        assert!(read_secret_file(path.to_str().unwrap()).is_err());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn serve_control_config_file_parses_sync_and_security() {
        let config: ServeControlConfigFile = serde_json::from_str(
            r#"{
                "bind": "127.0.0.1:9999",
                "peer_id": "cfg-node",
                "state_file": "state.json",
                "control_token": "control",
                "control_token_file": "control.secret",
                "cors_allow_origins": ["https://allowed.example"],
                "sync_interval_seconds": 5,
                "sync_max_backoff_seconds": 60,
                "rate_limit_window_seconds": 30,
                "rate_limit_max_requests": 120,
                "sync_peers": [
                    { "url": "http://127.0.0.1:8787", "token": "peer-token", "token_file": "peer.secret" }
                ]
            }"#,
        )
        .unwrap();
        assert_eq!(config.bind.as_deref(), Some("127.0.0.1:9999"));
        assert_eq!(config.peer_id.as_deref(), Some("cfg-node"));
        assert_eq!(config.control_token.as_deref(), Some("control"));
        assert_eq!(config.control_token_file.as_deref(), Some("control.secret"));
        assert_eq!(
            config.cors_allow_origins.unwrap(),
            vec!["https://allowed.example"]
        );
        assert_eq!(config.sync_interval_seconds, Some(5));
        assert_eq!(config.sync_max_backoff_seconds, Some(60));
        assert_eq!(config.rate_limit_window_seconds, Some(30));
        assert_eq!(config.rate_limit_max_requests, Some(120));
        let peer = &config.sync_peers.unwrap()[0];
        assert_eq!(peer.url, "http://127.0.0.1:8787");
        assert_eq!(peer.token.as_deref(), Some("peer-token"));
        assert_eq!(peer.token_file.as_deref(), Some("peer.secret"));
    }
}
