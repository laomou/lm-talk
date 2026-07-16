use futures::StreamExt;
use libp2p::{
    Multiaddr, PeerId, StreamProtocol, connection_limits, noise, request_response,
    swarm::NetworkBehaviour, tcp, yamux,
};
use lm_core::{PublicPeerAnnounce, UserId};
use lm_node::{
    ConsumedOneTimePreKey, ControlRequest, DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES,
    DhtRecord, DhtRecordReplicationPlan, DhtRpcRequest, DhtRpcResponse, MailboxDelivery,
    NativeNode, NodeConfig, NodeMaintenanceStats, NodeMergeStats, NodeStateSnapshot,
    NodeSyncPeerStatus, NodeSyncStatus, RoutingPeer, decode_identity_public_key_base64,
    parse_capabilities_csv, restore_identity_from_backup_text,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    env, fs,
    fs::{File, OpenOptions},
    io::{Read, Write},
    net::{IpAddr, TcpListener, TcpStream, ToSocketAddrs},
    path::{Path, PathBuf},
    process,
    time::{Duration, Instant},
};

#[allow(dead_code)]
const LIBP2P_DHT_RPC_PROTOCOL: &str = "/lm-talk/dht-rpc/1";
const MAX_CONTROL_PEER_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
const CONTROL_PEER_HTTP_TIMEOUT: Duration = Duration::from_secs(10);
const CONTROL_PEER_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const CONTROL_CLIENT_HTTP_TIMEOUT: Duration = Duration::from_secs(10);
const DHT_PEER_FAILURE_BACKOFF_BASE_SECONDS: u64 = 30;
const DHT_PEER_FAILURE_BACKOFF_MAX_SECONDS: u64 = 10 * 60;
#[allow(dead_code)]
const MAX_DHT_RESPONSE_RECORDS: usize = 64;
const MAX_DHT_RESPONSE_NODES: usize = 64;
const MAX_LIBP2P_DHT_RPC_REQUEST_BYTES: u64 = 1024 * 1024;
const MAX_LIBP2P_DHT_RPC_RESPONSE_BYTES: u64 = MAX_CONTROL_PEER_RESPONSE_BYTES as u64;
const MAX_LIBP2P_DHT_RPC_CONCURRENT_STREAMS: usize = 32;
const MAX_LIBP2P_DHT_PENDING_INCOMING: u32 = 64;
const MAX_LIBP2P_DHT_PENDING_OUTGOING: u32 = 64;
const MAX_LIBP2P_DHT_ESTABLISHED_INCOMING: u32 = 128;
const MAX_LIBP2P_DHT_ESTABLISHED_OUTGOING: u32 = 128;
const MAX_LIBP2P_DHT_ESTABLISHED_TOTAL: u32 = 256;
const MAX_LIBP2P_DHT_ESTABLISHED_PER_PEER: u32 = 4;
const MAX_CONTROL_REQUEST_HEADER_BYTES: usize = 1024 * 1024;
const MAX_CONTROL_REQUEST_BODY_BYTES: usize = 4 * 1024 * 1024;
const MAX_CONTROL_REQUEST_METHOD_BYTES: usize = 16;
const MAX_CONTROL_REQUEST_PATH_BYTES: usize = 4096;
const MAX_CONTROL_REQUEST_HEADER_LINE_BYTES: usize = 8192;

#[allow(dead_code)]
type Libp2pDhtRpcBehaviour = request_response::json::Behaviour<DhtRpcRequest, DhtRpcResponse>;

#[allow(dead_code)]
#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "Libp2pDhtEvent")]
struct Libp2pDhtBehaviour {
    dht_rpc: Libp2pDhtRpcBehaviour,
    limits: connection_limits::Behaviour,
}

#[allow(dead_code)]
#[derive(Debug)]
enum Libp2pDhtEvent {
    DhtRpc(request_response::Event<DhtRpcRequest, DhtRpcResponse>),
}

impl From<request_response::Event<DhtRpcRequest, DhtRpcResponse>> for Libp2pDhtEvent {
    fn from(event: request_response::Event<DhtRpcRequest, DhtRpcResponse>) -> Self {
        Self::DhtRpc(event)
    }
}

impl From<std::convert::Infallible> for Libp2pDhtEvent {
    fn from(value: std::convert::Infallible) -> Self {
        match value {}
    }
}

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
        "serve-dht-libp2p" => {
            let listen = optional_arg(&args, "--listen")?
                .or_else(|| env::var("LM_NODE_LIBP2P_LISTEN").ok())
                .unwrap_or("/ip4/0.0.0.0/tcp/4001".into());
            let bootstrap_peers = optional_arg(&args, "--bootstrap-peer")?
                .or_else(|| env::var("LM_NODE_LIBP2P_BOOTSTRAP_PEERS").ok())
                .map(|value| parse_libp2p_bootstrap_peers(&value))
                .transpose()?
                .unwrap_or_default();
            let peer_id = optional_arg(&args, "--peer-id")?.unwrap_or("lm-node-dev".into());
            let state_file = optional_arg(&args, "--state-file")?;
            let state_db = optional_arg(&args, "--state-db")?;
            let state_db_require_encryption = optional_arg(&args, "--state-db-require-encryption")?
                .or_else(|| env::var("LM_NODE_STATE_DB_REQUIRE_ENCRYPTION").ok())
                .map(|value| value.parse::<bool>())
                .transpose()?
                .unwrap_or(false);
            validate_state_db_encryption_requirement(state_db_require_encryption)?;
            let mut node = if let Some(path) = &state_db {
                load_node_state_db(path).unwrap_or_else(|_| {
                    NativeNode::new(NodeConfig {
                        peer_id: peer_id.clone(),
                        ..Default::default()
                    })
                })
            } else if let Some(path) = &state_file {
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
            serve_libp2p_dht(
                &listen,
                &bootstrap_peers,
                &mut node,
                state_file.as_deref(),
                state_db.as_deref(),
            )?;
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
            let state_db = optional_arg(&args, "--state-db")?.or(file_config.state_db);
            let state_db_require_encryption = optional_arg(&args, "--state-db-require-encryption")?
                .or_else(|| env::var("LM_NODE_STATE_DB_REQUIRE_ENCRYPTION").ok())
                .map(|value| value.parse::<bool>())
                .transpose()?
                .or(file_config.state_db_require_encryption)
                .unwrap_or(false);
            validate_state_db_encryption_requirement(state_db_require_encryption)?;
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
                    peer_id: peer.peer_id,
                });
            }
            if let Some(sync_peer_urls) = optional_arg(&args, "--sync-peer")? {
                sync_peers = parse_csv(&sync_peer_urls)
                    .into_iter()
                    .map(|url| SyncPeerConfig {
                        url,
                        token: sync_peer_token.clone(),
                        peer_id: None,
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
            let default_dht = DhtRunnerConfig::default();
            let dht_replication_factor = optional_arg(&args, "--dht-replication-factor")?
                .or_else(|| env::var("LM_NODE_DHT_REPLICATION_FACTOR").ok())
                .map(|value| value.parse::<usize>())
                .transpose()?
                .or(file_config.dht_replication_factor)
                .unwrap_or(default_dht.replication_factor);
            let dht_routing_refresh_limit = optional_arg(&args, "--dht-routing-refresh-limit")?
                .or_else(|| env::var("LM_NODE_DHT_ROUTING_REFRESH_LIMIT").ok())
                .map(|value| value.parse::<usize>())
                .transpose()?
                .or(file_config.dht_routing_refresh_limit)
                .unwrap_or(default_dht.routing_refresh_limit);
            let dht_routing_refresh_max_targets =
                optional_arg(&args, "--dht-routing-refresh-max-targets")?
                    .or_else(|| env::var("LM_NODE_DHT_ROUTING_REFRESH_MAX_TARGETS").ok())
                    .map(|value| value.parse::<usize>())
                    .transpose()?
                    .or(file_config.dht_routing_refresh_max_targets)
                    .unwrap_or(default_dht.routing_refresh_max_targets);
            let dht_transport = optional_arg(&args, "--dht-transport")?
                .or_else(|| env::var("LM_NODE_DHT_TRANSPORT").ok())
                .or(file_config.dht_transport)
                .map(|value| parse_dht_transport_kind(&value))
                .transpose()?
                .unwrap_or(default_dht.transport);
            let dht_peer_quarantine_consecutive_failures =
                optional_arg(&args, "--dht-peer-quarantine-consecutive-failures")?
                    .or_else(|| env::var("LM_NODE_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES").ok())
                    .map(|value| value.parse::<u32>())
                    .transpose()?
                    .or(file_config.dht_peer_quarantine_consecutive_failures)
                    .unwrap_or(default_dht.peer_quarantine_consecutive_failures);
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
            let log_format = optional_arg(&args, "--log-format")?
                .or_else(|| env::var("LM_NODE_LOG_FORMAT").ok())
                .or(file_config.log_format)
                .map(|value| parse_log_format(&value))
                .transpose()?
                .unwrap_or_default();
            let mailbox_sender_rate_limit_window_seconds =
                optional_arg(&args, "--mailbox-sender-rate-limit-window-seconds")?
                    .or_else(|| env::var("LM_NODE_MAILBOX_SENDER_RATE_LIMIT_WINDOW_SECONDS").ok())
                    .map(|value| value.parse::<u64>())
                    .transpose()?
                    .or(file_config.mailbox_sender_rate_limit_window_seconds);
            let mailbox_sender_rate_limit_max_messages =
                optional_arg(&args, "--mailbox-sender-rate-limit-max-messages")?
                    .or_else(|| env::var("LM_NODE_MAILBOX_SENDER_RATE_LIMIT_MAX_MESSAGES").ok())
                    .map(|value| value.parse::<u32>())
                    .transpose()?
                    .or(file_config.mailbox_sender_rate_limit_max_messages);
            let mailbox_global_rate_limit_window_seconds =
                optional_arg(&args, "--mailbox-global-rate-limit-window-seconds")?
                    .or_else(|| env::var("LM_NODE_MAILBOX_GLOBAL_RATE_LIMIT_WINDOW_SECONDS").ok())
                    .map(|value| value.parse::<u64>())
                    .transpose()?
                    .or(file_config.mailbox_global_rate_limit_window_seconds);
            let mailbox_global_rate_limit_max_messages =
                optional_arg(&args, "--mailbox-global-rate-limit-max-messages")?
                    .or_else(|| env::var("LM_NODE_MAILBOX_GLOBAL_RATE_LIMIT_MAX_MESSAGES").ok())
                    .map(|value| value.parse::<u32>())
                    .transpose()?
                    .or(file_config.mailbox_global_rate_limit_max_messages);
            let default_node_config = lm_node::NodeConfig::default();
            let max_mailbox_bytes = optional_arg(&args, "--max-mailbox-bytes")?
                .or_else(|| env::var("LM_NODE_MAX_MAILBOX_BYTES").ok())
                .map(|value| value.parse::<u64>())
                .transpose()?
                .or(file_config.max_mailbox_bytes)
                .or(default_node_config.max_mailbox_bytes);
            let max_mailbox_bytes_per_user = optional_arg(&args, "--max-mailbox-bytes-per-user")?
                .or_else(|| env::var("LM_NODE_MAX_MAILBOX_BYTES_PER_USER").ok())
                .map(|value| value.parse::<u64>())
                .transpose()?
                .or(file_config.max_mailbox_bytes_per_user)
                .or(default_node_config.max_mailbox_bytes_per_user);
            let max_mailbox_messages_per_user =
                optional_arg(&args, "--max-mailbox-messages-per-user")?
                    .or_else(|| env::var("LM_NODE_MAX_MAILBOX_MESSAGES_PER_USER").ok())
                    .map(|value| value.parse::<usize>())
                    .transpose()?
                    .or(file_config.max_mailbox_messages_per_user)
                    .or(default_node_config.max_mailbox_messages_per_user);
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
            let control_previous_tokens = optional_arg(&args, "--control-previous-token")?
                .or_else(|| env::var("LM_NODE_CONTROL_PREVIOUS_TOKENS").ok())
                .map(|value| parse_csv(&value))
                .or(file_config.control_previous_tokens)
                .unwrap_or_default();
            let cors_allow_origins = optional_arg(&args, "--cors-allow-origin")?
                .or_else(|| env::var("LM_NODE_CORS_ALLOW_ORIGIN").ok())
                .map(|value| parse_csv(&value))
                .or(file_config.cors_allow_origins)
                .unwrap_or_default();
            let security = ControlSecurityConfig {
                token,
                previous_tokens: control_previous_tokens,
                cors_allow_origins,
            };
            let mut node = if let Some(path) = &state_db {
                load_node_state_db(path).unwrap_or_else(|_| {
                    NativeNode::new(NodeConfig {
                        peer_id: peer_id.clone(),
                        mailbox_sender_rate_limit_window_seconds,
                        mailbox_sender_rate_limit_max_messages,
                        mailbox_global_rate_limit_window_seconds,
                        mailbox_global_rate_limit_max_messages,
                        max_mailbox_bytes,
                        max_mailbox_bytes_per_user,
                        max_mailbox_messages_per_user,
                        dht_peer_quarantine_consecutive_failures,
                        ..Default::default()
                    })
                })
            } else if let Some(path) = &state_file {
                load_node_state(path).unwrap_or_else(|_| {
                    NativeNode::new(NodeConfig {
                        peer_id: peer_id.clone(),
                        mailbox_sender_rate_limit_window_seconds,
                        mailbox_sender_rate_limit_max_messages,
                        mailbox_global_rate_limit_window_seconds,
                        mailbox_global_rate_limit_max_messages,
                        max_mailbox_bytes,
                        max_mailbox_bytes_per_user,
                        max_mailbox_messages_per_user,
                        dht_peer_quarantine_consecutive_failures,
                        ..Default::default()
                    })
                })
            } else {
                NativeNode::new(NodeConfig {
                    peer_id,
                    mailbox_sender_rate_limit_window_seconds,
                    mailbox_sender_rate_limit_max_messages,
                    mailbox_global_rate_limit_window_seconds,
                    mailbox_global_rate_limit_max_messages,
                    max_mailbox_bytes,
                    max_mailbox_bytes_per_user,
                    max_mailbox_messages_per_user,
                    dht_peer_quarantine_consecutive_failures,
                    ..Default::default()
                })
            };
            node.config.mailbox_sender_rate_limit_window_seconds =
                mailbox_sender_rate_limit_window_seconds;
            node.config.mailbox_sender_rate_limit_max_messages =
                mailbox_sender_rate_limit_max_messages;
            node.config.mailbox_global_rate_limit_window_seconds =
                mailbox_global_rate_limit_window_seconds;
            node.config.mailbox_global_rate_limit_max_messages =
                mailbox_global_rate_limit_max_messages;
            node.config.max_mailbox_bytes = max_mailbox_bytes;
            node.config.max_mailbox_bytes_per_user = max_mailbox_bytes_per_user;
            node.config.max_mailbox_messages_per_user = max_mailbox_messages_per_user;
            node.config.dht_peer_quarantine_consecutive_failures =
                dht_peer_quarantine_consecutive_failures;
            serve_control(
                &bind,
                &mut node,
                state_file.as_deref(),
                state_db.as_deref(),
                sync_peers,
                sync_interval_seconds,
                sync_max_backoff_seconds,
                DhtRunnerConfig {
                    replication_factor: dht_replication_factor,
                    routing_refresh_limit: dht_routing_refresh_limit,
                    routing_refresh_max_targets: dht_routing_refresh_max_targets,
                    transport: dht_transport,
                    peer_quarantine_consecutive_failures: dht_peer_quarantine_consecutive_failures,
                },
                security,
                RateLimitConfig {
                    window_seconds: rate_limit_window_seconds,
                    max_requests: rate_limit_max_requests,
                },
                ControlLogger::new(log_format),
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
    validate_secret_file_permissions(path)?;
    let value = fs::read_to_string(path)?.trim().to_string();
    if value.is_empty() {
        Err(format!("secret file is empty: {path}").into())
    } else {
        Ok(value)
    }
}

#[cfg(unix)]
fn validate_secret_file_permissions(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(format!("secret file must not be a symlink: {path}").into());
    }
    if !metadata.file_type().is_file() {
        return Err(format!("secret path is not a regular file: {path}").into());
    }
    let mode = metadata.permissions().mode();
    if mode & 0o077 != 0 {
        return Err(format!("secret file permissions too broad: {path}; use chmod 600").into());
    }
    Ok(())
}

#[cfg(not(unix))]
fn validate_secret_file_permissions(_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
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

fn validate_state_db_encryption_requirement(
    required: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if required {
        Err("state_db encryption is required but this build uses plain SQLite; enable SQLCipher/equivalent before production".into())
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
struct ControlSecurityConfig {
    token: Option<String>,
    previous_tokens: Vec<String>,
    cors_allow_origins: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SyncPeerConfig {
    url: String,
    token: Option<String>,
    peer_id: Option<String>,
}

trait DhtTransport: Sync {
    fn send_dht_rpc(
        &self,
        peer: &SyncPeerConfig,
        request: &DhtRpcRequest,
    ) -> Result<DhtRpcResponse, Box<dyn std::error::Error>>;
}

#[derive(Debug, Clone, Copy)]
struct HttpControlDhtTransport;

impl DhtTransport for HttpControlDhtTransport {
    fn send_dht_rpc(
        &self,
        peer: &SyncPeerConfig,
        request: &DhtRpcRequest,
    ) -> Result<DhtRpcResponse, Box<dyn std::error::Error>> {
        let body = serde_json::json!({ "request": request }).to_string();
        let response = http_control_request(peer, "POST", "/dht/rpc", &body)?;
        let response = serde_json::from_str(&response)?;
        validate_dht_rpc_response(request, response)
    }
}

fn dht_rpc_request_id(request: &DhtRpcRequest) -> &str {
    match request {
        DhtRpcRequest::FindNode { request_id, .. }
        | DhtRpcRequest::FindValue { request_id, .. }
        | DhtRpcRequest::StoreRecord { request_id, .. } => request_id,
    }
}

fn dht_rpc_response_id(response: &DhtRpcResponse) -> &str {
    match response {
        DhtRpcResponse::Nodes { request_id, .. }
        | DhtRpcResponse::Value { request_id, .. }
        | DhtRpcResponse::StoreResult { request_id, .. }
        | DhtRpcResponse::Error { request_id, .. } => request_id,
    }
}

fn validate_dht_rpc_response(
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

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct Libp2pDhtTransport {
    timeout: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
struct Libp2pBootstrapPeer {
    peer_id: PeerId,
    address: Multiaddr,
}

#[allow(dead_code)]
impl Default for Libp2pDhtTransport {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(10),
        }
    }
}

#[allow(dead_code)]
impl DhtTransport for Libp2pDhtTransport {
    fn send_dht_rpc(
        &self,
        peer: &SyncPeerConfig,
        request: &DhtRpcRequest,
    ) -> Result<DhtRpcResponse, Box<dyn std::error::Error>> {
        let (peer_id, address) = parse_libp2p_dht_peer(peer)?;
        let response = send_libp2p_dht_rpc(peer_id, address, request.clone(), self.timeout)?;
        validate_dht_rpc_response(request, response)
    }
}

#[allow(dead_code)]
fn libp2p_dht_rpc_behaviour() -> Libp2pDhtRpcBehaviour {
    request_response::json::Behaviour::with_codec(
        request_response::json::codec::Codec::default()
            .set_request_size_maximum(MAX_LIBP2P_DHT_RPC_REQUEST_BYTES)
            .set_response_size_maximum(MAX_LIBP2P_DHT_RPC_RESPONSE_BYTES),
        [(
            StreamProtocol::new(LIBP2P_DHT_RPC_PROTOCOL),
            request_response::ProtocolSupport::Full,
        )],
        libp2p_dht_rpc_config(),
    )
}

fn libp2p_dht_rpc_config() -> request_response::Config {
    request_response::Config::default()
        .with_request_timeout(Duration::from_secs(10))
        .with_max_concurrent_streams(MAX_LIBP2P_DHT_RPC_CONCURRENT_STREAMS)
}

#[allow(dead_code)]

fn libp2p_dht_connection_limits() -> connection_limits::ConnectionLimits {
    connection_limits::ConnectionLimits::default()
        .with_max_pending_incoming(Some(MAX_LIBP2P_DHT_PENDING_INCOMING))
        .with_max_pending_outgoing(Some(MAX_LIBP2P_DHT_PENDING_OUTGOING))
        .with_max_established_incoming(Some(MAX_LIBP2P_DHT_ESTABLISHED_INCOMING))
        .with_max_established_outgoing(Some(MAX_LIBP2P_DHT_ESTABLISHED_OUTGOING))
        .with_max_established(Some(MAX_LIBP2P_DHT_ESTABLISHED_TOTAL))
        .with_max_established_per_peer(Some(MAX_LIBP2P_DHT_ESTABLISHED_PER_PEER))
}

fn libp2p_dht_swarm() -> Result<libp2p::Swarm<Libp2pDhtBehaviour>, Box<dyn std::error::Error>> {
    Ok(libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_behaviour(|_| Libp2pDhtBehaviour {
            dht_rpc: libp2p_dht_rpc_behaviour(),
            limits: connection_limits::Behaviour::new(libp2p_dht_connection_limits()),
        })?
        .build())
}

#[allow(dead_code)]
fn handle_libp2p_dht_rpc_request(node: &mut NativeNode, request: DhtRpcRequest) -> DhtRpcResponse {
    node.handle_dht_rpc(request)
}

#[allow(dead_code)]
fn handle_libp2p_dht_rpc_event(
    node: &mut NativeNode,
    behaviour: &mut Libp2pDhtRpcBehaviour,
    event: request_response::Event<DhtRpcRequest, DhtRpcResponse>,
) -> Option<DhtRpcResponse> {
    if let request_response::Event::Message {
        message: request_response::Message::Request {
            request, channel, ..
        },
        ..
    } = event
    {
        let response = handle_libp2p_dht_rpc_request(node, request);
        let _ = behaviour.send_response(channel, response.clone());
        return Some(response);
    }
    None
}

#[allow(dead_code)]
fn handle_libp2p_dht_server_event(
    node: &mut NativeNode,
    swarm: &mut libp2p::Swarm<Libp2pDhtBehaviour>,
    pending_discovery: &mut HashSet<request_response::OutboundRequestId>,
    event: libp2p::swarm::SwarmEvent<Libp2pDhtEvent>,
) -> Option<DhtRpcResponse> {
    match event {
        libp2p::swarm::SwarmEvent::NewListenAddr { address, .. } => {
            println!("libp2p_dht_listen={address}");
            None
        }
        libp2p::swarm::SwarmEvent::ConnectionEstablished { peer_id, .. } => {
            let request_id = swarm.behaviour_mut().dht_rpc.send_request(
                &peer_id,
                DhtRpcRequest::FindNode {
                    request_id: format!("bootstrap-discovery-{}", current_unix_timestamp()),
                    target: node.kademlia.local_id(),
                    limit: 8,
                },
            );
            pending_discovery.insert(request_id);
            println!("libp2p_dht_connected={peer_id}");
            None
        }
        libp2p::swarm::SwarmEvent::Behaviour(Libp2pDhtEvent::DhtRpc(event)) => {
            if let request_response::Event::Message {
                message:
                    request_response::Message::Response {
                        request_id,
                        response: DhtRpcResponse::Nodes { nodes, .. },
                    },
                ..
            } = &event
            {
                if pending_discovery.remove(request_id) {
                    let returned = nodes.len();
                    let merged = node.merge_verified_routing_peers(nodes.clone());
                    println!("libp2p_dht_discovery_nodes={returned} merged={merged}");
                    return None;
                }
            }
            handle_libp2p_dht_rpc_event(node, &mut swarm.behaviour_mut().dht_rpc, event)
        }
        _ => None,
    }
}

#[allow(dead_code)]
fn persist_libp2p_dht_state(
    node: &NativeNode,
    state_file: Option<&str>,
    state_db: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(path) = state_db {
        save_node_state_db(path, node)?;
    }
    if let Some(path) = state_file {
        save_node_state(path, node)?;
    }
    Ok(())
}

#[allow(dead_code)]
fn serve_libp2p_dht(
    listen: &str,
    bootstrap_peers: &[Libp2pBootstrapPeer],
    node: &mut NativeNode,
    state_file: Option<&str>,
    state_db: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()?;
    runtime.block_on(async {
        let mut swarm = libp2p_dht_swarm()?;
        let local_peer_id = *swarm.local_peer_id();
        swarm.listen_on(listen.parse::<Multiaddr>()?)?;
        dial_libp2p_bootstrap_peers(&mut swarm, bootstrap_peers)?;
        let mut pending_discovery = HashSet::new();
        println!("libp2p_dht_peer_id={local_peer_id}");
        loop {
            let event = swarm.select_next_some().await;
            if handle_libp2p_dht_server_event(node, &mut swarm, &mut pending_discovery, event)
                .is_some()
            {
                persist_libp2p_dht_state(node, state_file, state_db)?;
            }
        }
        #[allow(unreachable_code)]
        Ok::<(), Box<dyn std::error::Error>>(())
    })
}

#[allow(dead_code)]
fn parse_libp2p_bootstrap_peers(
    value: &str,
) -> Result<Vec<Libp2pBootstrapPeer>, Box<dyn std::error::Error>> {
    parse_csv(value)
        .into_iter()
        .map(|part| {
            let (address, peer_id) = part
                .rsplit_once('|')
                .ok_or("libp2p bootstrap peers must use libp2p://<multiaddr>|<peer_id>")?;
            Ok(Libp2pBootstrapPeer {
                address: address
                    .strip_prefix("libp2p://")
                    .unwrap_or(address)
                    .parse::<Multiaddr>()?,
                peer_id: peer_id.parse::<PeerId>()?,
            })
        })
        .collect()
}

#[allow(dead_code)]
fn dial_libp2p_bootstrap_peers(
    swarm: &mut libp2p::Swarm<Libp2pDhtBehaviour>,
    peers: &[Libp2pBootstrapPeer],
) -> Result<(), Box<dyn std::error::Error>> {
    for peer in peers {
        swarm.add_peer_address(peer.peer_id, peer.address.clone());
        swarm.dial(peer.address.clone())?;
        println!(
            "libp2p_dht_bootstrap_peer={} {}",
            peer.peer_id, peer.address
        );
    }
    Ok(())
}

#[allow(dead_code)]
fn parse_libp2p_dht_peer(
    peer: &SyncPeerConfig,
) -> Result<(PeerId, Multiaddr), Box<dyn std::error::Error>> {
    let peer_id = peer
        .peer_id
        .as_deref()
        .ok_or("libp2p DHT peers require peer_id")?
        .parse::<PeerId>()?;
    let address_text = peer
        .url
        .strip_prefix("libp2p://")
        .unwrap_or(peer.url.as_str());
    let address = address_text.parse::<Multiaddr>()?;
    Ok((peer_id, address))
}

#[allow(dead_code)]
fn send_libp2p_dht_rpc(
    peer_id: PeerId,
    address: Multiaddr,
    request: DhtRpcRequest,
    timeout: Duration,
) -> Result<DhtRpcResponse, Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()?;
    runtime.block_on(send_libp2p_dht_rpc_async(
        peer_id, address, request, timeout,
    ))
}

#[allow(dead_code)]
async fn send_libp2p_dht_rpc_async(
    peer_id: PeerId,
    address: Multiaddr,
    request: DhtRpcRequest,
    timeout: Duration,
) -> Result<DhtRpcResponse, Box<dyn std::error::Error>> {
    let mut swarm = libp2p_dht_swarm()?;
    let request_id =
        swarm
            .behaviour_mut()
            .dht_rpc
            .send_request_with_addresses(&peer_id, request, vec![address]);
    let response = async {
        loop {
            if let libp2p::swarm::SwarmEvent::Behaviour(Libp2pDhtEvent::DhtRpc(
                request_response::Event::Message { message, .. },
            )) = swarm.select_next_some().await
            {
                match message {
                    request_response::Message::Response {
                        request_id: received_id,
                        response,
                    } if received_id == request_id => break Ok(response),
                    request_response::Message::Request { .. } => {}
                    request_response::Message::Response { .. } => {}
                }
            }
        }
    };
    tokio::time::timeout(timeout, response)
        .await
        .map_err(|_| "libp2p DHT RPC timed out")?
}

#[derive(Debug, Clone)]
struct SyncPeerRuntime {
    config: SyncPeerConfig,
    next_attempt_at: Instant,
    consecutive_failures: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DhtRunnerConfig {
    replication_factor: usize,
    routing_refresh_limit: usize,
    routing_refresh_max_targets: usize,
    transport: DhtTransportKind,
    peer_quarantine_consecutive_failures: u32,
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
enum DhtTransportKind {
    HttpControl,
    Libp2p,
}

impl DhtTransportKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::HttpControl => "http-control",
            Self::Libp2p => "libp2p",
        }
    }
}

fn parse_dht_transport_kind(value: &str) -> Result<DhtTransportKind, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "http" | "http-control" | "control" => Ok(DhtTransportKind::HttpControl),
        "libp2p" => Ok(DhtTransportKind::Libp2p),
        other => Err(format!(
            "unsupported dht transport {other:?}; expected http-control or libp2p"
        )),
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
struct DhtReplicationRunStats {
    records: usize,
    attempts: usize,
    successes: usize,
    failures: usize,
    peers_quarantined: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
struct DhtRoutingRefreshRunStats {
    targets: usize,
    attempts: usize,
    successes: usize,
    failures: usize,
    nodes_returned: usize,
    nodes_merged: usize,
    nodes_rejected_non_closer: usize,
    nodes_rejected_duplicate: usize,
    peers_quarantined: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
struct DhtFindValueRunStats {
    attempts: usize,
    successes: usize,
    failures: usize,
    found_records: usize,
    closer_records: usize,
    closer_nodes_returned: usize,
    closer_nodes_merged: usize,
    closer_nodes_rejected_non_closer: usize,
    closer_nodes_rejected_duplicate: usize,
    peers_quarantined: usize,
    quarantine_threshold: u32,
    query_rounds: usize,
    alpha: usize,
    exhausted: bool,
}

#[derive(Debug, Serialize)]
struct DhtFindValueRunResponse {
    key: String,
    found: bool,
    records: usize,
    stats: DhtFindValueRunStats,
}

#[derive(Debug, Serialize)]
struct DhtReplicationRunResponse {
    peers: usize,
    records: usize,
    stats: DhtReplicationRunStats,
}

#[derive(Debug, Serialize)]
struct DhtMaintenanceRunResponse {
    peers: usize,
    records: usize,
    routing_peers: usize,
    replication: DhtReplicationRunStats,
    routing_refresh: DhtRoutingRefreshRunStats,
}

#[derive(Debug, Serialize)]
struct DhtRoutingRefreshRunResponse {
    peers: usize,
    routing_peers: usize,
    stats: DhtRoutingRefreshRunStats,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum LogFormat {
    #[default]
    Text,
    Json,
}

fn parse_log_format(value: &str) -> Result<LogFormat, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "" | "text" | "plain" => Ok(LogFormat::Text),
        "json" | "structured" => Ok(LogFormat::Json),
        other => Err(format!(
            "invalid log format: {other}; expected text or json"
        )),
    }
}

#[derive(Debug, Clone, Copy)]
struct ControlLogger {
    format: LogFormat,
}

impl ControlLogger {
    fn new(format: LogFormat) -> Self {
        Self { format }
    }

    fn info(&self, event: &str, message: impl Into<String>, fields: serde_json::Value) {
        self.log("info", event, message.into(), fields);
    }

    fn warn(&self, event: &str, message: impl Into<String>, fields: serde_json::Value) {
        self.log("warn", event, message.into(), fields);
    }

    fn error(&self, event: &str, message: impl Into<String>, fields: serde_json::Value) {
        self.log("error", event, message.into(), fields);
    }

    fn log(&self, level: &str, event: &str, message: String, fields: serde_json::Value) {
        println!("{}", self.render_line(level, event, message, fields));
    }

    fn render_line(
        &self,
        level: &str,
        event: &str,
        message: String,
        fields: serde_json::Value,
    ) -> String {
        match self.format {
            LogFormat::Text => {
                if fields.is_null() {
                    message
                } else {
                    format!("{message} {}", compact_json(&fields))
                }
            }
            LogFormat::Json => {
                let value = serde_json::json!({
                    "ts": current_unix_timestamp(),
                    "level": level,
                    "event": event,
                    "message": message,
                    "fields": fields,
                });
                compact_json(&value)
            }
        }
    }
}

fn compact_json(value: &serde_json::Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string())
}

#[derive(Debug, Serialize)]
struct ControlStatsResponse<'a> {
    #[serde(flatten)]
    runtime: &'a ControlRuntimeStats,
    maintenance: NodeMaintenanceStats,
    state_db: Option<StateDbStats>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct StateDbStats {
    page_count: u64,
    page_size_bytes: u64,
    freelist_count: u64,
    file_bytes: u64,
    encrypted: bool,
    permissions_hardened: bool,
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
    dht_replication_runs: u64,
    dht_replication_records: u64,
    dht_replication_attempts: u64,
    dht_replication_successes: u64,
    dht_replication_failures: u64,
    dht_replication_peers_quarantined: u64,
    last_dht_replication_at: Option<u64>,
    dht_routing_refresh_runs: u64,
    dht_routing_refresh_targets: u64,
    dht_routing_refresh_attempts: u64,
    dht_routing_refresh_successes: u64,
    dht_routing_refresh_failures: u64,
    dht_routing_refresh_nodes_returned: u64,
    dht_routing_refresh_nodes_merged: u64,
    dht_routing_refresh_nodes_rejected_non_closer: u64,
    dht_routing_refresh_nodes_rejected_duplicate: u64,
    dht_routing_refresh_peers_quarantined: u64,
    last_dht_routing_refresh_at: Option<u64>,
    dht_find_value_runs: u64,
    dht_find_value_attempts: u64,
    dht_find_value_successes: u64,
    dht_find_value_failures: u64,
    dht_find_value_found_records: u64,
    dht_find_value_closer_records: u64,
    dht_find_value_closer_nodes_returned: u64,
    dht_find_value_closer_nodes_merged: u64,
    dht_find_value_closer_nodes_rejected_non_closer: u64,
    dht_find_value_closer_nodes_rejected_duplicate: u64,
    dht_find_value_peers_quarantined: u64,
    dht_find_value_query_rounds: u64,
    dht_find_value_alpha_max: usize,
    dht_find_value_exhausted: u64,
    last_dht_find_value_at: Option<u64>,
    sync_schedule_delay_micros_total: u128,
    sync_schedule_delay_micros_max: u128,
    last_sync_schedule_delay_micros: Option<u128>,
    dht_replication_schedule_delay_micros_total: u128,
    dht_replication_schedule_delay_micros_max: u128,
    last_dht_replication_schedule_delay_micros: Option<u128>,
    dht_routing_refresh_schedule_delay_micros_total: u128,
    dht_routing_refresh_schedule_delay_micros_max: u128,
    last_dht_routing_refresh_schedule_delay_micros: Option<u128>,
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
            dht_replication_runs: 0,
            dht_replication_records: 0,
            dht_replication_attempts: 0,
            dht_replication_successes: 0,
            dht_replication_failures: 0,
            dht_replication_peers_quarantined: 0,
            last_dht_replication_at: None,
            dht_routing_refresh_runs: 0,
            dht_routing_refresh_targets: 0,
            dht_routing_refresh_attempts: 0,
            dht_routing_refresh_successes: 0,
            dht_routing_refresh_failures: 0,
            dht_routing_refresh_nodes_returned: 0,
            dht_routing_refresh_nodes_merged: 0,
            dht_routing_refresh_nodes_rejected_non_closer: 0,
            dht_routing_refresh_nodes_rejected_duplicate: 0,
            dht_routing_refresh_peers_quarantined: 0,
            last_dht_routing_refresh_at: None,
            dht_find_value_runs: 0,
            dht_find_value_attempts: 0,
            dht_find_value_successes: 0,
            dht_find_value_failures: 0,
            dht_find_value_found_records: 0,
            dht_find_value_closer_records: 0,
            dht_find_value_closer_nodes_returned: 0,
            dht_find_value_closer_nodes_merged: 0,
            dht_find_value_closer_nodes_rejected_non_closer: 0,
            dht_find_value_closer_nodes_rejected_duplicate: 0,
            dht_find_value_peers_quarantined: 0,
            dht_find_value_query_rounds: 0,
            dht_find_value_alpha_max: 0,
            dht_find_value_exhausted: 0,
            last_dht_find_value_at: None,
            sync_schedule_delay_micros_total: 0,
            sync_schedule_delay_micros_max: 0,
            last_sync_schedule_delay_micros: None,
            dht_replication_schedule_delay_micros_total: 0,
            dht_replication_schedule_delay_micros_max: 0,
            last_dht_replication_schedule_delay_micros: None,
            dht_routing_refresh_schedule_delay_micros_total: 0,
            dht_routing_refresh_schedule_delay_micros_max: 0,
            last_dht_routing_refresh_schedule_delay_micros: None,
            endpoints: HashMap::new(),
        }
    }

    fn to_openmetrics(
        &self,
        maintenance: &NodeMaintenanceStats,
        state_db: Option<&StateDbStats>,
        sync_status: Option<&NodeSyncStatus>,
        dht_peer_quarantine_threshold: u32,
    ) -> String {
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
            "lm_node_dht_replication_runs_total",
            "Total DHT control-peer replication runner executions.",
        );
        push_metric_type(&mut out, "lm_node_dht_replication_runs_total", "counter");
        push_metric_value(
            &mut out,
            "lm_node_dht_replication_runs_total",
            self.dht_replication_runs,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_replication_records_total",
            "DHT records included in replication runner plans.",
        );
        push_metric_type(&mut out, "lm_node_dht_replication_records_total", "counter");
        push_metric_value(
            &mut out,
            "lm_node_dht_replication_records_total",
            self.dht_replication_records,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_replication_attempts_total",
            "DHT StoreRecord replication attempts by result.",
        );
        push_metric_type(
            &mut out,
            "lm_node_dht_replication_attempts_total",
            "counter",
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_replication_attempts_total",
            "result",
            "success",
            self.dht_replication_successes,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_replication_attempts_total",
            "result",
            "failure",
            self.dht_replication_failures,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_replication_attempts_total",
            "result",
            "all",
            self.dht_replication_attempts,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_replication_peers_quarantined_total",
            "DHT replication peers skipped by quarantine.",
        );
        push_metric_type(
            &mut out,
            "lm_node_dht_replication_peers_quarantined_total",
            "counter",
        );
        push_metric_value(
            &mut out,
            "lm_node_dht_replication_peers_quarantined_total",
            self.dht_replication_peers_quarantined,
        );
        if let Some(last_dht_replication_at) = self.last_dht_replication_at {
            push_metric_help(
                &mut out,
                "lm_node_dht_replication_last_run_at",
                "Unix timestamp of the last DHT replication runner execution.",
            );
            push_metric_type(&mut out, "lm_node_dht_replication_last_run_at", "gauge");
            push_metric_value(
                &mut out,
                "lm_node_dht_replication_last_run_at",
                last_dht_replication_at,
            );
        }
        push_metric_help(
            &mut out,
            "lm_node_dht_routing_refresh_runs_total",
            "Total DHT routing refresh runner executions.",
        );
        push_metric_type(
            &mut out,
            "lm_node_dht_routing_refresh_runs_total",
            "counter",
        );
        push_metric_value(
            &mut out,
            "lm_node_dht_routing_refresh_runs_total",
            self.dht_routing_refresh_runs,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_routing_refresh_targets_total",
            "DHT routing refresh targets queried by the runner.",
        );
        push_metric_type(
            &mut out,
            "lm_node_dht_routing_refresh_targets_total",
            "counter",
        );
        push_metric_value(
            &mut out,
            "lm_node_dht_routing_refresh_targets_total",
            self.dht_routing_refresh_targets,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_routing_refresh_attempts_total",
            "DHT FindNode routing refresh attempts by result.",
        );
        push_metric_type(
            &mut out,
            "lm_node_dht_routing_refresh_attempts_total",
            "counter",
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_routing_refresh_attempts_total",
            "result",
            "success",
            self.dht_routing_refresh_successes,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_routing_refresh_attempts_total",
            "result",
            "failure",
            self.dht_routing_refresh_failures,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_routing_refresh_attempts_total",
            "result",
            "all",
            self.dht_routing_refresh_attempts,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_routing_refresh_nodes_returned_total",
            "Routing peers returned by DHT FindNode refresh responses.",
        );
        push_metric_type(
            &mut out,
            "lm_node_dht_routing_refresh_nodes_returned_total",
            "counter",
        );
        push_metric_value(
            &mut out,
            "lm_node_dht_routing_refresh_nodes_returned_total",
            self.dht_routing_refresh_nodes_returned,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_routing_refresh_nodes_merged_total",
            "Trusted routing peers merged from DHT FindNode refresh responses.",
        );
        push_metric_type(
            &mut out,
            "lm_node_dht_routing_refresh_nodes_merged_total",
            "counter",
        );
        push_metric_value(
            &mut out,
            "lm_node_dht_routing_refresh_nodes_merged_total",
            self.dht_routing_refresh_nodes_merged,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_routing_refresh_nodes_rejected_total",
            "Routing peers rejected from DHT FindNode refresh responses before merge.",
        );
        push_metric_type(
            &mut out,
            "lm_node_dht_routing_refresh_nodes_rejected_total",
            "counter",
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_routing_refresh_nodes_rejected_total",
            "reason",
            "non_closer",
            self.dht_routing_refresh_nodes_rejected_non_closer,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_routing_refresh_nodes_rejected_total",
            "reason",
            "duplicate",
            self.dht_routing_refresh_nodes_rejected_duplicate,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_routing_refresh_peers_quarantined_total",
            "DHT routing refresh peers skipped by quarantine.",
        );
        push_metric_type(
            &mut out,
            "lm_node_dht_routing_refresh_peers_quarantined_total",
            "counter",
        );
        push_metric_value(
            &mut out,
            "lm_node_dht_routing_refresh_peers_quarantined_total",
            self.dht_routing_refresh_peers_quarantined,
        );
        if let Some(last_dht_routing_refresh_at) = self.last_dht_routing_refresh_at {
            push_metric_help(
                &mut out,
                "lm_node_dht_routing_refresh_last_run_at",
                "Unix timestamp of the last DHT routing refresh runner execution.",
            );
            push_metric_type(&mut out, "lm_node_dht_routing_refresh_last_run_at", "gauge");
            push_metric_value(
                &mut out,
                "lm_node_dht_routing_refresh_last_run_at",
                last_dht_routing_refresh_at,
            );
        }
        push_metric_help(
            &mut out,
            "lm_node_dht_find_value_runs_total",
            "Total manual DHT FindValue runner executions.",
        );
        push_metric_type(&mut out, "lm_node_dht_find_value_runs_total", "counter");
        push_metric_value(
            &mut out,
            "lm_node_dht_find_value_runs_total",
            self.dht_find_value_runs,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_find_value_attempts_total",
            "Manual DHT FindValue attempts by result.",
        );
        push_metric_type(&mut out, "lm_node_dht_find_value_attempts_total", "counter");
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_find_value_attempts_total",
            "result",
            "success",
            self.dht_find_value_successes,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_find_value_attempts_total",
            "result",
            "failure",
            self.dht_find_value_failures,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_find_value_attempts_total",
            "result",
            "all",
            self.dht_find_value_attempts,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_find_value_records_total",
            "Manual DHT FindValue records merged by kind.",
        );
        push_metric_type(&mut out, "lm_node_dht_find_value_records_total", "counter");
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_find_value_records_total",
            "kind",
            "found",
            self.dht_find_value_found_records,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_find_value_records_total",
            "kind",
            "closer",
            self.dht_find_value_closer_records,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_find_value_closer_nodes_total",
            "Manual DHT FindValue closer nodes returned and merged.",
        );
        push_metric_type(
            &mut out,
            "lm_node_dht_find_value_closer_nodes_total",
            "counter",
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_find_value_closer_nodes_total",
            "kind",
            "returned",
            self.dht_find_value_closer_nodes_returned,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_find_value_closer_nodes_total",
            "kind",
            "merged",
            self.dht_find_value_closer_nodes_merged,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_find_value_closer_nodes_total",
            "kind",
            "rejected_non_closer",
            self.dht_find_value_closer_nodes_rejected_non_closer,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_find_value_closer_nodes_total",
            "kind",
            "rejected_duplicate",
            self.dht_find_value_closer_nodes_rejected_duplicate,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_find_value_peers_quarantined_total",
            "Manual DHT FindValue peers skipped by quarantine.",
        );
        push_metric_type(
            &mut out,
            "lm_node_dht_find_value_peers_quarantined_total",
            "counter",
        );
        push_metric_value(
            &mut out,
            "lm_node_dht_find_value_peers_quarantined_total",
            self.dht_find_value_peers_quarantined,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_find_value_query_rounds_total",
            "Total query rounds used by manual DHT FindValue runs.",
        );
        push_metric_type(
            &mut out,
            "lm_node_dht_find_value_query_rounds_total",
            "counter",
        );
        push_metric_value(
            &mut out,
            "lm_node_dht_find_value_query_rounds_total",
            self.dht_find_value_query_rounds,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_find_value_alpha_max",
            "Maximum alpha requested by manual DHT FindValue runs.",
        );
        push_metric_type(&mut out, "lm_node_dht_find_value_alpha_max", "gauge");
        push_metric_value(
            &mut out,
            "lm_node_dht_find_value_alpha_max",
            self.dht_find_value_alpha_max,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_find_value_exhausted_total",
            "Manual DHT FindValue runs that exhausted candidates without finding a record.",
        );
        push_metric_type(
            &mut out,
            "lm_node_dht_find_value_exhausted_total",
            "counter",
        );
        push_metric_value(
            &mut out,
            "lm_node_dht_find_value_exhausted_total",
            self.dht_find_value_exhausted,
        );
        if let Some(last_dht_find_value_at) = self.last_dht_find_value_at {
            push_metric_help(
                &mut out,
                "lm_node_dht_find_value_last_run_at",
                "Unix timestamp of the last manual DHT FindValue execution.",
            );
            push_metric_type(&mut out, "lm_node_dht_find_value_last_run_at", "gauge");
            push_metric_value(
                &mut out,
                "lm_node_dht_find_value_last_run_at",
                last_dht_find_value_at,
            );
        }
        push_metric_help(
            &mut out,
            "lm_node_background_schedule_delay_micros_total",
            "Total scheduler delay in microseconds for background jobs.",
        );
        push_metric_type(
            &mut out,
            "lm_node_background_schedule_delay_micros_total",
            "counter",
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_background_schedule_delay_micros_total",
            "job",
            "snapshot_sync",
            self.sync_schedule_delay_micros_total,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_background_schedule_delay_micros_total",
            "job",
            "dht_replication",
            self.dht_replication_schedule_delay_micros_total,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_background_schedule_delay_micros_total",
            "job",
            "dht_routing_refresh",
            self.dht_routing_refresh_schedule_delay_micros_total,
        );
        push_metric_help(
            &mut out,
            "lm_node_background_schedule_delay_micros_max",
            "Maximum scheduler delay in microseconds for background jobs.",
        );
        push_metric_type(
            &mut out,
            "lm_node_background_schedule_delay_micros_max",
            "gauge",
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_background_schedule_delay_micros_max",
            "job",
            "snapshot_sync",
            self.sync_schedule_delay_micros_max,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_background_schedule_delay_micros_max",
            "job",
            "dht_replication",
            self.dht_replication_schedule_delay_micros_max,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_background_schedule_delay_micros_max",
            "job",
            "dht_routing_refresh",
            self.dht_routing_refresh_schedule_delay_micros_max,
        );
        push_metric_help(
            &mut out,
            "lm_node_background_schedule_delay_micros_last",
            "Last scheduler delay in microseconds for background jobs.",
        );
        push_metric_type(
            &mut out,
            "lm_node_background_schedule_delay_micros_last",
            "gauge",
        );
        if let Some(value) = self.last_sync_schedule_delay_micros {
            push_labeled_metric_value(
                &mut out,
                "lm_node_background_schedule_delay_micros_last",
                "job",
                "snapshot_sync",
                value,
            );
        }
        if let Some(value) = self.last_dht_replication_schedule_delay_micros {
            push_labeled_metric_value(
                &mut out,
                "lm_node_background_schedule_delay_micros_last",
                "job",
                "dht_replication",
                value,
            );
        }
        if let Some(value) = self.last_dht_routing_refresh_schedule_delay_micros {
            push_labeled_metric_value(
                &mut out,
                "lm_node_background_schedule_delay_micros_last",
                "job",
                "dht_routing_refresh",
                value,
            );
        }
        if let Some(sync_status) = sync_status {
            push_sync_status_metrics(&mut out, sync_status, dht_peer_quarantine_threshold);
        }
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
        push_metric_help(
            &mut out,
            "lm_node_mailbox_push_rejections_total",
            "Rejected mailbox push attempts by reason.",
        );
        push_metric_type(&mut out, "lm_node_mailbox_push_rejections_total", "counter");
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_push_rejections_total",
            "reason",
            "invalid_json",
            maintenance.mailbox_push_rejects.invalid_json,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_push_rejections_total",
            "reason",
            "invalid_message_format",
            maintenance.mailbox_push_rejects.invalid_message_format,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_push_rejections_total",
            "reason",
            "invalid_identity_public_key",
            maintenance.mailbox_push_rejects.invalid_identity_public_key,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_push_rejections_total",
            "reason",
            "invalid_signature",
            maintenance.mailbox_push_rejects.invalid_signature,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_push_rejections_total",
            "reason",
            "expired_object",
            maintenance.mailbox_push_rejects.expired_object,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_push_rejections_total",
            "reason",
            "duplicate_message",
            maintenance.mailbox_push_rejects.duplicate_message,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_push_rejections_total",
            "reason",
            "payload_too_large",
            maintenance.mailbox_push_rejects.payload_too_large,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_push_rejections_total",
            "reason",
            "global_rate_limited",
            maintenance.mailbox_push_rejects.global_rate_limited,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_push_rejections_total",
            "reason",
            "sender_rate_limited",
            maintenance.mailbox_push_rejects.sender_rate_limited,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_push_rejections_total",
            "reason",
            "other",
            maintenance.mailbox_push_rejects.other,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_push_rejections_total",
            "reason",
            "all",
            maintenance.mailbox_push_rejects.total(),
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_record_rejections_total",
            "Rejected DHT record store attempts by reason.",
        );
        push_metric_type(&mut out, "lm_node_dht_record_rejections_total", "counter");
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_record_rejections_total",
            "reason",
            "invalid_json",
            maintenance.dht_record_rejects.invalid_json,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_record_rejections_total",
            "reason",
            "expired",
            maintenance.dht_record_rejects.expired,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_record_rejections_total",
            "reason",
            "ttl_too_long",
            maintenance.dht_record_rejects.ttl_too_long,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_record_rejections_total",
            "reason",
            "payload_too_large",
            maintenance.dht_record_rejects.payload_too_large,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_record_rejections_total",
            "reason",
            "invalid_record",
            maintenance.dht_record_rejects.invalid_record,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_record_rejections_total",
            "reason",
            "all",
            maintenance.dht_record_rejects.total(),
        );
        push_metric_help(
            &mut out,
            "lm_node_mailbox_ack_rejections_total",
            "Rejected mailbox ack attempts by reason.",
        );
        push_metric_type(&mut out, "lm_node_mailbox_ack_rejections_total", "counter");
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_ack_rejections_total",
            "reason",
            "invalid_json",
            maintenance.mailbox_ack_rejects.invalid_json,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_ack_rejections_total",
            "reason",
            "invalid_user_id",
            maintenance.mailbox_ack_rejects.invalid_user_id,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_ack_rejections_total",
            "reason",
            "too_many_ids",
            maintenance.mailbox_ack_rejects.too_many_ids,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_ack_rejections_total",
            "reason",
            "empty_id",
            maintenance.mailbox_ack_rejects.empty_id,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_ack_rejections_total",
            "reason",
            "id_too_large",
            maintenance.mailbox_ack_rejects.id_too_large,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_ack_rejections_total",
            "reason",
            "all",
            maintenance.mailbox_ack_rejects.total(),
        );
        push_metric_help(
            &mut out,
            "lm_node_routing_peer_rejections_total",
            "Rejected DHT routing peer merge attempts by reason.",
        );
        push_metric_type(&mut out, "lm_node_routing_peer_rejections_total", "counter");
        push_labeled_metric_value(
            &mut out,
            "lm_node_routing_peer_rejections_total",
            "reason",
            "expired",
            maintenance.routing_peer_rejects.expired,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_routing_peer_rejections_total",
            "reason",
            "mismatched_node_id",
            maintenance.routing_peer_rejects.mismatched_node_id,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_routing_peer_rejections_total",
            "reason",
            "local_node",
            maintenance.routing_peer_rejects.local_node,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_routing_peer_rejections_total",
            "reason",
            "missing_identity_public_key",
            maintenance.routing_peer_rejects.missing_identity_public_key,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_routing_peer_rejections_total",
            "reason",
            "invalid_identity_public_key",
            maintenance.routing_peer_rejects.invalid_identity_public_key,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_routing_peer_rejections_total",
            "reason",
            "invalid_signature",
            maintenance.routing_peer_rejects.invalid_signature,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_routing_peer_rejections_total",
            "reason",
            "too_many_addresses",
            maintenance.routing_peer_rejects.too_many_addresses,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_routing_peer_rejections_total",
            "reason",
            "address_too_large",
            maintenance.routing_peer_rejects.address_too_large,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_routing_peer_rejections_total",
            "reason",
            "all",
            maintenance.routing_peer_rejects.total(),
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
        if let Some(state_db) = state_db {
            push_metric_help(
                &mut out,
                "lm_node_state_db_pages",
                "SQLite state database page counts.",
            );
            push_metric_type(&mut out, "lm_node_state_db_pages", "gauge");
            push_labeled_metric_value(
                &mut out,
                "lm_node_state_db_pages",
                "kind",
                "total",
                state_db.page_count,
            );
            push_labeled_metric_value(
                &mut out,
                "lm_node_state_db_pages",
                "kind",
                "free",
                state_db.freelist_count,
            );
            push_metric_help(
                &mut out,
                "lm_node_state_db_page_size_bytes",
                "SQLite state database page size in bytes.",
            );
            push_metric_type(&mut out, "lm_node_state_db_page_size_bytes", "gauge");
            push_metric_value(
                &mut out,
                "lm_node_state_db_page_size_bytes",
                state_db.page_size_bytes,
            );
            push_metric_help(
                &mut out,
                "lm_node_state_db_file_bytes",
                "SQLite state database file size in bytes.",
            );
            push_metric_type(&mut out, "lm_node_state_db_file_bytes", "gauge");
            push_metric_value(&mut out, "lm_node_state_db_file_bytes", state_db.file_bytes);
            push_metric_help(
                &mut out,
                "lm_node_state_db_encrypted",
                "Whether the SQLite state database is encrypted at the database layer.",
            );
            push_metric_type(&mut out, "lm_node_state_db_encrypted", "gauge");
            push_metric_value(
                &mut out,
                "lm_node_state_db_encrypted",
                if state_db.encrypted { 1 } else { 0 },
            );
            push_metric_help(
                &mut out,
                "lm_node_state_db_permissions_hardened",
                "Whether the state database files are expected to be restricted to the node user.",
            );
            push_metric_type(&mut out, "lm_node_state_db_permissions_hardened", "gauge");
            push_metric_value(
                &mut out,
                "lm_node_state_db_permissions_hardened",
                if state_db.permissions_hardened { 1 } else { 0 },
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

    fn record_dht_replication_run(&mut self, stats: DhtReplicationRunStats, finished_at: u64) {
        self.dht_replication_runs = self.dht_replication_runs.saturating_add(1);
        self.dht_replication_records = self
            .dht_replication_records
            .saturating_add(stats.records as u64);
        self.dht_replication_attempts = self
            .dht_replication_attempts
            .saturating_add(stats.attempts as u64);
        self.dht_replication_successes = self
            .dht_replication_successes
            .saturating_add(stats.successes as u64);
        self.dht_replication_failures = self
            .dht_replication_failures
            .saturating_add(stats.failures as u64);
        self.dht_replication_peers_quarantined = self
            .dht_replication_peers_quarantined
            .saturating_add(stats.peers_quarantined as u64);
        self.last_dht_replication_at = Some(finished_at);
    }

    fn record_dht_routing_refresh_run(
        &mut self,
        stats: DhtRoutingRefreshRunStats,
        finished_at: u64,
    ) {
        self.dht_routing_refresh_runs = self.dht_routing_refresh_runs.saturating_add(1);
        self.dht_routing_refresh_targets = self
            .dht_routing_refresh_targets
            .saturating_add(stats.targets as u64);
        self.dht_routing_refresh_attempts = self
            .dht_routing_refresh_attempts
            .saturating_add(stats.attempts as u64);
        self.dht_routing_refresh_successes = self
            .dht_routing_refresh_successes
            .saturating_add(stats.successes as u64);
        self.dht_routing_refresh_failures = self
            .dht_routing_refresh_failures
            .saturating_add(stats.failures as u64);
        self.dht_routing_refresh_nodes_returned = self
            .dht_routing_refresh_nodes_returned
            .saturating_add(stats.nodes_returned as u64);
        self.dht_routing_refresh_nodes_merged = self
            .dht_routing_refresh_nodes_merged
            .saturating_add(stats.nodes_merged as u64);
        self.dht_routing_refresh_nodes_rejected_non_closer = self
            .dht_routing_refresh_nodes_rejected_non_closer
            .saturating_add(stats.nodes_rejected_non_closer as u64);
        self.dht_routing_refresh_nodes_rejected_duplicate = self
            .dht_routing_refresh_nodes_rejected_duplicate
            .saturating_add(stats.nodes_rejected_duplicate as u64);
        self.dht_routing_refresh_peers_quarantined = self
            .dht_routing_refresh_peers_quarantined
            .saturating_add(stats.peers_quarantined as u64);
        self.last_dht_routing_refresh_at = Some(finished_at);
    }

    fn record_dht_find_value_run(&mut self, stats: DhtFindValueRunStats, finished_at: u64) {
        self.dht_find_value_runs = self.dht_find_value_runs.saturating_add(1);
        self.dht_find_value_attempts = self
            .dht_find_value_attempts
            .saturating_add(stats.attempts as u64);
        self.dht_find_value_successes = self
            .dht_find_value_successes
            .saturating_add(stats.successes as u64);
        self.dht_find_value_failures = self
            .dht_find_value_failures
            .saturating_add(stats.failures as u64);
        self.dht_find_value_found_records = self
            .dht_find_value_found_records
            .saturating_add(stats.found_records as u64);
        self.dht_find_value_closer_records = self
            .dht_find_value_closer_records
            .saturating_add(stats.closer_records as u64);
        self.dht_find_value_closer_nodes_returned = self
            .dht_find_value_closer_nodes_returned
            .saturating_add(stats.closer_nodes_returned as u64);
        self.dht_find_value_closer_nodes_merged = self
            .dht_find_value_closer_nodes_merged
            .saturating_add(stats.closer_nodes_merged as u64);
        self.dht_find_value_closer_nodes_rejected_non_closer = self
            .dht_find_value_closer_nodes_rejected_non_closer
            .saturating_add(stats.closer_nodes_rejected_non_closer as u64);
        self.dht_find_value_closer_nodes_rejected_duplicate = self
            .dht_find_value_closer_nodes_rejected_duplicate
            .saturating_add(stats.closer_nodes_rejected_duplicate as u64);
        self.dht_find_value_peers_quarantined = self
            .dht_find_value_peers_quarantined
            .saturating_add(stats.peers_quarantined as u64);
        self.dht_find_value_query_rounds = self
            .dht_find_value_query_rounds
            .saturating_add(stats.query_rounds as u64);
        self.dht_find_value_alpha_max = self.dht_find_value_alpha_max.max(stats.alpha);
        if stats.exhausted {
            self.dht_find_value_exhausted = self.dht_find_value_exhausted.saturating_add(1);
        }
        self.last_dht_find_value_at = Some(finished_at);
    }

    fn record_sync_schedule_delay(&mut self, delay: Duration) {
        let micros = delay.as_micros();
        self.sync_schedule_delay_micros_total =
            self.sync_schedule_delay_micros_total.saturating_add(micros);
        self.sync_schedule_delay_micros_max = self.sync_schedule_delay_micros_max.max(micros);
        self.last_sync_schedule_delay_micros = Some(micros);
    }

    fn record_dht_replication_schedule_delay(&mut self, delay: Duration) {
        let micros = delay.as_micros();
        self.dht_replication_schedule_delay_micros_total = self
            .dht_replication_schedule_delay_micros_total
            .saturating_add(micros);
        self.dht_replication_schedule_delay_micros_max =
            self.dht_replication_schedule_delay_micros_max.max(micros);
        self.last_dht_replication_schedule_delay_micros = Some(micros);
    }

    fn record_dht_routing_refresh_schedule_delay(&mut self, delay: Duration) {
        let micros = delay.as_micros();
        self.dht_routing_refresh_schedule_delay_micros_total = self
            .dht_routing_refresh_schedule_delay_micros_total
            .saturating_add(micros);
        self.dht_routing_refresh_schedule_delay_micros_max = self
            .dht_routing_refresh_schedule_delay_micros_max
            .max(micros);
        self.last_dht_routing_refresh_schedule_delay_micros = Some(micros);
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

fn push_sync_status_metrics(
    out: &mut String,
    sync_status: &NodeSyncStatus,
    dht_peer_quarantine_threshold: u32,
) {
    push_metric_help(
        out,
        "lm_node_sync_peer_attempts_total",
        "Snapshot sync attempts by peer.",
    );
    push_metric_type(out, "lm_node_sync_peer_attempts_total", "counter");
    push_metric_help(
        out,
        "lm_node_sync_peer_successes_total",
        "Snapshot sync successes by peer.",
    );
    push_metric_type(out, "lm_node_sync_peer_successes_total", "counter");
    push_metric_help(
        out,
        "lm_node_sync_peer_failures_total",
        "Snapshot sync failures by peer.",
    );
    push_metric_type(out, "lm_node_sync_peer_failures_total", "counter");
    push_metric_help(
        out,
        "lm_node_sync_peer_consecutive_failures",
        "Current consecutive snapshot sync failures by peer.",
    );
    push_metric_type(out, "lm_node_sync_peer_consecutive_failures", "gauge");
    push_metric_help(
        out,
        "lm_node_sync_peer_next_attempt_at",
        "Unix timestamp for the next scheduled snapshot sync attempt by peer.",
    );
    push_metric_type(out, "lm_node_sync_peer_next_attempt_at", "gauge");
    push_metric_help(
        out,
        "lm_node_dht_peer_quarantined",
        "Whether a sync/control peer is currently skipped by DHT runners due to consecutive failures.",
    );
    push_metric_type(out, "lm_node_dht_peer_quarantined", "gauge");
    for (peer, status) in &sync_status.peers {
        push_labeled_metric_value(
            out,
            "lm_node_sync_peer_attempts_total",
            "peer",
            peer,
            status.attempts,
        );
        push_labeled_metric_value(
            out,
            "lm_node_sync_peer_successes_total",
            "peer",
            peer,
            status.successes,
        );
        push_labeled_metric_value(
            out,
            "lm_node_sync_peer_failures_total",
            "peer",
            peer,
            status.failures,
        );
        push_labeled_metric_value(
            out,
            "lm_node_sync_peer_consecutive_failures",
            "peer",
            peer,
            status.consecutive_failures,
        );
        push_labeled_metric_value(
            out,
            "lm_node_dht_peer_quarantined",
            "peer",
            peer,
            if sync_peer_is_dht_quarantined(
                status,
                current_unix_timestamp(),
                dht_peer_quarantine_threshold,
            ) {
                1
            } else {
                0
            },
        );
        if let Some(next_attempt_at) = status.next_attempt_at {
            push_labeled_metric_value(
                out,
                "lm_node_sync_peer_next_attempt_at",
                "peer",
                peer,
                next_attempt_at,
            );
        }
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
    state_db: Option<String>,
    state_db_require_encryption: Option<bool>,
    control_token: Option<String>,
    control_token_file: Option<String>,
    control_previous_tokens: Option<Vec<String>>,
    cors_allow_origins: Option<Vec<String>>,
    sync_peers: Option<Vec<SyncPeerConfigFile>>,
    sync_interval_seconds: Option<u64>,
    sync_max_backoff_seconds: Option<u64>,
    dht_replication_factor: Option<usize>,
    dht_routing_refresh_limit: Option<usize>,
    dht_routing_refresh_max_targets: Option<usize>,
    dht_transport: Option<String>,
    dht_peer_quarantine_consecutive_failures: Option<u32>,
    rate_limit_window_seconds: Option<u64>,
    rate_limit_max_requests: Option<u32>,
    log_format: Option<String>,
    mailbox_sender_rate_limit_window_seconds: Option<u64>,
    mailbox_sender_rate_limit_max_messages: Option<u32>,
    mailbox_global_rate_limit_window_seconds: Option<u64>,
    mailbox_global_rate_limit_max_messages: Option<u32>,
    max_mailbox_bytes: Option<u64>,
    max_mailbox_bytes_per_user: Option<u64>,
    max_mailbox_messages_per_user: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
struct SyncPeerConfigFile {
    url: String,
    peer_id: Option<String>,
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
    fn has_bearer_tokens(&self) -> bool {
        self.token.is_some() || !self.previous_tokens.is_empty()
    }

    fn is_loopback_only(&self) -> bool {
        !self.has_bearer_tokens()
    }

    fn token_matches(&self, value: &str) -> bool {
        self.token
            .as_deref()
            .into_iter()
            .chain(self.previous_tokens.iter().map(String::as_str))
            .any(|token| constant_time_eq(value.as_bytes(), token.as_bytes()))
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

fn open_state_db(path: &str) -> Result<Connection, Box<dyn std::error::Error>> {
    if let Some(parent) = Path::new(path)
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    init_state_db(&conn)?;
    set_state_db_private_permissions(Path::new(path))?;
    Ok(conn)
}

fn init_state_db(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = FULL;
        PRAGMA busy_timeout = 5000;
        PRAGMA foreign_keys = ON;
        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS public_peers (
            peer_id TEXT PRIMARY KEY,
            announce_json TEXT NOT NULL,
            routing_peer_json TEXT
        );
        CREATE TABLE IF NOT EXISTS mailbox_deliveries (
            delivery_id TEXT PRIMARY KEY,
            to_user_id TEXT NOT NULL,
            message_id TEXT NOT NULL,
            expires_at INTEGER NOT NULL,
            delivery_json TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_mailbox_deliveries_to_user
            ON mailbox_deliveries(to_user_id);
        CREATE INDEX IF NOT EXISTS idx_mailbox_deliveries_expires_at
            ON mailbox_deliveries(expires_at);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_mailbox_deliveries_to_user_message_id
            ON mailbox_deliveries(to_user_id, message_id);
        CREATE TABLE IF NOT EXISTS mailbox_ack_receipts (
            delivery_id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            acked_at INTEGER NOT NULL,
            expires_at INTEGER NOT NULL,
            receipt_json TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_mailbox_ack_receipts_user_id
            ON mailbox_ack_receipts(user_id);
        CREATE INDEX IF NOT EXISTS idx_mailbox_ack_receipts_expires_at
            ON mailbox_ack_receipts(expires_at);
        CREATE TABLE IF NOT EXISTS prekey_bundles (
            user_id TEXT PRIMARY KEY,
            expires_at INTEGER NOT NULL,
            signed_prekey_expires_at INTEGER NOT NULL,
            bundle_json TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_prekey_bundles_expires_at
            ON prekey_bundles(expires_at);
        CREATE TABLE IF NOT EXISTS signed_one_time_prekey_records (
            user_id TEXT NOT NULL,
            signed_prekey_id INTEGER NOT NULL,
            key_id INTEGER NOT NULL,
            expires_at INTEGER NOT NULL,
            record_json TEXT NOT NULL,
            PRIMARY KEY(user_id, signed_prekey_id, key_id)
        );
        CREATE INDEX IF NOT EXISTS idx_signed_one_time_prekey_records_expires_at
            ON signed_one_time_prekey_records(expires_at);
        CREATE TABLE IF NOT EXISTS consumed_one_time_prekeys (
            user_id TEXT NOT NULL,
            key_id INTEGER NOT NULL,
            PRIMARY KEY(user_id, key_id)
        );
        CREATE TABLE IF NOT EXISTS dht_records (
            record_key TEXT PRIMARY KEY,
            kind TEXT NOT NULL,
            expires_at INTEGER NOT NULL,
            republish_at INTEGER NOT NULL,
            record_json TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_dht_records_expires_at
            ON dht_records(expires_at);
        "#,
    )?;
    ensure_column(
        conn,
        "public_peers",
        "routing_peer_json",
        "ALTER TABLE public_peers ADD COLUMN routing_peer_json TEXT",
    )?;
    Ok(())
}

fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    alter_sql: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for existing in columns {
        if existing? == column {
            return Ok(());
        }
    }
    conn.execute(alter_sql, [])?;
    Ok(())
}

fn load_node_state_db(path: &str) -> Result<NativeNode, Box<dyn std::error::Error>> {
    let conn = open_state_db(path)?;
    let version = db_get_json::<u16>(&conn, "version")?.unwrap_or(1);
    let config = db_get_json::<NodeConfig>(&conn, "config")?
        .ok_or_else(|| format!("state db has no saved config: {path}"))?;
    let sync_status = db_get_json::<NodeSyncStatus>(&conn, "sync_status")?.unwrap_or_default();
    let maintenance =
        db_get_json::<NodeMaintenanceStats>(&conn, "maintenance")?.unwrap_or_default();
    let public_peers = db_get_all_json(&conn, "SELECT announce_json FROM public_peers")?;
    let routing_peers = db_get_all_json::<RoutingPeer>(
        &conn,
        "SELECT routing_peer_json FROM public_peers WHERE routing_peer_json IS NOT NULL",
    )?;
    let mailbox_deliveries =
        db_get_all_json::<MailboxDelivery>(&conn, "SELECT delivery_json FROM mailbox_deliveries")?;
    let prekey_bundles = db_get_all_json(&conn, "SELECT bundle_json FROM prekey_bundles")?;
    let signed_one_time_prekey_records = db_get_all_json(
        &conn,
        "SELECT record_json FROM signed_one_time_prekey_records",
    )?;
    let consumed_one_time_prekeys = db_get_consumed_prekeys(&conn)?;
    let dht_records = db_get_all_json::<DhtRecord>(&conn, "SELECT record_json FROM dht_records")?;
    Ok(NativeNode::from_state_snapshot(NodeStateSnapshot {
        version,
        config,
        public_peers,
        routing_peers,
        mailbox_deliveries,
        mailbox_ack_receipts: db_get_all_json::<lm_node::MailboxAckReceipt>(
            &conn,
            "SELECT receipt_json FROM mailbox_ack_receipts",
        )?,
        mailbox_messages: Vec::new(),
        prekey_bundles,
        signed_one_time_prekey_records,
        consumed_one_time_prekeys,
        dht_records,
        sync_status,
        maintenance,
    }))
}

fn save_node_state_db(path: &str, node: &NativeNode) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = open_state_db(path)?;
    let snapshot = node.to_state_snapshot();
    let tx = conn.transaction()?;
    db_set_json_tx(&tx, "version", &snapshot.version)?;
    db_set_json_tx(&tx, "config", &snapshot.config)?;
    db_set_json_tx(&tx, "sync_status", &snapshot.sync_status)?;
    db_set_json_tx(&tx, "maintenance", &snapshot.maintenance)?;
    tx.execute("DELETE FROM public_peers", [])?;
    let routing_peers_by_id = snapshot
        .routing_peers
        .iter()
        .map(|peer| (peer.announce.peer_id.as_str(), peer))
        .collect::<HashMap<_, _>>();
    for peer in &snapshot.public_peers {
        let routing_peer_json = routing_peers_by_id
            .get(peer.peer_id.as_str())
            .map(|routing_peer| serde_json::to_string(routing_peer))
            .transpose()?;
        tx.execute(
            "INSERT INTO public_peers(peer_id, announce_json, routing_peer_json) VALUES (?1, ?2, ?3)",
            params![peer.peer_id, serde_json::to_string(peer)?, routing_peer_json],
        )?;
    }
    tx.execute("DELETE FROM mailbox_deliveries", [])?;
    for delivery in &snapshot.mailbox_deliveries {
        tx.execute(
            "INSERT INTO mailbox_deliveries(delivery_id, to_user_id, message_id, expires_at, delivery_json)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                delivery.delivery_id,
                delivery.message.to_user_id.to_string(),
                delivery.message.message_id.to_string(),
                delivery.message.expires_at as i64,
                serde_json::to_string(delivery)?,
            ],
        )?;
    }
    tx.execute("DELETE FROM mailbox_ack_receipts", [])?;
    for receipt in &snapshot.mailbox_ack_receipts {
        tx.execute(
            "INSERT INTO mailbox_ack_receipts(delivery_id, user_id, acked_at, expires_at, receipt_json)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                receipt.delivery_id,
                receipt.user_id.to_string(),
                receipt.acked_at as i64,
                receipt.expires_at as i64,
                serde_json::to_string(receipt)?,
            ],
        )?;
    }
    tx.execute("DELETE FROM prekey_bundles", [])?;
    for bundle in &snapshot.prekey_bundles {
        tx.execute(
            "INSERT INTO prekey_bundles(user_id, expires_at, signed_prekey_expires_at, bundle_json)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                bundle.user_id.to_string(),
                bundle.expires_at as i64,
                bundle.signed_prekey_expires_at as i64,
                serde_json::to_string(bundle)?,
            ],
        )?;
    }
    tx.execute("DELETE FROM signed_one_time_prekey_records", [])?;
    for record in &snapshot.signed_one_time_prekey_records {
        tx.execute(
            "INSERT INTO signed_one_time_prekey_records(user_id, signed_prekey_id, key_id, expires_at, record_json)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                record.user_id.to_string(),
                record.signed_prekey_id as i64,
                record.key_id as i64,
                record.expires_at as i64,
                serde_json::to_string(record)?,
            ],
        )?;
    }
    tx.execute("DELETE FROM consumed_one_time_prekeys", [])?;
    for item in &snapshot.consumed_one_time_prekeys {
        tx.execute(
            "INSERT INTO consumed_one_time_prekeys(user_id, key_id) VALUES (?1, ?2)",
            params![item.user_id.to_string(), item.key_id as i64],
        )?;
    }
    tx.execute("DELETE FROM dht_records", [])?;
    for record in &snapshot.dht_records {
        tx.execute(
            "INSERT INTO dht_records(record_key, kind, expires_at, republish_at, record_json)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                record.key.to_hex(),
                format!("{:?}", record.kind),
                record.expires_at as i64,
                record.republish_at as i64,
                serde_json::to_string(record)?,
            ],
        )?;
    }
    tx.commit()?;
    set_state_db_private_permissions(Path::new(path))?;
    Ok(())
}

fn state_db_stats(path: &str) -> Result<StateDbStats, Box<dyn std::error::Error>> {
    let conn = open_state_db(path)?;
    let page_count: u64 =
        conn.query_row("PRAGMA page_count", [], |row| row.get::<_, i64>(0))? as u64;
    let page_size_bytes: u64 =
        conn.query_row("PRAGMA page_size", [], |row| row.get::<_, i64>(0))? as u64;
    let freelist_count: u64 =
        conn.query_row("PRAGMA freelist_count", [], |row| row.get::<_, i64>(0))? as u64;
    let file_bytes = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    Ok(StateDbStats {
        page_count,
        page_size_bytes,
        freelist_count,
        file_bytes,
        encrypted: false,
        permissions_hardened: true,
    })
}

fn state_db_stats_opt(path: Option<&str>) -> Option<StateDbStats> {
    path.and_then(|path| state_db_stats(path).ok())
}

fn db_get_json<T: DeserializeOwned>(
    conn: &Connection,
    key: &str,
) -> Result<Option<T>, Box<dyn std::error::Error>> {
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()?;
    value
        .map(|value| serde_json::from_str(&value).map_err(Into::into))
        .transpose()
}

fn db_set_json_tx<T: Serialize>(
    tx: &rusqlite::Transaction<'_>,
    key: &str,
    value: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    tx.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, serde_json::to_string(value)?],
    )?;
    Ok(())
}

fn db_get_all_json<T: DeserializeOwned>(
    conn: &Connection,
    sql: &str,
) -> Result<Vec<T>, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare(sql)?;
    let values = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    values
        .into_iter()
        .map(|value| serde_json::from_str(&value).map_err(Into::into))
        .collect()
}

fn db_get_consumed_prekeys(
    conn: &Connection,
) -> Result<Vec<ConsumedOneTimePreKey>, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare(
        "SELECT user_id, key_id FROM consumed_one_time_prekeys ORDER BY user_id, key_id",
    )?;
    let rows = stmt
        .query_map([], |row| {
            let user_id: String = row.get(0)?;
            let key_id: i64 = row.get(1)?;
            Ok((user_id, key_id))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(|(user_id, key_id)| {
            Ok(ConsumedOneTimePreKey {
                user_id: lm_core::UserId::from_raw(user_id)?,
                key_id: key_id as u32,
            })
        })
        .collect()
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
        let mut file = create_private_file(&tmp_path)?;
        file.write_all(text.as_bytes())?;
        file.write_all(b"\n")?;
        file.sync_all()?;
    }
    fs::rename(&tmp_path, path)?;
    set_private_file_permissions(path)?;
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

fn create_private_file(path: &Path) -> Result<File, Box<dyn std::error::Error>> {
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(path)?;
    set_private_file_permissions(path)?;
    Ok(file)
}

#[cfg(unix)]
fn set_private_file_permissions(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;
    if path.exists() {
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn set_private_file_permissions(_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

fn set_state_db_private_permissions(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    set_private_file_permissions(path)?;
    set_private_file_permissions(&path_with_suffix(path, "-wal"))?;
    set_private_file_permissions(&path_with_suffix(path, "-shm"))?;
    Ok(())
}

fn path_with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
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

fn status_for_request_error(error: &str) -> u16 {
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

fn status_reason(status: u16) -> &'static str {
    match status {
        400 => "Bad Request",
        413 => "Payload Too Large",
        431 => "Request Header Fields Too Large",
        _ => "Bad Request",
    }
}

fn control_error_http_response(status: u16, body: &str) -> String {
    format!(
        "HTTP/1.1 {} {}\r\ncontent-type: text/plain; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        status,
        status_reason(status),
        body.len(),
        body
    )
}

fn serve_control(
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
            if let Some(path) = state_file {
                if let Err(err) = save_node_state(path, node) {
                    logger.error(
                        "state_file.save_error",
                        format!("state save error: {err}"),
                        serde_json::json!({"path": path, "error": err.to_string()}),
                    );
                }
            }
            if let Some(path) = state_db {
                if let Err(err) = save_node_state_db(path, node) {
                    logger.error(
                        "state_db.save_error",
                        format!("state db save error: {err}"),
                        serde_json::json!({"path": path, "error": err.to_string()}),
                    );
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
                } else if let Some(path) = state_file {
                    if let Err(err) = save_node_state(path, node) {
                        logger.error(
                            "state_file.save_error",
                            format!("state save error: {err}"),
                            serde_json::json!({"path": path, "error": err.to_string()}),
                        );
                    }
                }
                if let Some(path) = state_db {
                    if let Err(err) = save_node_state_db(path, node) {
                        logger.error(
                            "state_db.save_error",
                            format!("state db save error: {err}"),
                            serde_json::json!({"path": path, "error": err.to_string()}),
                        );
                    }
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

#[cfg(test)]
fn run_dht_replication(
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

fn run_dht_replication_with_logger(
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
fn dht_runner_peer_configs(
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

fn dht_runner_peer_configs_with_quarantine_count(
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

fn dht_peer_is_quarantined(node: &NativeNode, peer: &SyncPeerConfig, threshold: u32) -> bool {
    node.sync_status
        .peers
        .get(&peer.url)
        .map(|status| sync_peer_is_dht_quarantined(status, current_unix_timestamp(), threshold))
        .unwrap_or(false)
}

fn sync_peer_is_dht_quarantined(status: &NodeSyncPeerStatus, now: u64, threshold: u32) -> bool {
    threshold > 0
        && status.consecutive_failures >= threshold
        && status
            .next_attempt_at
            .map(|next| next > now)
            .unwrap_or(true)
}

fn dht_peer_health_sort_key(node: &NativeNode, peer: &SyncPeerConfig) -> (u32, u64, u64) {
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

fn sync_peer_config_from_http_routing_peer(peer: &RoutingPeer) -> Option<SyncPeerConfig> {
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

fn sync_peer_config_from_libp2p_routing_peer(peer: &RoutingPeer) -> Option<SyncPeerConfig> {
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

fn record_dht_peer_success(node: &mut NativeNode, peer: &SyncPeerConfig) {
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

fn record_dht_peer_failure(node: &mut NativeNode, peer: &SyncPeerConfig, error: impl Into<String>) {
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

fn run_dht_replication_with_transport(
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

fn filter_routing_refresh_nodes(
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

fn routing_peer_makes_find_node_progress(
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

fn routing_peer_is_seed_peer(seed_peer: &SyncPeerConfig, candidate: &RoutingPeer) -> bool {
    candidate.announce.peer_id == seed_peer.peer_id.as_deref().unwrap_or_default()
        || sync_peer_config_from_routing_peer_for_seed(seed_peer, candidate)
            .map(|candidate_config| candidate_config.url == seed_peer.url)
            .unwrap_or(false)
}

#[cfg(test)]
fn run_dht_routing_refresh(
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

fn run_dht_routing_refresh_with_logger(
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

fn run_dht_routing_refresh_with_transport(
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
fn dht_find_value_with_transport(
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
                    if let Some(record) = record {
                        if node.accept_dht_record_from_peer(record) {
                            stats.found_records = stats.found_records.saturating_add(1);
                            useful_response = true;
                            found = true;
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
                    if returned > 0 && rejected == returned && !useful_response {
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
fn merge_find_value_closer_results(
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

fn dht_query_peer_dedup_key(peer: &SyncPeerConfig) -> (String, Option<String>) {
    (peer.url.clone(), peer.peer_id.clone())
}

fn candidate_is_seed_peer(seed_peer: &SyncPeerConfig, candidate: &SyncPeerConfig) -> bool {
    candidate.url == seed_peer.url
        || candidate
            .peer_id
            .as_deref()
            .zip(seed_peer.peer_id.as_deref())
            .map(|(candidate, seed)| candidate == seed)
            .unwrap_or(false)
}

fn routing_peer_makes_find_value_progress(
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

fn sync_peer_config_from_routing_peer_for_seed(
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
fn dht_response_record_limit(requested: usize) -> usize {
    requested.clamp(1, MAX_DHT_RESPONSE_RECORDS)
}

fn dht_response_node_limit(requested: usize) -> usize {
    requested.clamp(1, MAX_DHT_RESPONSE_NODES)
}

#[allow(dead_code)]
fn send_dht_find_value(
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

fn run_snapshot_sync(
    node: &mut NativeNode,
    peer: &mut SyncPeerRuntime,
    base_interval_seconds: u64,
    max_backoff_seconds: u64,
    logger: &ControlLogger,
) {
    let delay_seconds;
    match fetch_snapshot(&peer.config) {
        Ok(snapshot) => {
            let stats = node.merge_snapshot(snapshot);
            node.sync_status.record_success(&peer.config.url, stats);
            peer.consecutive_failures = 0;
            delay_seconds = base_interval_seconds.max(1);
            logger.info(
                "sync.snapshot.success",
                format!(
                    "snapshot sync from {}: peers={} mailbox_deliveries={} prekey_bundles={} signed_one_time_prekey_records={}",
                    peer.config.url,
                    stats.peers,
                    stats.mailbox_deliveries,
                    stats.prekey_bundles,
                    stats.signed_one_time_prekey_records
                ),
                serde_json::json!({
                    "peer": peer.config.url,
                    "peers": stats.peers,
                    "mailbox_deliveries": stats.mailbox_deliveries,
                    "prekey_bundles": stats.prekey_bundles,
                    "signed_one_time_prekey_records": stats.signed_one_time_prekey_records,
                    "dht_records": stats.dht_records,
                }),
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
            logger.error(
                "sync.snapshot.error",
                format!("snapshot sync from {} failed: {error}", peer.config.url),
                serde_json::json!({"peer": peer.config.url, "error": error}),
            );
        }
    }
    peer.next_attempt_at = Instant::now() + Duration::from_secs(delay_seconds);
    node.sync_status.record_next_attempt(
        &peer.config.url,
        current_unix_timestamp().saturating_add(delay_seconds),
    );
}

fn log_warn_or_stderr(
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

fn log_error_or_stderr(
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

#[cfg(test)]
fn send_dht_rpc(
    peer: &SyncPeerConfig,
    request: &DhtRpcRequest,
) -> Result<DhtRpcResponse, Box<dyn std::error::Error>> {
    HttpControlDhtTransport.send_dht_rpc(peer, request)
}

fn fetch_snapshot(peer: &SyncPeerConfig) -> Result<NodeStateSnapshot, Box<dyn std::error::Error>> {
    let body = http_control_request(peer, "GET", "/sync/snapshot", "")?;
    Ok(serde_json::from_str(&body)?)
}

fn http_control_request(
    peer: &SyncPeerConfig,
    method: &str,
    path: &str,
    body: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let normalized = peer.url.trim().trim_end_matches('/');
    let without_scheme = normalized
        .strip_prefix("http://")
        .ok_or("only http:// control peers are supported")?;
    let (host_port, path_prefix) = without_scheme
        .split_once('/')
        .map(|(host, path)| (host, format!("/{path}")))
        .unwrap_or((without_scheme, String::new()));
    let path = format!("{path_prefix}{path}");
    let mut stream = connect_control_peer(host_port)?;
    configure_control_peer_stream(&stream)?;
    let auth_header = peer
        .token
        .as_ref()
        .map(|token| format!("authorization: Bearer {token}\r\n"))
        .unwrap_or_default();
    let content_headers = if body.is_empty() {
        String::new()
    } else {
        format!(
            "content-type: application/json\r\ncontent-length: {}\r\n",
            body.len()
        )
    };
    let request = format!(
        "{method} {path} HTTP/1.1\r\nhost: {host_port}\r\n{auth_header}{content_headers}connection: close\r\n\r\n{body}"
    );
    stream.write_all(request.as_bytes())?;
    let response = read_http_response_limited(&mut stream, MAX_CONTROL_PEER_RESPONSE_BYTES)?;
    let response = String::from_utf8(response)?;
    let (headers, body) = response
        .split_once("\r\n\r\n")
        .ok_or("invalid http response")?;
    let status_line = headers.lines().next().ok_or("missing status line")?;
    if !status_line.contains(" 200 ") && !status_line.contains(" 201 ") {
        return Err(format!("control peer returned {status_line}").into());
    }
    Ok(body.to_string())
}

fn connect_control_peer(host_port: &str) -> Result<TcpStream, Box<dyn std::error::Error>> {
    let mut last_error: Option<std::io::Error> = None;
    for addr in host_port.to_socket_addrs()? {
        match TcpStream::connect_timeout(&addr, CONTROL_PEER_CONNECT_TIMEOUT) {
            Ok(stream) => return Ok(stream),
            Err(err) => last_error = Some(err),
        }
    }
    Err(last_error
        .map(|err| format!("control peer connect failed: {err}"))
        .unwrap_or_else(|| "control peer resolved no addresses".to_string())
        .into())
}

fn configure_control_peer_stream(stream: &TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    stream.set_read_timeout(Some(CONTROL_PEER_HTTP_TIMEOUT))?;
    stream.set_write_timeout(Some(CONTROL_PEER_HTTP_TIMEOUT))?;
    Ok(())
}

fn configure_control_client_stream(stream: &TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    stream.set_read_timeout(Some(CONTROL_CLIENT_HTTP_TIMEOUT))?;
    stream.set_write_timeout(Some(CONTROL_CLIENT_HTTP_TIMEOUT))?;
    Ok(())
}

fn read_http_response_limited(
    stream: &mut TcpStream,
    max_bytes: usize,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut response = Vec::new();
    let mut temp = [0u8; 4096];
    loop {
        let n = stream.read(&mut temp)?;
        if n == 0 {
            break;
        }
        response.extend_from_slice(&temp[..n]);
        if response.len() > max_bytes {
            return Err("control peer response too large".into());
        }
    }
    Ok(response)
}

fn handle_stream(
    stream: &mut TcpStream,
    node: &mut NativeNode,
    security: &ControlSecurityConfig,
    rate_limiter: &mut RateLimiter,
    rate_limit: RateLimitConfig,
    runtime_stats: &mut ControlRuntimeStats,
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
            },
        )
    } else if request.method == "GET" && request.path.starts_with("/control/metrics") {
        node.prune_expired_records();
        ControlHttpResponse::openmetrics(
            200,
            &runtime_stats.to_openmetrics(
                node.maintenance_stats(),
                state_db_stats_opt(state_db).as_ref(),
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

fn handle_control_dht_maintenance_run(
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
    if let Some(runtime_stats) = runtime_stats.as_deref_mut() {
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

fn handle_control_dht_replication_run(
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

fn handle_control_dht_routing_refresh_run(
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

fn handle_control_dht_find_value_run(
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
    let found = node.dht_records.find_value(&key).is_some();
    ControlHttpResponse::json(
        200,
        &DhtFindValueRunResponse {
            key: key.to_hex(),
            found,
            records: node.dht_records.len(),
            stats,
        },
    )
}

fn dht_record_key_from_find_value_query(path: &str) -> Result<lm_node::DhtRecordKey, String> {
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

fn query_param_value(path: &str, name: &str) -> Option<String> {
    let (_, query) = path.split_once('?')?;
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        if key == name {
            return Some(percent_decode_query_component(value));
        }
    }
    None
}

fn percent_decode_query_component(value: &str) -> String {
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

fn from_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
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

fn parse_headers(headers: &str) -> Vec<(String, String)> {
    headers
        .lines()
        .skip(1)
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_string()))
        .collect()
}

fn parse_content_length_and_validate_headers(
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
serve-dht-libp2p [--listen <multiaddr>] [--bootstrap-peer <libp2p://multiaddr|peer_id,csv>] [--peer-id <id>] [--state-file <file>] [--state-db <sqlite>] [--state-db-require-encryption <true|false>]\n  \
serve-control [--config-file <json>] [--bind <host:port>] [--peer-id <id>] [--state-file <file>] [--state-db <sqlite>] [--state-db-require-encryption <true|false>] [--control-token <token>] [--control-previous-token <old-token,csv>] [--sync-peer <url,csv>] [--sync-interval-seconds <n>] [--dht-transport <http-control|libp2p>] [--dht-peer-quarantine-consecutive-failures <n>] [--rate-limit-window-seconds <n>] [--rate-limit-max-requests <n>] [--log-format <text|json>] [--mailbox-global-rate-limit-window-seconds <n>] [--mailbox-global-rate-limit-max-messages <n>] [--mailbox-sender-rate-limit-window-seconds <n>] [--mailbox-sender-rate-limit-max-messages <n>]\n"
    );
}

#[cfg(test)]
mod tests {
    use super::{
        CONTROL_CLIENT_HTTP_TIMEOUT, CONTROL_PEER_CONNECT_TIMEOUT, CONTROL_PEER_HTTP_TIMEOUT,
        ControlHttpResponse, ControlLogger, ControlRuntimeStats, ControlSecurityConfig,
        DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES, DhtFindValueRunStats,
        DhtReplicationRunStats, DhtRoutingRefreshRunStats, DhtRunnerConfig, DhtTransport,
        DhtTransportKind, LIBP2P_DHT_RPC_PROTOCOL, Libp2pDhtTransport, LogFormat,
        MAX_CONTROL_PEER_RESPONSE_BYTES, MAX_CONTROL_REQUEST_HEADER_LINE_BYTES,
        NodeMaintenanceStats, RateLimitConfig, RateLimiter, ServeControlConfigFile, StateDbStats,
        SyncPeerConfig, atomic_write_text, configure_control_client_stream,
        configure_control_peer_stream, connect_control_peer, control_error_http_response,
        current_unix_timestamp, dht_find_value_with_transport, dht_runner_peer_configs,
        dht_runner_peer_configs_with_quarantine_count, dial_libp2p_bootstrap_peers,
        handle_control_dht_find_value_run, handle_control_dht_maintenance_run,
        handle_control_dht_replication_run, handle_control_dht_routing_refresh_run,
        handle_libp2p_dht_rpc_request, handle_libp2p_dht_server_event, http_control_request,
        libp2p_dht_rpc_behaviour, libp2p_dht_swarm, load_node_state_db, open_state_db,
        parse_content_length_and_validate_headers, parse_dht_transport_kind,
        parse_libp2p_bootstrap_peers, parse_libp2p_dht_peer, parse_log_format, read_secret_file,
        request_is_authorized, run_dht_replication, run_dht_replication_with_transport,
        run_dht_routing_refresh, run_dht_routing_refresh_with_transport, save_node_state_db,
        send_dht_rpc, send_libp2p_dht_rpc_async, status_for_request_error, status_reason,
        sync_backoff_delay_seconds, validate_state_db_encryption_requirement,
    };
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
    use futures::StreamExt;
    use libp2p::swarm::SwarmEvent;
    use lm_core::{Identity, MailboxMessage, MailboxMessageKind, PreKeyBundle};
    use lm_node::{
        DhtRecord, DhtRecordKey, DhtRecordKind, DhtRecordRejectStats, DhtRpcRequest,
        DhtRpcResponse, MailboxPushRejectStats, NativeNode, NodeConfig, NodeSyncStatus,
    };
    use std::collections::HashSet;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::{
        Barrier, Mutex,
        atomic::{AtomicUsize, Ordering},
    };
    use std::time::Duration;

    #[derive(Default)]
    struct FakeDhtTransport {
        requests: Mutex<Vec<(String, DhtRpcRequest)>>,
        responses: Mutex<Vec<DhtRpcResponse>>,
    }

    impl DhtTransport for FakeDhtTransport {
        fn send_dht_rpc(
            &self,
            peer: &SyncPeerConfig,
            request: &DhtRpcRequest,
        ) -> Result<DhtRpcResponse, Box<dyn std::error::Error>> {
            self.requests
                .lock()
                .unwrap()
                .push((peer.url.clone(), request.clone()));
            self.responses
                .lock()
                .unwrap()
                .pop()
                .ok_or_else(|| "fake DHT response exhausted".into())
        }
    }

    fn extract_dht_rpc_request_id_from_http(raw: &str) -> String {
        let body = raw
            .split_once("\r\n\r\n")
            .map(|(_, body)| body)
            .unwrap_or_default();
        let value: serde_json::Value = serde_json::from_str(body).unwrap();
        let request = value.get("request").unwrap();
        for kind in ["FindNode", "FindValue", "StoreRecord"] {
            if let Some(value) = request.get(kind) {
                if let Some(request_id) = value.get("request_id").and_then(|v| v.as_str()) {
                    return request_id.to_string();
                }
            }
        }
        panic!("missing DHT RPC request_id in {body}");
    }

    async fn libp2p_dht_roundtrip(
        server_node: &mut NativeNode,
        request: DhtRpcRequest,
    ) -> DhtRpcResponse {
        let mut server_swarm = libp2p_dht_swarm().unwrap();
        let server_peer = *server_swarm.local_peer_id();
        server_swarm
            .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
            .unwrap();

        let listen_addr = loop {
            if let SwarmEvent::NewListenAddr { address, .. } = server_swarm.select_next_some().await
            {
                break address;
            }
        };

        let client_request =
            send_libp2p_dht_rpc_async(server_peer, listen_addr, request, Duration::from_secs(10));
        let server = async {
            let mut pending_discovery = HashSet::new();
            loop {
                let event = server_swarm.select_next_some().await;
                if handle_libp2p_dht_server_event(
                    server_node,
                    &mut server_swarm,
                    &mut pending_discovery,
                    event,
                )
                .is_some()
                {
                    break;
                }
            }
        };

        let (response, _) = futures::future::join(client_request, server).await;
        response.unwrap()
    }

    async fn libp2p_dht_transport_roundtrip(
        server_node: &mut NativeNode,
        request: DhtRpcRequest,
    ) -> DhtRpcResponse {
        let mut server_swarm = libp2p_dht_swarm().unwrap();
        let server_peer = *server_swarm.local_peer_id();
        server_swarm
            .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
            .unwrap();
        let listen_addr = loop {
            if let SwarmEvent::NewListenAddr { address, .. } = server_swarm.select_next_some().await
            {
                break address;
            }
        };
        let peer = SyncPeerConfig {
            url: format!("libp2p://{listen_addr}"),
            token: None,
            peer_id: Some(server_peer.to_string()),
        };
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let transport = Libp2pDhtTransport {
                timeout: Duration::from_secs(10),
            };
            let response = transport
                .send_dht_rpc(&peer, &request)
                .map_err(|err| err.to_string());
            tx.send(response).unwrap();
        });
        tokio::time::timeout(Duration::from_secs(10), async {
            let mut pending_discovery = HashSet::new();
            loop {
                if let Ok(response) = rx.try_recv() {
                    return response.expect("libp2p DHT transport request should complete");
                }
                let event = server_swarm.select_next_some().await;
                let _ = handle_libp2p_dht_server_event(
                    server_node,
                    &mut server_swarm,
                    &mut pending_discovery,
                    event,
                );
            }
        })
        .await
        .unwrap()
    }

    fn spawn_dht_rpc_store_result_server(
        expected_requests: usize,
    ) -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            for _ in 0..expected_requests {
                let (mut stream, _) = listener.accept().unwrap();
                let mut raw = [0u8; 4096];
                let len = stream.read(&mut raw).unwrap();
                let request = String::from_utf8_lossy(&raw[..len]);
                assert!(request.starts_with("POST /dht/rpc HTTP/1.1"));
                assert!(request.contains("StoreRecord"));
                let request_id = extract_dht_rpc_request_id_from_http(&request);
                let body = serde_json::to_string(&DhtRpcResponse::StoreResult {
                    request_id,
                    stored: true,
                    inserted: true,
                })
                .unwrap();
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                stream.write_all(response.as_bytes()).unwrap();
            }
        });
        (format!("http://{addr}"), server)
    }

    fn spawn_dht_rpc_find_node_server(
        expected_requests: usize,
    ) -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            for _ in 0..expected_requests {
                let (mut stream, _) = listener.accept().unwrap();
                let mut raw = [0u8; 4096];
                let len = stream.read(&mut raw).unwrap();
                let request = String::from_utf8_lossy(&raw[..len]);
                assert!(request.starts_with("POST /dht/rpc HTTP/1.1"));
                let request_id = extract_dht_rpc_request_id_from_http(&request);
                let body = serde_json::to_string(&DhtRpcResponse::Nodes {
                    request_id,
                    nodes: Vec::new(),
                })
                .unwrap();
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                stream.write_all(response.as_bytes()).unwrap();
            }
        });
        (format!("http://{addr}"), server)
    }

    #[test]
    fn dht_replication_runner_sends_due_records_to_peers() {
        let (url, server) = spawn_dht_rpc_store_result_server(1);
        let mut node = NativeNode::new(NodeConfig::default());
        let now = current_unix_timestamp();
        let record = DhtRecord {
            key: DhtRecordKey::for_public_peer("replication-runner"),
            kind: DhtRecordKind::PublicPeer,
            value: "replicate-over-http".into(),
            created_at: now,
            expires_at: now + 120,
            republish_at: now,
        };
        assert!(node.dht_records.store(record));
        let stats = run_dht_replication(
            &mut node,
            &[SyncPeerConfig {
                url,
                token: None,
                peer_id: None,
            }],
            3,
        );
        assert_eq!(stats.records, 1);
        assert_eq!(stats.attempts, 1);
        assert_eq!(stats.successes, 1);
        assert_eq!(stats.failures, 0);
        server.join().unwrap();
    }

    #[test]
    fn dht_replication_runner_uses_closest_control_peer_when_peer_id_is_known() {
        let (url, server) = spawn_dht_rpc_store_result_server(1);
        let (identity, _) = Identity::create_with_passphrase("closest replication peer").unwrap();
        let announce = NodeConfig {
            peer_id: "closest-peer".into(),
            ..Default::default()
        }
        .create_announce(&identity)
        .unwrap();
        let mut node = NativeNode::new(NodeConfig::default());
        node.kademlia
            .insert_verified(announce.clone(), &identity.identity_public_key())
            .unwrap();
        let now = current_unix_timestamp();
        let record = DhtRecord {
            key: DhtRecordKey::for_public_peer("closest-record"),
            kind: DhtRecordKind::PublicPeer,
            value: "value".into(),
            created_at: now,
            expires_at: now.saturating_add(60),
            republish_at: now,
        };
        assert!(node.dht_records.store(record));
        let stats = run_dht_replication(
            &mut node,
            &[
                SyncPeerConfig {
                    url,
                    token: None,
                    peer_id: Some(announce.peer_id),
                },
                SyncPeerConfig {
                    url: "http://127.0.0.1:1".into(),
                    token: None,
                    peer_id: Some("not-a-target".into()),
                },
            ],
            1,
        );
        assert_eq!(stats.records, 1);
        assert_eq!(stats.attempts, 1);
        assert_eq!(stats.successes, 1);
        assert_eq!(stats.failures, 0);
        server.join().unwrap();
    }

    #[test]
    fn dht_routing_refresh_runner_sends_find_node_to_peers() {
        let (url, server) = spawn_dht_rpc_find_node_server(2);
        let mut node = NativeNode::new(NodeConfig::default());
        let stats = run_dht_routing_refresh(
            &mut node,
            &[SyncPeerConfig {
                url,
                token: None,
                peer_id: None,
            }],
            8,
            2,
        );
        assert_eq!(stats.targets, 2);
        assert_eq!(stats.attempts, 2);
        assert_eq!(stats.successes, 2);
        assert_eq!(stats.failures, 0);
        assert_eq!(stats.nodes_returned, 0);
        server.join().unwrap();
    }

    #[test]
    fn dht_find_value_skips_invalid_found_record() {
        let mut node = NativeNode::new(NodeConfig::default());
        let now = current_unix_timestamp();
        let key = DhtRecordKey::for_public_peer("invalid-found");
        let invalid_record = DhtRecord {
            key,
            kind: DhtRecordKind::PublicPeer,
            value: "not-a-public-peer-announce".into(),
            created_at: now,
            expires_at: now.saturating_add(60),
            republish_at: now.saturating_add(30),
        };
        let invalid_closer_records = (0..20)
            .map(|index| DhtRecord {
                key: DhtRecordKey::for_public_peer(&format!("invalid-closer-{index}")),
                kind: DhtRecordKind::PublicPeer,
                value: "not-a-public-peer-announce".into(),
                created_at: now,
                expires_at: now.saturating_add(60),
                republish_at: now.saturating_add(30),
            })
            .collect::<Vec<_>>();
        let peer = SyncPeerConfig {
            url: "transport://invalid-found".into(),
            token: None,
            peer_id: None,
        };
        let transport = FakeDhtTransport {
            responses: Mutex::new(vec![DhtRpcResponse::Value {
                request_id: "fake-find-value".into(),
                record: Some(invalid_record),
                closer_records: invalid_closer_records,
                closer_nodes: Vec::new(),
            }]),
            ..Default::default()
        };
        let stats = dht_find_value_with_transport(&mut node, &[peer], key, 8, 1, 1, &transport);
        assert_eq!(stats.attempts, 1);
        assert_eq!(stats.successes, 1);
        assert_eq!(stats.found_records, 0);
        assert_eq!(stats.closer_records, 0);
        assert_eq!(node.maintenance.dht_record_rejects.invalid_record, 9);
        assert!(node.dht_records.find_value(&key).is_none());
        assert!(
            node.dht_records
                .find_value(&DhtRecordKey::for_public_peer("invalid-closer-8"))
                .is_none()
        );
    }

    #[test]
    fn dht_routing_refresh_limits_oversized_node_responses() {
        let (identity, _) = Identity::create_with_passphrase("too many routing nodes").unwrap();
        let announce = NodeConfig {
            peer_id: "too-many-routing-nodes".into(),
            ..Default::default()
        }
        .create_announce(&identity)
        .unwrap();
        let node_id = lm_node::KademliaNodeId::from_peer_id(&announce.peer_id);
        let now = current_unix_timestamp();
        let noisy_nodes = (0..20)
            .map(|_| lm_node::RoutingPeer {
                node_id,
                announce: announce.clone(),
                identity_public_key: None,
                last_seen_at: now,
            })
            .collect::<Vec<_>>();
        let mut node = NativeNode::new(NodeConfig::default());
        let peer = SyncPeerConfig {
            url: "transport://too-many-nodes".into(),
            token: None,
            peer_id: None,
        };
        let transport = FakeDhtTransport {
            responses: Mutex::new(vec![DhtRpcResponse::Nodes {
                request_id: "fake-find-node".into(),
                nodes: noisy_nodes,
            }]),
            ..Default::default()
        };
        let stats =
            run_dht_routing_refresh_with_transport(&mut node, &[peer], 8, 1, None, &transport);
        assert_eq!(stats.attempts, 1);
        assert_eq!(stats.successes, 1);
        assert_eq!(stats.nodes_returned, 8);
        assert_eq!(stats.nodes_merged, 0);
    }

    #[test]
    fn dht_routing_refresh_filters_non_closer_and_duplicate_nodes() {
        let (identity, _) = Identity::create_with_passphrase("refresh filter identity").unwrap();
        let mut node = NativeNode::new(NodeConfig::default());
        let target = node.plan_dht_routing_refresh().targets[0];
        let seed_peer_id = (0..10_000)
            .map(|index| format!("refresh-filter-seed-{index}"))
            .find(|peer_id| {
                let seed_distance =
                    lm_node::KademliaNodeId::from_peer_id(peer_id).xor_distance(&target);
                (0..10_000).any(|candidate_index| {
                    lm_node::KademliaNodeId::from_peer_id(&format!(
                        "refresh-filter-closer-{candidate_index}"
                    ))
                    .xor_distance(&target)
                        < seed_distance
                }) && (0..10_000).any(|candidate_index| {
                    lm_node::KademliaNodeId::from_peer_id(&format!(
                        "refresh-filter-farther-{candidate_index}"
                    ))
                    .xor_distance(&target)
                        >= seed_distance
                })
            })
            .expect("test should find seed peer id");
        let seed_distance =
            lm_node::KademliaNodeId::from_peer_id(&seed_peer_id).xor_distance(&target);
        let closer_peer_id = (0..10_000)
            .map(|index| format!("refresh-filter-closer-{index}"))
            .find(|peer_id| {
                lm_node::KademliaNodeId::from_peer_id(peer_id).xor_distance(&target) < seed_distance
            })
            .expect("test should find closer peer id");
        let farther_peer_id = (0..10_000)
            .map(|index| format!("refresh-filter-farther-{index}"))
            .find(|peer_id| {
                lm_node::KademliaNodeId::from_peer_id(peer_id).xor_distance(&target)
                    >= seed_distance
            })
            .expect("test should find non-closer peer id");
        let make_peer = |peer_id: String| {
            let announce = NodeConfig {
                peer_id: peer_id.clone(),
                addresses: vec![format!("transport://{peer_id}")],
                ..Default::default()
            }
            .create_announce(&identity)
            .unwrap();
            lm_node::RoutingPeer {
                node_id: lm_node::KademliaNodeId::from_peer_id(&announce.peer_id),
                announce,
                identity_public_key: Some(BASE64.encode(identity.identity_public_key())),
                last_seen_at: current_unix_timestamp(),
            }
        };
        let closer_peer = make_peer(closer_peer_id.clone());
        let farther_peer = make_peer(farther_peer_id);
        let seed_peer = SyncPeerConfig {
            url: "transport://refresh-filter-seed".into(),
            token: None,
            peer_id: Some(seed_peer_id),
        };
        let transport = FakeDhtTransport {
            responses: Mutex::new(vec![DhtRpcResponse::Nodes {
                request_id: "refresh-filter".into(),
                nodes: vec![closer_peer.clone(), farther_peer.clone(), closer_peer],
            }]),
            ..Default::default()
        };
        let stats = run_dht_routing_refresh_with_transport(
            &mut node,
            std::slice::from_ref(&seed_peer),
            8,
            1,
            None,
            &transport,
        );
        assert_eq!(stats.attempts, 1);
        assert_eq!(stats.successes, 1);
        assert_eq!(stats.nodes_returned, 3);
        assert_eq!(stats.nodes_merged, 1);
        assert_eq!(stats.nodes_rejected_non_closer, 1);
        assert_eq!(stats.nodes_rejected_duplicate, 1);
        assert_eq!(node.kademlia.all_peers().len(), 1);
        assert_eq!(
            node.kademlia.all_peers()[0].announce.peer_id,
            closer_peer_id
        );

        let mut all_filtered_node = NativeNode::new(NodeConfig::default());
        let transport = FakeDhtTransport {
            responses: Mutex::new(vec![DhtRpcResponse::Nodes {
                request_id: "refresh-filter-all".into(),
                nodes: vec![farther_peer],
            }]),
            ..Default::default()
        };
        let stats = run_dht_routing_refresh_with_transport(
            &mut all_filtered_node,
            std::slice::from_ref(&seed_peer),
            8,
            1,
            None,
            &transport,
        );
        assert_eq!(stats.nodes_returned, 1);
        assert_eq!(stats.nodes_rejected_non_closer, 1);
        assert_eq!(stats.nodes_merged, 0);
        assert_eq!(
            all_filtered_node
                .sync_status
                .peers
                .get(&seed_peer.url)
                .unwrap()
                .consecutive_failures,
            1
        );
    }

    #[test]
    fn dht_find_value_iterates_to_verified_closer_nodes() {
        let (closer_identity, _) =
            Identity::create_with_passphrase("iterative closer peer").unwrap();
        let (target_user, _) =
            Identity::create_with_passphrase("iterative mailbox target").unwrap();
        let closer_announce = NodeConfig {
            peer_id: "iterative-closer-peer".into(),
            addresses: vec!["transport://iterative-closer".into()],
            ..Default::default()
        }
        .create_announce(&closer_identity)
        .unwrap();
        let closer_peer = lm_node::RoutingPeer {
            node_id: lm_node::KademliaNodeId::from_peer_id(&closer_announce.peer_id),
            announce: closer_announce,
            identity_public_key: Some(BASE64.encode(closer_identity.identity_public_key())),
            last_seen_at: current_unix_timestamp(),
        };
        let key = DhtRecordKey::for_mailbox_hint(target_user.user_id());
        let found_record =
            DhtRecord::mailbox_hint(target_user.user_id(), "mailbox://target".into(), 3600);
        let seed_peer = SyncPeerConfig {
            url: "transport://seed".into(),
            token: None,
            peer_id: None,
        };
        let transport = FakeDhtTransport {
            responses: Mutex::new(vec![
                DhtRpcResponse::Value {
                    request_id: "iterative-second".into(),
                    record: Some(found_record.clone()),
                    closer_records: Vec::new(),
                    closer_nodes: Vec::new(),
                },
                DhtRpcResponse::Value {
                    request_id: "iterative-first".into(),
                    record: None,
                    closer_records: Vec::new(),
                    closer_nodes: vec![closer_peer],
                },
            ]),
            ..Default::default()
        };
        let mut node = NativeNode::new(NodeConfig::default());
        let stats =
            dht_find_value_with_transport(&mut node, &[seed_peer], key, 8, 2, 1, &transport);
        assert_eq!(stats.attempts, 2);
        assert_eq!(stats.successes, 2);
        assert_eq!(stats.found_records, 1);
        assert_eq!(stats.closer_nodes_returned, 1);
        assert_eq!(stats.closer_nodes_merged, 1);
        assert_eq!(
            node.dht_records.find_value(&key).unwrap().value,
            found_record.value
        );
        let requests = transport.requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].0, "transport://seed");
        assert_eq!(requests[1].0, "transport://iterative-closer");
    }

    #[test]
    fn dht_find_value_rejects_duplicate_closer_nodes_before_queueing() {
        let (identity, _) = Identity::create_with_passphrase("duplicate closer identity").unwrap();
        let (target_user, _) = Identity::create_with_passphrase("duplicate closer target").unwrap();
        let key = DhtRecordKey::for_mailbox_hint(target_user.user_id());
        let closer_announce = NodeConfig {
            peer_id: "duplicate-closer-peer".into(),
            addresses: vec!["transport://duplicate-closer-peer".into()],
            ..Default::default()
        }
        .create_announce(&identity)
        .unwrap();
        let closer_peer = lm_node::RoutingPeer {
            node_id: lm_node::KademliaNodeId::from_peer_id(&closer_announce.peer_id),
            announce: closer_announce,
            identity_public_key: Some(BASE64.encode(identity.identity_public_key())),
            last_seen_at: current_unix_timestamp(),
        };
        let seed_peer = SyncPeerConfig {
            url: "transport://duplicate-seed".into(),
            token: None,
            // Leave peer_id unknown so this test focuses on duplicate queue
            // filtering rather than the progress filter.
            peer_id: None,
        };
        let transport = FakeDhtTransport {
            responses: Mutex::new(vec![
                DhtRpcResponse::Value {
                    request_id: "duplicate-second".into(),
                    record: None,
                    closer_records: Vec::new(),
                    closer_nodes: Vec::new(),
                },
                DhtRpcResponse::Value {
                    request_id: "duplicate-first".into(),
                    record: None,
                    closer_records: Vec::new(),
                    closer_nodes: vec![closer_peer.clone(), closer_peer],
                },
            ]),
            ..Default::default()
        };
        let mut node = NativeNode::new(NodeConfig::default());
        let stats =
            dht_find_value_with_transport(&mut node, &[seed_peer], key, 8, 3, 1, &transport);
        assert_eq!(stats.attempts, 2);
        assert_eq!(stats.closer_nodes_returned, 2);
        assert_eq!(stats.closer_nodes_merged, 1);
        assert_eq!(stats.closer_nodes_rejected_duplicate, 1);
        let requests = transport.requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[1].0, "transport://duplicate-closer-peer");
    }

    #[test]
    fn dht_find_value_rejects_non_closer_nodes_when_responder_identity_is_known() {
        let (identity, _) = Identity::create_with_passphrase("non closer peer identity").unwrap();
        let (target_user, _) = Identity::create_with_passphrase("non closer target user").unwrap();
        let key = DhtRecordKey::for_mailbox_hint(target_user.user_id());
        let target_node_id = key.to_node_id();
        let seed_peer_id = "known-seed-for-non-closer";
        let seed_distance =
            lm_node::KademliaNodeId::from_peer_id(seed_peer_id).xor_distance(&target_node_id);
        let far_peer_id = (0..512)
            .map(|index| format!("non-closer-candidate-{index}"))
            .find(|peer_id| {
                lm_node::KademliaNodeId::from_peer_id(peer_id).xor_distance(&target_node_id)
                    >= seed_distance
            })
            .expect("test should find a non-closer candidate");
        let far_announce = NodeConfig {
            peer_id: far_peer_id.clone(),
            addresses: vec![format!("transport://{far_peer_id}")],
            ..Default::default()
        }
        .create_announce(&identity)
        .unwrap();
        let far_peer = lm_node::RoutingPeer {
            node_id: lm_node::KademliaNodeId::from_peer_id(&far_announce.peer_id),
            announce: far_announce,
            identity_public_key: Some(BASE64.encode(identity.identity_public_key())),
            last_seen_at: current_unix_timestamp(),
        };
        let seed_peer = SyncPeerConfig {
            url: "transport://known-seed".into(),
            token: None,
            peer_id: Some(seed_peer_id.into()),
        };
        let transport = FakeDhtTransport {
            responses: Mutex::new(vec![DhtRpcResponse::Value {
                request_id: "non-closer-first".into(),
                record: None,
                closer_records: Vec::new(),
                closer_nodes: vec![far_peer],
            }]),
            ..Default::default()
        };
        let mut node = NativeNode::new(NodeConfig::default());
        let stats =
            dht_find_value_with_transport(&mut node, &[seed_peer], key, 8, 2, 1, &transport);
        assert_eq!(stats.attempts, 1);
        assert_eq!(stats.closer_nodes_returned, 1);
        assert_eq!(stats.closer_nodes_rejected_non_closer, 1);
        assert_eq!(stats.closer_nodes_merged, 0);
        assert!(node.kademlia.all_peers().is_empty());
        assert_eq!(
            node.sync_status
                .peers
                .get("transport://known-seed")
                .unwrap()
                .consecutive_failures,
            1
        );
    }

    #[test]
    fn dht_find_value_queries_closer_nodes_by_distance_not_response_order() {
        let (identity, _) = Identity::create_with_passphrase("ordered closer peers").unwrap();
        let (target_user, _) = Identity::create_with_passphrase("ordered target user").unwrap();
        let key = DhtRecordKey::for_mailbox_hint(target_user.user_id());
        let target_node_id = key.to_node_id();
        let mut peers = ["ordered-peer-a", "ordered-peer-b"]
            .into_iter()
            .map(|peer_id| {
                let announce = NodeConfig {
                    peer_id: peer_id.into(),
                    addresses: vec![format!("transport://{peer_id}")],
                    ..Default::default()
                }
                .create_announce(&identity)
                .unwrap();
                lm_node::RoutingPeer {
                    node_id: lm_node::KademliaNodeId::from_peer_id(&announce.peer_id),
                    announce,
                    identity_public_key: Some(BASE64.encode(identity.identity_public_key())),
                    last_seen_at: current_unix_timestamp(),
                }
            })
            .collect::<Vec<_>>();
        peers.sort_by_key(|peer| peer.node_id.xor_distance(&target_node_id));
        let nearer = peers[0].clone();
        let farther = peers[1].clone();
        let found_record = DhtRecord::mailbox_hint(
            target_user.user_id(),
            "mailbox://ordered-target".into(),
            3600,
        );
        let seed_peer = SyncPeerConfig {
            url: "transport://ordered-seed".into(),
            token: None,
            peer_id: None,
        };
        let transport = FakeDhtTransport {
            responses: Mutex::new(vec![
                DhtRpcResponse::Value {
                    request_id: "ordered-second".into(),
                    record: Some(found_record.clone()),
                    closer_records: Vec::new(),
                    closer_nodes: Vec::new(),
                },
                DhtRpcResponse::Value {
                    request_id: "ordered-first".into(),
                    record: None,
                    closer_records: Vec::new(),
                    // Deliberately return the farther peer first. The runner
                    // should sort by target distance before spending its final
                    // query budget.
                    closer_nodes: vec![farther, nearer.clone()],
                },
            ]),
            ..Default::default()
        };
        let mut node = NativeNode::new(NodeConfig::default());
        let stats =
            dht_find_value_with_transport(&mut node, &[seed_peer], key, 8, 2, 1, &transport);
        assert_eq!(stats.attempts, 2);
        assert_eq!(stats.found_records, 1);
        assert_eq!(
            node.dht_records.find_value(&key).unwrap().value,
            found_record.value
        );
        let requests = transport.requests.lock().unwrap();
        assert_eq!(
            requests[1].0,
            format!("transport://{}", nearer.announce.peer_id)
        );
    }

    #[test]
    fn control_dht_maintenance_run_combines_replication_and_refresh() {
        let mut node = NativeNode::new(NodeConfig::default());
        let response = handle_control_dht_maintenance_run(
            &mut node,
            &[],
            DhtRunnerConfig::default(),
            "/dht/maintenance?factor=2&limit=4&max_targets=2",
            None,
        );
        assert_eq!(response.status, 200, "{}", response.body);
        let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(body["peers"], 0);
        assert_eq!(body["replication"]["attempts"], 0);
        assert_eq!(body["routing_refresh"]["attempts"], 0);

        for failure in 0..DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES {
            node.sync_status.record_failure(
                "http://maintenance-quarantined",
                format!("failure-{failure}"),
            );
        }
        let peers = vec![SyncPeerConfig {
            url: "http://maintenance-quarantined".into(),
            token: None,
            peer_id: None,
        }];
        let response = handle_control_dht_maintenance_run(
            &mut node,
            &peers,
            DhtRunnerConfig::default(),
            "/dht/maintenance?factor=2&limit=4&max_targets=2",
            None,
        );
        assert_eq!(response.status, 200, "{}", response.body);
        let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(body["peers"], 0);
        assert_eq!(body["replication"]["peers_quarantined"], 1);
        assert_eq!(body["routing_refresh"]["peers_quarantined"], 1);
    }

    #[test]
    fn control_dht_replication_run_reports_empty_and_quarantine_stats() {
        let mut node = NativeNode::new(NodeConfig::default());
        let empty = handle_control_dht_replication_run(
            &mut node,
            &[],
            DhtRunnerConfig::default(),
            "/dht/replicate?factor=2",
            None,
        );
        assert_eq!(empty.status, 200, "{}", empty.body);
        let body: serde_json::Value = serde_json::from_str(&empty.body).unwrap();
        assert_eq!(body["peers"], 0);
        assert_eq!(body["stats"]["records"], 0);
        assert_eq!(body["stats"]["attempts"], 0);

        let now = current_unix_timestamp();
        assert!(node.dht_records.store(DhtRecord {
            key: DhtRecordKey::for_public_peer("manual-replication-record"),
            kind: DhtRecordKind::PublicPeer,
            value: "manual-replication-value".into(),
            created_at: now,
            expires_at: now.saturating_add(60),
            republish_at: now,
        }));
        for failure in 0..DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES {
            node.sync_status.record_failure(
                "http://replication-quarantined",
                format!("failure-{failure}"),
            );
        }
        let peers = vec![SyncPeerConfig {
            url: "http://replication-quarantined".into(),
            token: None,
            peer_id: None,
        }];
        let quarantined = handle_control_dht_replication_run(
            &mut node,
            &peers,
            DhtRunnerConfig::default(),
            "/dht/replicate?factor=2",
            None,
        );
        assert_eq!(quarantined.status, 200, "{}", quarantined.body);
        let body: serde_json::Value = serde_json::from_str(&quarantined.body).unwrap();
        assert_eq!(body["peers"], 0);
        assert_eq!(body["records"], 1);
        assert_eq!(body["stats"]["peers_quarantined"], 1);
        assert_eq!(body["stats"]["attempts"], 0);
    }

    #[test]
    fn control_dht_routing_refresh_run_reports_empty_and_quarantine_stats() {
        let mut node = NativeNode::new(NodeConfig::default());
        let empty = handle_control_dht_routing_refresh_run(
            &mut node,
            &[],
            DhtRunnerConfig::default(),
            "/dht/routing-refresh?limit=4&max_targets=2",
            None,
        );
        assert_eq!(empty.status, 200, "{}", empty.body);
        let body: serde_json::Value = serde_json::from_str(&empty.body).unwrap();
        assert_eq!(body["peers"], 0);
        assert_eq!(body["stats"]["targets"], 0);
        assert_eq!(body["stats"]["attempts"], 0);

        for failure in 0..DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES {
            node.sync_status
                .record_failure("http://refresh-quarantined", format!("failure-{failure}"));
        }
        let peers = vec![SyncPeerConfig {
            url: "http://refresh-quarantined".into(),
            token: None,
            peer_id: None,
        }];
        let quarantined = handle_control_dht_routing_refresh_run(
            &mut node,
            &peers,
            DhtRunnerConfig::default(),
            "/dht/routing-refresh?limit=4&max_targets=2",
            None,
        );
        assert_eq!(quarantined.status, 200, "{}", quarantined.body);
        let body: serde_json::Value = serde_json::from_str(&quarantined.body).unwrap();
        assert_eq!(body["peers"], 0);
        assert_eq!(body["stats"]["peers_quarantined"], 1);
    }

    #[test]
    fn control_dht_find_value_run_requires_key_and_reports_empty_peer_stats() {
        let mut node = NativeNode::new(NodeConfig::default());
        let missing = handle_control_dht_find_value_run(
            &mut node,
            &[],
            DhtRunnerConfig::default(),
            "/dht/find-value",
            None,
        );
        assert_eq!(missing.status, 400);
        assert!(missing.body.contains("missing key"));

        let key = DhtRecordKey::for_public_peer("control-find-run-missing");
        let ok = handle_control_dht_find_value_run(
            &mut node,
            &[],
            DhtRunnerConfig::default(),
            &format!("/dht/find-value?key={key}&limit=8&max_peers=2"),
            None,
        );
        assert_eq!(ok.status, 200, "{}", ok.body);
        let body: serde_json::Value = serde_json::from_str(&ok.body).unwrap();
        assert_eq!(body["key"], key.to_hex());
        assert_eq!(body["found"], false);
        assert_eq!(body["stats"]["attempts"], 0);
        assert_eq!(body["stats"]["alpha"], 3);
        assert_eq!(body["stats"]["exhausted"], true);

        let (target_identity, _) =
            Identity::create_with_passphrase("find value query key").unwrap();
        let derived = handle_control_dht_find_value_run(
            &mut node,
            &[],
            DhtRunnerConfig::default(),
            &format!(
                "/dht/find-value?kind=mailbox-hint&value={}&limit=8&max_peers=2",
                target_identity.user_id()
            ),
            None,
        );
        assert_eq!(derived.status, 200, "{}", derived.body);
        let body: serde_json::Value = serde_json::from_str(&derived.body).unwrap();
        assert_eq!(
            body["key"],
            DhtRecordKey::for_mailbox_hint(target_identity.user_id()).to_hex()
        );
        assert_eq!(body["found"], false);

        for failure in 0..DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES {
            node.sync_status.record_failure(
                "http://quarantined-control-peer",
                format!("failure-{failure}"),
            );
        }
        let peers = vec![SyncPeerConfig {
            url: "http://quarantined-control-peer".into(),
            token: None,
            peer_id: None,
        }];
        let quarantined = handle_control_dht_find_value_run(
            &mut node,
            &peers,
            DhtRunnerConfig::default(),
            &format!("/dht/find-value?key={key}&limit=8&max_peers=2"),
            None,
        );
        assert_eq!(quarantined.status, 200, "{}", quarantined.body);
        let body: serde_json::Value = serde_json::from_str(&quarantined.body).unwrap();
        assert_eq!(body["stats"]["attempts"], 0);
        assert_eq!(body["stats"]["peers_quarantined"], 1);
    }

    #[test]
    fn dht_find_value_stops_after_valid_record_is_found() {
        let (identity, _) = Identity::create_with_passphrase("stop after found closer").unwrap();
        let (target_user, _) = Identity::create_with_passphrase("stop after found target").unwrap();
        let key = DhtRecordKey::for_mailbox_hint(target_user.user_id());
        let announce = NodeConfig {
            peer_id: "stop-after-found-closer".into(),
            addresses: vec!["transport://should-not-be-queried".into()],
            ..Default::default()
        }
        .create_announce(&identity)
        .unwrap();
        let closer_peer = lm_node::RoutingPeer {
            node_id: lm_node::KademliaNodeId::from_peer_id(&announce.peer_id),
            announce,
            identity_public_key: Some(BASE64.encode(identity.identity_public_key())),
            last_seen_at: current_unix_timestamp(),
        };
        let found_record = DhtRecord::mailbox_hint(
            target_user.user_id(),
            "mailbox://stop-after-found".into(),
            3600,
        );
        let seed_peer = SyncPeerConfig {
            url: "transport://stop-seed".into(),
            token: None,
            peer_id: None,
        };
        let transport = FakeDhtTransport {
            responses: Mutex::new(vec![
                DhtRpcResponse::Value {
                    request_id: "unused-second".into(),
                    record: None,
                    closer_records: Vec::new(),
                    closer_nodes: Vec::new(),
                },
                DhtRpcResponse::Value {
                    request_id: "found-first".into(),
                    record: Some(found_record.clone()),
                    closer_records: Vec::new(),
                    closer_nodes: vec![closer_peer],
                },
            ]),
            ..Default::default()
        };
        let mut node = NativeNode::new(NodeConfig::default());
        let stats =
            dht_find_value_with_transport(&mut node, &[seed_peer], key, 8, 2, 1, &transport);
        assert_eq!(stats.attempts, 1);
        assert_eq!(stats.successes, 1);
        assert_eq!(stats.found_records, 1);
        assert_eq!(stats.closer_nodes_returned, 0);
        assert_eq!(
            node.dht_records.find_value(&key).unwrap().value,
            found_record.value
        );
        assert_eq!(transport.requests.lock().unwrap().len(), 1);
    }

    struct ConcurrentProbeDhtTransport {
        barrier: Barrier,
        in_flight: AtomicUsize,
        max_in_flight: AtomicUsize,
        requests: Mutex<Vec<String>>,
    }

    impl DhtTransport for ConcurrentProbeDhtTransport {
        fn send_dht_rpc(
            &self,
            peer: &SyncPeerConfig,
            _request: &DhtRpcRequest,
        ) -> Result<DhtRpcResponse, Box<dyn std::error::Error>> {
            self.requests.lock().unwrap().push(peer.url.clone());
            let current = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            let mut observed = self.max_in_flight.load(Ordering::SeqCst);
            while current > observed {
                match self.max_in_flight.compare_exchange(
                    observed,
                    current,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ) {
                    Ok(_) => break,
                    Err(next) => observed = next,
                }
            }
            self.barrier.wait();
            self.in_flight.fetch_sub(1, Ordering::SeqCst);
            Ok(DhtRpcResponse::Value {
                request_id: "concurrent-probe".into(),
                record: None,
                closer_records: Vec::new(),
                closer_nodes: Vec::new(),
            })
        }
    }

    #[test]
    fn dht_find_value_uses_alpha_parallelism_within_round() {
        let (target_identity, _) = Identity::create_with_passphrase("alpha target").unwrap();
        let key = DhtRecordKey::for_mailbox_hint(target_identity.user_id());
        let peers = (0..3)
            .map(|index| SyncPeerConfig {
                url: format!("transport://alpha-{index}"),
                token: None,
                peer_id: Some(format!("alpha-peer-{index}")),
            })
            .collect::<Vec<_>>();
        let transport = ConcurrentProbeDhtTransport {
            barrier: Barrier::new(3),
            in_flight: AtomicUsize::new(0),
            max_in_flight: AtomicUsize::new(0),
            requests: Mutex::new(Vec::new()),
        };
        let mut node = NativeNode::new(NodeConfig::default());
        let stats = dht_find_value_with_transport(&mut node, &peers, key, 8, 3, 3, &transport);
        assert_eq!(stats.attempts, 3);
        assert_eq!(stats.query_rounds, 1);
        assert_eq!(stats.successes, 3);
        assert!(stats.exhausted);
        assert_eq!(transport.requests.lock().unwrap().len(), 3);
        assert!(
            transport.max_in_flight.load(Ordering::SeqCst) > 1,
            "alpha=3 should issue more than one FindValue request concurrently"
        );
    }

    #[test]
    fn dht_find_value_updates_peer_health_and_quarantine() {
        let (target_identity, _) = Identity::create_with_passphrase("health target").unwrap();
        let key = DhtRecordKey::for_mailbox_hint(target_identity.user_id());
        let peer = SyncPeerConfig {
            url: "transport://health-peer".into(),
            token: None,
            peer_id: Some("health-peer".into()),
        };
        let mut failure_responses = Vec::new();
        for index in 0..DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES {
            failure_responses.push(DhtRpcResponse::Error {
                request_id: format!("health-failure-{index}"),
                message: format!("synthetic failure {index}"),
            });
        }
        failure_responses.reverse();
        let transport = FakeDhtTransport {
            responses: Mutex::new(failure_responses),
            ..Default::default()
        };
        let mut node = NativeNode::new(NodeConfig::default());
        for _ in 0..DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES {
            let stats = dht_find_value_with_transport(
                &mut node,
                std::slice::from_ref(&peer),
                key,
                8,
                1,
                1,
                &transport,
            );
            assert_eq!(stats.failures, 1);
        }
        let status = node.sync_status.peers.get(&peer.url).unwrap();
        assert_eq!(
            status.consecutive_failures,
            DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES
        );
        assert!(status.next_attempt_at.unwrap() > current_unix_timestamp());
        let (eligible, quarantined) = dht_runner_peer_configs_with_quarantine_count(
            &node,
            std::slice::from_ref(&peer),
            DhtTransportKind::HttpControl,
            DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES,
        );
        assert!(eligible.is_empty());
        assert_eq!(quarantined, 1);

        let transport = FakeDhtTransport {
            responses: Mutex::new(vec![DhtRpcResponse::Value {
                request_id: "health-success".into(),
                record: None,
                closer_records: Vec::new(),
                closer_nodes: Vec::new(),
            }]),
            ..Default::default()
        };
        let stats = dht_find_value_with_transport(
            &mut node,
            std::slice::from_ref(&peer),
            key,
            8,
            1,
            1,
            &transport,
        );
        assert_eq!(stats.successes, 1);
        let status = node.sync_status.peers.get(&peer.url).unwrap();
        assert_eq!(status.consecutive_failures, 0);
        assert!(status.last_error.is_none());
    }

    #[test]
    fn dht_runners_use_transport_abstraction() {
        let (closer_identity, _) =
            Identity::create_with_passphrase("transport closer peer").unwrap();
        let closer_announce = NodeConfig {
            peer_id: "transport-closer".into(),
            ..Default::default()
        }
        .create_announce(&closer_identity)
        .unwrap();
        let mut node = NativeNode::new(NodeConfig::default());
        let now = current_unix_timestamp();
        let record = DhtRecord {
            key: DhtRecordKey::for_public_peer("transport-record"),
            kind: DhtRecordKind::PublicPeer,
            value: "value".into(),
            created_at: now,
            expires_at: now.saturating_add(60),
            republish_at: now,
        };
        assert!(node.dht_records.store(record.clone()));
        let found_announce = NodeConfig {
            peer_id: "transport-found".into(),
            ..Default::default()
        }
        .create_announce(&closer_identity)
        .unwrap();
        let closer_record_announce = NodeConfig {
            peer_id: "transport-closer-record".into(),
            ..Default::default()
        }
        .create_announce(&closer_identity)
        .unwrap();
        let found_record = DhtRecord::public_peer(
            &found_announce,
            found_announce.to_export_text().unwrap(),
            60,
        );
        let closer_record = DhtRecord::public_peer(
            &closer_record_announce,
            closer_record_announce.to_export_text().unwrap(),
            60,
        );
        let closer_peer = lm_node::RoutingPeer {
            node_id: lm_node::KademliaNodeId::from_peer_id(&closer_announce.peer_id),
            announce: closer_announce,
            identity_public_key: Some(BASE64.encode(closer_identity.identity_public_key())),
            last_seen_at: now,
        };
        let peer = SyncPeerConfig {
            url: "transport://peer-a".into(),
            token: None,
            peer_id: None,
        };
        let transport = FakeDhtTransport {
            responses: Mutex::new(vec![
                DhtRpcResponse::Nodes {
                    request_id: "fake-refresh".into(),
                    nodes: Vec::new(),
                },
                DhtRpcResponse::Value {
                    request_id: "fake-find-value".into(),
                    record: Some(found_record.clone()),
                    closer_records: vec![closer_record.clone()],
                    closer_nodes: vec![closer_peer],
                },
                DhtRpcResponse::StoreResult {
                    request_id: "fake-store".into(),
                    stored: true,
                    inserted: true,
                },
            ]),
            ..Default::default()
        };

        let replication = run_dht_replication_with_transport(
            &mut node,
            std::slice::from_ref(&peer),
            1,
            None,
            &transport,
        );
        assert_eq!(replication.attempts, 1);
        assert_eq!(replication.successes, 1);

        let find_value = dht_find_value_with_transport(
            &mut node,
            std::slice::from_ref(&peer),
            DhtRecordKey::for_public_peer("transport-found"),
            8,
            1,
            1,
            &transport,
        );
        assert_eq!(find_value.attempts, 1);
        assert_eq!(find_value.successes, 1);
        assert_eq!(find_value.found_records, 1);
        assert_eq!(find_value.closer_records, 0);
        assert_eq!(find_value.closer_nodes_returned, 0);
        assert_eq!(find_value.closer_nodes_merged, 0);
        assert_eq!(
            node.dht_records
                .find_value(&DhtRecordKey::for_public_peer("transport-found"))
                .unwrap()
                .value,
            found_record.value
        );
        let refresh = run_dht_routing_refresh_with_transport(
            &mut node,
            std::slice::from_ref(&peer),
            8,
            1,
            None,
            &transport,
        );
        assert_eq!(refresh.attempts, 1);
        assert_eq!(refresh.successes, 1);

        let requests = transport.requests.lock().unwrap();
        assert_eq!(requests.len(), 3);
        assert!(matches!(requests[0].1, DhtRpcRequest::StoreRecord { .. }));
        assert!(matches!(requests[1].1, DhtRpcRequest::FindValue { .. }));
        assert!(matches!(requests[2].1, DhtRpcRequest::FindNode { .. }));
    }

    #[test]
    fn dht_runner_libp2p_transport_uses_discovered_routing_peers() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(async {
            let mut swarm = libp2p_dht_swarm().unwrap();
            let libp2p_peer_id = *swarm.local_peer_id();
            swarm
                .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
                .unwrap();
            let address = loop {
                if let SwarmEvent::NewListenAddr { address, .. } = swarm.select_next_some().await {
                    break address;
                }
            };
            let (identity, _) =
                Identity::create_with_passphrase("libp2p runner discovered peer").unwrap();
            let announce = NodeConfig {
                peer_id: libp2p_peer_id.to_string(),
                addresses: vec![address.to_string()],
                ..Default::default()
            }
            .create_announce(&identity)
            .unwrap();
            let mut node = NativeNode::new(NodeConfig::default());
            node.kademlia
                .insert_verified(announce, &identity.identity_public_key())
                .unwrap();
            let configured = vec![SyncPeerConfig {
                url: "libp2p:///ip4/127.0.0.1/tcp/9999".into(),
                token: None,
                peer_id: Some(libp2p_peer_id.to_string()),
            }];
            let http_peers = dht_runner_peer_configs(
                &node,
                &configured,
                DhtTransportKind::HttpControl,
                DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES,
            );
            assert_eq!(http_peers, configured);
            let libp2p_peers = dht_runner_peer_configs(
                &node,
                &configured,
                DhtTransportKind::Libp2p,
                DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES,
            );
            assert_eq!(libp2p_peers.len(), 2);
            assert!(
                libp2p_peers
                    .iter()
                    .any(|peer| peer.url == format!("libp2p://{address}"))
            );
            assert!(
                libp2p_peers
                    .iter()
                    .all(|peer| peer.peer_id.as_deref() == Some(&libp2p_peer_id.to_string()))
            );
        });
    }

    #[test]
    fn dht_runner_http_transport_uses_discovered_http_routing_peers() {
        let (identity, _) =
            Identity::create_with_passphrase("http runner discovered peer").unwrap();
        let announce = NodeConfig {
            peer_id: "http-discovered-peer".into(),
            addresses: vec!["http://127.0.0.1:9999".into()],
            ..Default::default()
        }
        .create_announce(&identity)
        .unwrap();
        let mut node = NativeNode::new(NodeConfig::default());
        node.kademlia
            .insert_verified(announce.clone(), &identity.identity_public_key())
            .unwrap();
        let configured = vec![SyncPeerConfig {
            url: "http://127.0.0.1:8787".into(),
            token: Some("shared-control-token".into()),
            peer_id: None,
        }];
        let peers = dht_runner_peer_configs(
            &node,
            &configured,
            DhtTransportKind::HttpControl,
            DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES,
        );
        assert_eq!(peers.len(), 2);
        let discovered = peers
            .iter()
            .find(|peer| peer.url == "http://127.0.0.1:9999")
            .unwrap();
        assert_eq!(discovered.peer_id.as_deref(), Some("http-discovered-peer"));
        assert_eq!(discovered.token, None);
    }

    #[test]
    fn dht_runner_peer_configs_prioritize_healthier_configured_peers() {
        let mut node = NativeNode::new(NodeConfig::default());
        node.sync_status
            .record_failure("http://bad-peer", "first failure");
        node.sync_status
            .record_failure("http://bad-peer", "second failure");
        node.sync_status
            .record_failure("http://flaky-peer", "one failure");
        let configured = vec![
            SyncPeerConfig {
                url: "http://bad-peer".into(),
                token: None,
                peer_id: None,
            },
            SyncPeerConfig {
                url: "http://good-peer".into(),
                token: None,
                peer_id: None,
            },
            SyncPeerConfig {
                url: "http://flaky-peer".into(),
                token: None,
                peer_id: None,
            },
        ];
        let peers = dht_runner_peer_configs(
            &node,
            &configured,
            DhtTransportKind::HttpControl,
            DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES,
        );
        assert_eq!(peers[0].url, "http://good-peer");
        assert_eq!(peers[1].url, "http://flaky-peer");
        assert_eq!(peers[2].url, "http://bad-peer");
    }

    #[test]
    fn dht_runner_peer_configs_skip_quarantined_configured_peers() {
        let mut node = NativeNode::new(NodeConfig::default());
        for failure in 0..DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES {
            node.sync_status
                .record_failure("http://quarantined-peer", format!("failure-{failure}"));
        }
        let configured = vec![
            SyncPeerConfig {
                url: "http://quarantined-peer".into(),
                token: None,
                peer_id: None,
            },
            SyncPeerConfig {
                url: "http://healthy-peer".into(),
                token: None,
                peer_id: None,
            },
        ];
        let peers = dht_runner_peer_configs(
            &node,
            &configured,
            DhtTransportKind::HttpControl,
            DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES,
        );
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].url, "http://healthy-peer");

        node.sync_status.record_next_attempt(
            "http://quarantined-peer",
            current_unix_timestamp().saturating_sub(1),
        );
        let peers = dht_runner_peer_configs(
            &node,
            &configured,
            DhtTransportKind::HttpControl,
            DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES,
        );
        assert_eq!(peers.len(), 2);
        assert_eq!(peers[0].url, "http://healthy-peer");
        assert_eq!(peers[1].url, "http://quarantined-peer");
    }

    #[test]
    fn libp2p_dht_rpc_behaviour_uses_lm_protocol() {
        let _behaviour = libp2p_dht_rpc_behaviour();
        assert_eq!(LIBP2P_DHT_RPC_PROTOCOL, "/lm-talk/dht-rpc/1");
    }

    #[test]
    fn libp2p_dht_swarm_builds_with_tcp_transport() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(async {
            let mut swarm = libp2p_dht_swarm().unwrap();
            swarm
                .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
                .unwrap();
        });
    }

    #[test]
    fn libp2p_dht_transport_parses_peer_config() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(async {
            let mut swarm = libp2p_dht_swarm().unwrap();
            let peer_id = *swarm.local_peer_id();
            swarm
                .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
                .unwrap();
            let address = loop {
                if let SwarmEvent::NewListenAddr { address, .. } = swarm.select_next_some().await {
                    break address;
                }
            };
            let peer = SyncPeerConfig {
                url: format!("libp2p://{address}"),
                token: Some("ignored-for-libp2p".into()),
                peer_id: Some(peer_id.to_string()),
            };
            let (parsed_peer_id, parsed_address) = parse_libp2p_dht_peer(&peer).unwrap();
            assert_eq!(parsed_peer_id, peer_id);
            assert_eq!(parsed_address, address);
        });
    }

    #[test]
    fn libp2p_bootstrap_peers_parse_multiaddr_and_peer_id() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(async {
            let mut first = libp2p_dht_swarm().unwrap();
            let mut second = libp2p_dht_swarm().unwrap();
            let first_peer = *first.local_peer_id();
            let second_peer = *second.local_peer_id();
            first
                .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
                .unwrap();
            second
                .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
                .unwrap();
            let first_addr = loop {
                if let SwarmEvent::NewListenAddr { address, .. } = first.select_next_some().await {
                    break address;
                }
            };
            let second_addr = loop {
                if let SwarmEvent::NewListenAddr { address, .. } = second.select_next_some().await {
                    break address;
                }
            };
            let peers = parse_libp2p_bootstrap_peers(&format!(
                "libp2p://{first_addr}|{first_peer},{second_addr}|{second_peer}"
            ))
            .unwrap();
            assert_eq!(peers.len(), 2);
            assert_eq!(peers[0].peer_id, first_peer);
            assert_eq!(peers[0].address, first_addr);
            assert_eq!(peers[1].peer_id, second_peer);
            assert_eq!(peers[1].address, second_addr);
            assert!(parse_libp2p_bootstrap_peers(&format!("{first_addr}")).is_err());
        });
    }

    #[test]
    fn libp2p_bootstrap_peers_are_dialed_for_discovery_seed() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(async {
            let mut seed = libp2p_dht_swarm().unwrap();
            let mut joining = libp2p_dht_swarm().unwrap();
            let seed_peer = *seed.local_peer_id();
            seed.listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
                .unwrap();
            let seed_addr = loop {
                if let SwarmEvent::NewListenAddr { address, .. } = seed.select_next_some().await {
                    break address;
                }
            };
            dial_libp2p_bootstrap_peers(
                &mut joining,
                &[super::Libp2pBootstrapPeer {
                    peer_id: seed_peer,
                    address: seed_addr,
                }],
            )
            .unwrap();
            let connected = tokio::time::timeout(Duration::from_secs(10), async {
                loop {
                    futures::select! {
                        event = seed.select_next_some() => {
                            if let SwarmEvent::ConnectionEstablished { peer_id, .. } = event {
                                break peer_id == *joining.local_peer_id();
                            }
                        }
                        event = joining.select_next_some() => {
                            if let SwarmEvent::ConnectionEstablished { peer_id, .. } = event {
                                break peer_id == seed_peer;
                            }
                        }
                    }
                }
            })
            .await
            .unwrap();
            assert!(connected);
        });
    }

    #[test]
    fn libp2p_discovery_merges_nodes_from_bootstrap_find_node() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(async {
            let mut seed_swarm = libp2p_dht_swarm().unwrap();
            let mut joining_swarm = libp2p_dht_swarm().unwrap();
            let seed_peer = *seed_swarm.local_peer_id();
            seed_swarm
                .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
                .unwrap();
            let seed_addr = loop {
                if let SwarmEvent::NewListenAddr { address, .. } =
                    seed_swarm.select_next_some().await
                {
                    break address;
                }
            };

            let (discovered_identity, _) =
                Identity::create_with_passphrase("libp2p discovered routing peer").unwrap();
            let discovered_announce = NodeConfig {
                peer_id: "libp2p-discovered-peer".into(),
                ..Default::default()
            }
            .create_announce(&discovered_identity)
            .unwrap();
            let mut seed_node = NativeNode::new(NodeConfig::default());
            seed_node
                .kademlia
                .insert_verified(
                    discovered_announce.clone(),
                    &discovered_identity.identity_public_key(),
                )
                .unwrap();
            let mut joining_node = NativeNode::new(NodeConfig::default());
            dial_libp2p_bootstrap_peers(
                &mut joining_swarm,
                &[super::Libp2pBootstrapPeer {
                    peer_id: seed_peer,
                    address: seed_addr,
                }],
            )
            .unwrap();

            let mut seed_pending = HashSet::new();
            let mut joining_pending = HashSet::new();
            tokio::time::timeout(Duration::from_secs(10), async {
                loop {
                    futures::select! {
                        event = seed_swarm.select_next_some() => {
                            let _ = handle_libp2p_dht_server_event(
                                &mut seed_node,
                                &mut seed_swarm,
                                &mut seed_pending,
                                event,
                            );
                        }
                        event = joining_swarm.select_next_some() => {
                            let _ = handle_libp2p_dht_server_event(
                                &mut joining_node,
                                &mut joining_swarm,
                                &mut joining_pending,
                                event,
                            );
                        }
                    }
                    if joining_node.kademlia.len() == 1 {
                        break;
                    }
                }
            })
            .await
            .unwrap();
            let closest = joining_node.kademlia.closest(
                lm_node::KademliaNodeId::from_peer_id(&discovered_announce.peer_id),
                1,
            );
            assert_eq!(closest.len(), 1);
            assert_eq!(closest[0].announce.peer_id, "libp2p-discovered-peer");
        });
    }

    #[test]
    fn libp2p_dht_rpc_request_handler_uses_native_node_logic() {
        let mut node = NativeNode::new(NodeConfig::default());
        let handler_identity = Identity::create_with_passphrase("libp2p handler")
            .unwrap()
            .0;
        let handler_announce = NodeConfig {
            peer_id: "libp2p-handler-peer".into(),
            ..Default::default()
        }
        .create_announce(&handler_identity)
        .unwrap();
        let record = DhtRecord::public_peer(
            &handler_announce,
            handler_announce.to_export_text().unwrap(),
            60,
        );
        let response = handle_libp2p_dht_rpc_request(
            &mut node,
            DhtRpcRequest::StoreRecord {
                request_id: "libp2p-store".into(),
                record,
            },
        );
        assert!(matches!(
            response,
            DhtRpcResponse::StoreResult {
                stored: true,
                inserted: true,
                ..
            }
        ));
    }

    #[test]
    fn libp2p_dht_rpc_roundtrips_find_value_between_local_swarms() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(async {
            let key = DhtRecordKey::for_public_peer("libp2p-roundtrip");
            let record = DhtRecord {
                key,
                kind: DhtRecordKind::PublicPeer,
                value: "roundtrip-value".into(),
                created_at: current_unix_timestamp(),
                expires_at: current_unix_timestamp().saturating_add(60),
                republish_at: current_unix_timestamp().saturating_add(30),
            };
            let mut server_node = NativeNode::new(NodeConfig::default());
            assert!(server_node.dht_records.store(record.clone()));

            let response = libp2p_dht_roundtrip(
                &mut server_node,
                DhtRpcRequest::FindValue {
                    request_id: "libp2p-find".into(),
                    key,
                    limit: 8,
                },
            )
            .await;
            match response {
                DhtRpcResponse::Value {
                    record: Some(found),
                    ..
                } => assert_eq!(found.value, "roundtrip-value"),
                other => panic!("unexpected response: {other:?}"),
            }
        });
    }

    #[test]
    fn libp2p_dht_rpc_roundtrips_store_record_between_local_swarms() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(async {
            let (identity, _) = Identity::create_with_passphrase("libp2p store roundtrip").unwrap();
            let announce = NodeConfig {
                peer_id: "libp2p-store-roundtrip".into(),
                ..Default::default()
            }
            .create_announce(&identity)
            .unwrap();
            let key = DhtRecordKey::for_public_peer(&announce.peer_id);
            let record = DhtRecord::public_peer(&announce, announce.to_export_text().unwrap(), 60);
            let expected_value = record.value.clone();
            let mut server_node = NativeNode::new(NodeConfig::default());
            let response = libp2p_dht_transport_roundtrip(
                &mut server_node,
                DhtRpcRequest::StoreRecord {
                    request_id: "libp2p-store".into(),
                    record,
                },
            )
            .await;
            assert!(matches!(
                response,
                DhtRpcResponse::StoreResult {
                    stored: true,
                    inserted: true,
                    ..
                }
            ));
            assert_eq!(
                server_node.dht_records.find_value(&key).unwrap().value,
                expected_value
            );
        });
    }

    #[test]
    fn libp2p_dht_rpc_roundtrips_find_node_between_local_swarms() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(async {
            let (identity, _) = Identity::create_with_passphrase("libp2p find node peer").unwrap();
            let announce = NodeConfig {
                peer_id: "libp2p-find-node-peer".into(),
                ..Default::default()
            }
            .create_announce(&identity)
            .unwrap();
            let mut server_node = NativeNode::new(NodeConfig::default());
            server_node
                .kademlia
                .insert_verified(announce.clone(), &identity.identity_public_key())
                .unwrap();
            let response = libp2p_dht_roundtrip(
                &mut server_node,
                DhtRpcRequest::FindNode {
                    request_id: "libp2p-find-node".into(),
                    target: lm_node::KademliaNodeId::from_peer_id(&announce.peer_id),
                    limit: 8,
                },
            )
            .await;
            match response {
                DhtRpcResponse::Nodes { nodes, .. } => {
                    assert_eq!(nodes.len(), 1);
                    assert_eq!(nodes[0].announce.peer_id, "libp2p-find-node-peer");
                }
                other => panic!("unexpected response: {other:?}"),
            }
        });
    }

    #[test]
    fn dht_rpc_http_client_posts_json_and_auth() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut raw = [0u8; 4096];
            let len = stream.read(&mut raw).unwrap();
            let request = String::from_utf8_lossy(&raw[..len]);
            assert!(request.starts_with("POST /dht/rpc HTTP/1.1"));
            assert!(request.contains("authorization: Bearer rpc-token"));
            assert!(request.contains("StoreRecord"));
            let body = serde_json::to_string(&DhtRpcResponse::StoreResult {
                request_id: "rpc-1".into(),
                stored: true,
                inserted: true,
            })
            .unwrap();
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        let peer = SyncPeerConfig {
            url: format!("http://{addr}"),
            token: Some("rpc-token".into()),
            peer_id: None,
        };
        let record = lm_node::DhtRecord {
            key: DhtRecordKey::for_public_peer("rpc-client"),
            kind: DhtRecordKind::PublicPeer,
            value: "value".into(),
            created_at: current_unix_timestamp(),
            expires_at: current_unix_timestamp().saturating_add(60),
            republish_at: current_unix_timestamp().saturating_add(30),
        };
        let response = send_dht_rpc(
            &peer,
            &DhtRpcRequest::StoreRecord {
                request_id: "rpc-1".into(),
                record,
            },
        )
        .unwrap();
        assert_eq!(
            response,
            DhtRpcResponse::StoreResult {
                request_id: "rpc-1".into(),
                stored: true,
                inserted: true,
            }
        );
        server.join().unwrap();
    }

    #[test]
    fn dht_rpc_http_client_rejects_mismatched_logical_request_id() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut raw = [0u8; 4096];
            let len = stream.read(&mut raw).unwrap();
            let request = String::from_utf8_lossy(&raw[..len]);
            assert!(request.starts_with("POST /dht/rpc HTTP/1.1"));
            let body = serde_json::to_string(&DhtRpcResponse::StoreResult {
                request_id: "wrong-request-id".into(),
                stored: true,
                inserted: true,
            })
            .unwrap();
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        let peer = SyncPeerConfig {
            url: format!("http://{addr}"),
            token: None,
            peer_id: None,
        };
        let record = lm_node::DhtRecord {
            key: DhtRecordKey::for_public_peer("rpc-client-mismatch"),
            kind: DhtRecordKind::PublicPeer,
            value: "value".into(),
            created_at: current_unix_timestamp(),
            expires_at: current_unix_timestamp().saturating_add(60),
            republish_at: current_unix_timestamp().saturating_add(30),
        };
        let err = send_dht_rpc(
            &peer,
            &DhtRpcRequest::StoreRecord {
                request_id: "expected-request-id".into(),
                record,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("request_id mismatch"));
        server.join().unwrap();
    }

    #[test]
    fn control_client_stream_has_read_and_write_timeouts() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            configure_control_client_stream(&stream).unwrap();
            assert_eq!(
                stream.read_timeout().unwrap(),
                Some(CONTROL_CLIENT_HTTP_TIMEOUT)
            );
            assert_eq!(
                stream.write_timeout().unwrap(),
                Some(CONTROL_CLIENT_HTTP_TIMEOUT)
            );
        });
        let stream = TcpStream::connect(addr).unwrap();
        drop(stream);
        server.join().unwrap();
    }

    #[test]
    fn control_peer_connect_uses_connect_timeout_path() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (_stream, _) = listener.accept().unwrap();
        });
        let stream = connect_control_peer(&addr.to_string()).unwrap();
        assert_eq!(stream.peer_addr().unwrap(), addr);
        drop(stream);
        server.join().unwrap();
        assert_eq!(CONTROL_PEER_CONNECT_TIMEOUT, Duration::from_secs(10));
    }

    #[test]
    fn control_peer_stream_has_read_and_write_timeouts() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (_stream, _) = listener.accept().unwrap();
        });
        let stream = TcpStream::connect(addr).unwrap();
        configure_control_peer_stream(&stream).unwrap();
        assert_eq!(
            stream.read_timeout().unwrap(),
            Some(CONTROL_PEER_HTTP_TIMEOUT)
        );
        assert_eq!(
            stream.write_timeout().unwrap(),
            Some(CONTROL_PEER_HTTP_TIMEOUT)
        );
        drop(stream);
        server.join().unwrap();
    }

    #[test]
    fn control_peer_http_client_rejects_oversized_response() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut raw = [0u8; 1024];
            let _ = stream.read(&mut raw).unwrap();
            let body = vec![b'a'; MAX_CONTROL_PEER_RESPONSE_BYTES + 1];
            let header = format!(
                "HTTP/1.1 200 OK
content-type: application/json
content-length: {}
connection: close

",
                body.len()
            );
            stream.write_all(header.as_bytes()).unwrap();
            stream.write_all(&body).unwrap();
        });
        let peer = SyncPeerConfig {
            url: format!("http://{addr}"),
            token: None,
            peer_id: None,
        };
        let err = http_control_request(&peer, "GET", "/sync/snapshot", "").unwrap_err();
        assert!(err.to_string().contains("response too large"));
        server.join().unwrap();
    }

    #[test]
    fn sync_backoff_is_exponential_and_capped() {
        assert_eq!(sync_backoff_delay_seconds(10, 300, 0), 10);
        assert_eq!(sync_backoff_delay_seconds(10, 300, 1), 10);
        assert_eq!(sync_backoff_delay_seconds(10, 300, 2), 20);
        assert_eq!(sync_backoff_delay_seconds(10, 300, 3), 40);
        assert_eq!(sync_backoff_delay_seconds(10, 30, 4), 30);
    }

    #[test]
    fn log_format_parses_text_and_json_aliases() {
        assert_eq!(parse_log_format("text").unwrap(), LogFormat::Text);
        assert_eq!(parse_log_format("plain").unwrap(), LogFormat::Text);
        assert_eq!(parse_log_format("json").unwrap(), LogFormat::Json);
        assert_eq!(parse_log_format("structured").unwrap(), LogFormat::Json);
        assert!(parse_log_format("xml").is_err());
    }

    #[test]
    fn dht_transport_kind_parses_supported_aliases() {
        assert_eq!(
            parse_dht_transport_kind("http-control").unwrap(),
            DhtTransportKind::HttpControl
        );
        assert_eq!(
            parse_dht_transport_kind("control").unwrap(),
            DhtTransportKind::HttpControl
        );
        assert_eq!(
            parse_dht_transport_kind("libp2p").unwrap(),
            DhtTransportKind::Libp2p
        );
        assert!(parse_dht_transport_kind("tcp").is_err());
    }

    #[test]
    fn control_logger_renders_structured_json_line() {
        let logger = ControlLogger::new(LogFormat::Json);
        let line = logger.render_line(
            "info",
            "control.request",
            "control request: GET /health status=200 duration_micros=5".into(),
            serde_json::json!({
                "method": "GET",
                "path": "/health",
                "status": 200,
                "duration_micros": 5,
            }),
        );
        let value: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(value["level"], "info");
        assert_eq!(value["event"], "control.request");
        assert_eq!(value["fields"]["method"], "GET");
        assert_eq!(value["fields"]["status"], 200);
        assert!(value["ts"].as_u64().unwrap() > 0);
    }

    #[test]
    fn control_http_response_sets_browser_security_headers() {
        let response = ControlHttpResponse::json(200, &serde_json::json!({"ok": true}))
            .to_http_string("https://app.example");
        assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(response.contains("\r\ncache-control: no-store\r\n"));
        assert!(response.contains("\r\nx-content-type-options: nosniff\r\n"));
        assert!(response.contains("\r\nreferrer-policy: no-referrer\r\n"));
        assert!(response.contains("\r\npermissions-policy: camera=(), microphone=(), geolocation=(), payment=(), usb=()\r\n"));
        assert!(response.contains("\r\ncontent-security-policy: default-src 'none'; frame-ancestors 'none'; base-uri 'none'\r\n"));
        assert!(response.contains("\r\naccess-control-allow-origin: https://app.example\r\n"));
    }

    #[test]
    fn control_logger_renders_text_line_with_compact_fields() {
        let logger = ControlLogger::new(LogFormat::Text);
        let line = logger.render_line(
            "warn",
            "control.security",
            "control security: no token configured; loopback clients only".into(),
            serde_json::json!({"auth": "loopback_only"}),
        );
        assert_eq!(
            line,
            "control security: no token configured; loopback clients only {\"auth\":\"loopback_only\"}"
        );
    }

    #[test]
    fn control_security_accepts_current_and_previous_tokens_for_rotation() {
        let security = ControlSecurityConfig {
            token: Some("new-token".into()),
            previous_tokens: vec!["old-token".into(), "older-token".into()],
            cors_allow_origins: Vec::new(),
        };
        let request = |token: &str| lm_node::ControlRequest {
            method: "GET".into(),
            path: "/sync/status".into(),
            body: String::new(),
            headers: vec![("authorization".into(), format!("Bearer {token}"))],
        };
        assert!(request_is_authorized(
            &request("new-token"),
            &security,
            None
        ));
        assert!(request_is_authorized(
            &request("old-token"),
            &security,
            None
        ));
        assert!(request_is_authorized(
            &request("older-token"),
            &security,
            None
        ));
        assert!(!request_is_authorized(
            &request("wrong-token"),
            &security,
            None
        ));
        assert!(!security.is_loopback_only());
    }

    #[test]
    fn oversized_control_request_maps_to_413() {
        assert_eq!(status_for_request_error("request body too large"), 413);
        assert_eq!(status_reason(413), "Payload Too Large");
        assert_eq!(status_for_request_error("request header too large"), 431);
        assert_eq!(status_for_request_error("request path too large"), 431);
        assert_eq!(status_reason(431), "Request Header Fields Too Large");
        assert_eq!(status_for_request_error("connection closed"), 400);
        assert!(
            control_error_http_response(413, "request error: request body too large")
                .starts_with("HTTP/1.1 413 Payload Too Large\r\n")
        );
        assert!(
            control_error_http_response(431, "request error: request header too large")
                .starts_with("HTTP/1.1 431 Request Header Fields Too Large\r\n")
        );
    }

    #[test]
    fn control_request_header_parser_rejects_smuggling_inputs() {
        assert_eq!(
            parse_content_length_and_validate_headers(
                "POST /sync/import HTTP/1.1\r\ncontent-length: 7\r\ncontent-length: 7"
            )
            .unwrap(),
            7
        );
        assert!(
            parse_content_length_and_validate_headers(
                "POST /sync/import HTTP/1.1\r\ncontent-length: 7\r\ncontent-length: 8"
            )
            .unwrap_err()
            .to_string()
            .contains("conflicting content-length")
        );
        assert!(
            parse_content_length_and_validate_headers(
                "POST /sync/import HTTP/1.1\r\ntransfer-encoding: chunked"
            )
            .unwrap_err()
            .to_string()
            .contains("unsupported transfer-encoding")
        );
        assert!(
            parse_content_length_and_validate_headers(&format!(
                "GET /health HTTP/1.1\r\nx-long: {}",
                "a".repeat(MAX_CONTROL_REQUEST_HEADER_LINE_BYTES + 1)
            ))
            .unwrap_err()
            .to_string()
            .contains("request header too large")
        );
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
        stats.record_dht_replication_run(
            DhtReplicationRunStats {
                records: 2,
                attempts: 4,
                successes: 3,
                failures: 1,
                peers_quarantined: 2,
            },
            456,
        );
        assert_eq!(stats.dht_replication_runs, 1);
        assert_eq!(stats.dht_replication_records, 2);
        assert_eq!(stats.dht_replication_attempts, 4);
        assert_eq!(stats.dht_replication_successes, 3);
        assert_eq!(stats.dht_replication_failures, 1);
        assert_eq!(stats.dht_replication_peers_quarantined, 2);
        assert_eq!(stats.last_dht_replication_at, Some(456));
        stats.record_dht_routing_refresh_run(
            DhtRoutingRefreshRunStats {
                targets: 2,
                attempts: 6,
                successes: 5,
                failures: 1,
                nodes_returned: 7,
                nodes_merged: 2,
                nodes_rejected_non_closer: 4,
                nodes_rejected_duplicate: 5,
                peers_quarantined: 3,
            },
            789,
        );
        assert_eq!(stats.dht_routing_refresh_runs, 1);
        assert_eq!(stats.dht_routing_refresh_targets, 2);
        assert_eq!(stats.dht_routing_refresh_attempts, 6);
        assert_eq!(stats.dht_routing_refresh_successes, 5);
        assert_eq!(stats.dht_routing_refresh_failures, 1);
        assert_eq!(stats.dht_routing_refresh_nodes_returned, 7);
        assert_eq!(stats.dht_routing_refresh_nodes_merged, 2);
        assert_eq!(stats.dht_routing_refresh_nodes_rejected_non_closer, 4);
        assert_eq!(stats.dht_routing_refresh_nodes_rejected_duplicate, 5);
        assert_eq!(stats.dht_routing_refresh_peers_quarantined, 3);
        assert_eq!(stats.last_dht_routing_refresh_at, Some(789));
        stats.record_dht_find_value_run(
            DhtFindValueRunStats {
                attempts: 4,
                successes: 3,
                failures: 1,
                found_records: 1,
                closer_records: 2,
                closer_nodes_returned: 5,
                closer_nodes_merged: 2,
                closer_nodes_rejected_non_closer: 4,
                closer_nodes_rejected_duplicate: 6,
                peers_quarantined: 1,
                exhausted: true,
                ..Default::default()
            },
            999,
        );
        assert_eq!(stats.dht_find_value_runs, 1);
        assert_eq!(stats.dht_find_value_attempts, 4);
        assert_eq!(stats.dht_find_value_successes, 3);
        assert_eq!(stats.dht_find_value_failures, 1);
        assert_eq!(stats.dht_find_value_found_records, 1);
        assert_eq!(stats.dht_find_value_closer_nodes_rejected_non_closer, 4);
        assert_eq!(stats.dht_find_value_closer_nodes_rejected_duplicate, 6);
        assert_eq!(stats.dht_find_value_peers_quarantined, 1);
        assert_eq!(stats.dht_find_value_query_rounds, 0);
        assert_eq!(stats.dht_find_value_alpha_max, 0);
        assert_eq!(stats.dht_find_value_exhausted, 1);
        assert_eq!(stats.last_dht_find_value_at, Some(999));
        stats.record_sync_schedule_delay(std::time::Duration::from_micros(11));
        stats.record_dht_replication_schedule_delay(std::time::Duration::from_micros(22));
        stats.record_dht_routing_refresh_schedule_delay(std::time::Duration::from_micros(33));
        assert_eq!(stats.last_sync_schedule_delay_micros, Some(11));
        assert_eq!(stats.last_dht_replication_schedule_delay_micros, Some(22));
        assert_eq!(
            stats.last_dht_routing_refresh_schedule_delay_micros,
            Some(33)
        );
        let health = stats.endpoints.get("GET /health").unwrap();
        assert_eq!(health.requests, 2);
        assert_eq!(health.responses_2xx, 2);
        assert_eq!(health.total_duration_micros, 30);
        assert_eq!(health.max_duration_micros, 20);
        assert_eq!(health.last_status, Some(201));
        let sync = stats.endpoints.get("GET /sync/status").unwrap();
        assert_eq!(sync.requests, 3);
        assert_eq!(sync.responses_4xx, 3);
        let metrics = stats.to_openmetrics(
            &NodeMaintenanceStats {
                prune_runs: 2,
                mailbox_expired_deliveries: 3,
                prekey_expired_bundles: 4,
                mailbox_push_rejects: MailboxPushRejectStats {
                    invalid_json: 2,
                    sender_rate_limited: 1,
                    ..Default::default()
                },
                dht_record_rejects: DhtRecordRejectStats {
                    invalid_json: 4,
                    invalid_record: 5,
                    ..Default::default()
                },
                mailbox_ack_rejects: lm_node::MailboxAckRejectStats {
                    invalid_json: 6,
                    too_many_ids: 7,
                    empty_id: 8,
                    ..Default::default()
                },
                routing_peer_rejects: lm_node::RoutingPeerRejectStats {
                    mismatched_node_id: 2,
                    invalid_signature: 3,
                    too_many_addresses: 4,
                    address_too_large: 5,
                    ..Default::default()
                },
                last_pruned_at: Some(1234),
            },
            Some(&StateDbStats {
                page_count: 10,
                page_size_bytes: 4096,
                freelist_count: 2,
                file_bytes: 40960,
                encrypted: false,
                permissions_hardened: true,
            }),
            Some(&NodeSyncStatus {
                peers: [(
                    "http://peer.example".to_string(),
                    lm_node::NodeSyncPeerStatus {
                        url: "http://peer.example".to_string(),
                        attempts: 5,
                        successes: 3,
                        failures: 2,
                        last_attempt_at: Some(100),
                        last_success_at: Some(90),
                        last_error_at: Some(80),
                        last_error: Some("timeout".to_string()),
                        next_attempt_at: Some(4_000_000_000),
                        consecutive_failures: DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES,
                        last_imported_peers: 1,
                        last_imported_mailbox_deliveries: 2,
                        last_imported_prekey_bundles: 3,
                        last_imported_signed_one_time_prekey_records: 4,
                    },
                )]
                .into_iter()
                .collect(),
            }),
            DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES,
        );
        assert!(metrics.contains("# TYPE lm_node_control_requests_total counter"));
        assert!(metrics.contains("lm_node_control_requests_total 7"));
        assert!(
            metrics.contains("lm_node_control_security_events_total{event=\"rate_limited\"} 1")
        );
        assert!(metrics.contains(
            "lm_node_control_endpoint_requests_total{endpoint=\"GET /sync/status\",class=\"4xx\"} 3"
        ));
        assert!(metrics.contains("lm_node_dht_replication_runs_total 1"));
        assert!(metrics.contains("lm_node_dht_replication_records_total 2"));
        assert!(metrics.contains("lm_node_dht_replication_attempts_total{result=\"success\"} 3"));
        assert!(metrics.contains("lm_node_dht_replication_attempts_total{result=\"failure\"} 1"));
        assert!(metrics.contains("lm_node_dht_replication_peers_quarantined_total 2"));
        assert!(metrics.contains("lm_node_dht_replication_last_run_at 456"));
        assert!(metrics.contains("lm_node_dht_routing_refresh_runs_total 1"));
        assert!(metrics.contains("lm_node_dht_routing_refresh_targets_total 2"));
        assert!(
            metrics.contains("lm_node_dht_routing_refresh_attempts_total{result=\"success\"} 5")
        );
        assert!(metrics.contains("lm_node_dht_routing_refresh_nodes_returned_total 7"));
        assert!(metrics.contains("lm_node_dht_routing_refresh_nodes_merged_total 2"));
        assert!(
            metrics.contains(
                "lm_node_dht_routing_refresh_nodes_rejected_total{reason=\"non_closer\"} 4"
            )
        );
        assert!(
            metrics.contains(
                "lm_node_dht_routing_refresh_nodes_rejected_total{reason=\"duplicate\"} 5"
            )
        );
        assert!(metrics.contains("lm_node_dht_routing_refresh_peers_quarantined_total 3"));
        assert!(metrics.contains("lm_node_dht_routing_refresh_last_run_at 789"));
        assert!(metrics.contains("lm_node_dht_find_value_runs_total 1"));
        assert!(metrics.contains("lm_node_dht_find_value_attempts_total{result=\"success\"} 3"));
        assert!(metrics.contains("lm_node_dht_find_value_attempts_total{result=\"failure\"} 1"));
        assert!(metrics.contains("lm_node_dht_find_value_records_total{kind=\"found\"} 1"));
        assert!(metrics.contains("lm_node_dht_find_value_records_total{kind=\"closer\"} 2"));
        assert!(metrics.contains("lm_node_dht_find_value_closer_nodes_total{kind=\"returned\"} 5"));
        assert!(metrics.contains("lm_node_dht_find_value_closer_nodes_total{kind=\"merged\"} 2"));
        assert!(
            metrics.contains(
                "lm_node_dht_find_value_closer_nodes_total{kind=\"rejected_non_closer\"} 4"
            )
        );
        assert!(
            metrics.contains(
                "lm_node_dht_find_value_closer_nodes_total{kind=\"rejected_duplicate\"} 6"
            )
        );
        assert!(metrics.contains("lm_node_dht_find_value_peers_quarantined_total 1"));
        assert!(metrics.contains("lm_node_dht_find_value_query_rounds_total 0"));
        assert!(metrics.contains("lm_node_dht_find_value_alpha_max 0"));
        assert!(metrics.contains("lm_node_dht_find_value_exhausted_total 1"));
        assert!(metrics.contains("lm_node_dht_find_value_last_run_at 999"));
        assert!(
            metrics.contains(r#"lm_node_sync_peer_attempts_total{peer="http://peer.example"} 5"#)
        );
        assert!(
            metrics.contains(r#"lm_node_sync_peer_successes_total{peer="http://peer.example"} 3"#)
        );
        assert!(
            metrics.contains(r#"lm_node_sync_peer_failures_total{peer="http://peer.example"} 2"#)
        );
        assert!(
            metrics.contains(
                format!(
                    r#"lm_node_sync_peer_consecutive_failures{{peer="http://peer.example"}} {}"#,
                    DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES
                )
                .as_str()
            )
        );
        assert!(metrics.contains(r#"lm_node_dht_peer_quarantined{peer="http://peer.example"} 1"#));
        assert!(metrics.contains(
            r#"lm_node_sync_peer_next_attempt_at{peer="http://peer.example"} 4000000000"#
        ));
        assert!(
            metrics.contains(
                "lm_node_background_schedule_delay_micros_total{job=\"snapshot_sync\"} 11"
            )
        );
        assert!(metrics.contains(
            "lm_node_background_schedule_delay_micros_total{job=\"dht_replication\"} 22"
        ));
        assert!(metrics.contains(
            "lm_node_background_schedule_delay_micros_last{job=\"dht_routing_refresh\"} 33"
        ));
        assert!(metrics.contains("lm_node_maintenance_prune_runs_total 2"));
        assert!(
            metrics
                .contains("lm_node_maintenance_expired_records_total{kind=\"mailbox_delivery\"} 3")
        );
        assert!(
            metrics.contains("lm_node_mailbox_push_rejections_total{reason=\"invalid_json\"} 2")
        );
        assert!(
            metrics.contains(
                "lm_node_mailbox_push_rejections_total{reason=\"sender_rate_limited\"} 1"
            )
        );
        assert!(metrics.contains("lm_node_mailbox_push_rejections_total{reason=\"all\"} 3"));
        assert!(metrics.contains("lm_node_dht_record_rejections_total{reason=\"invalid_json\"} 4"));
        assert!(
            metrics.contains("lm_node_dht_record_rejections_total{reason=\"invalid_record\"} 5")
        );
        assert!(metrics.contains("lm_node_dht_record_rejections_total{reason=\"all\"} 9"));
        assert!(
            metrics
                .contains("lm_node_routing_peer_rejections_total{reason=\"mismatched_node_id\"} 2")
        );
        assert!(
            metrics
                .contains("lm_node_routing_peer_rejections_total{reason=\"invalid_signature\"} 3")
        );
        assert!(
            metrics
                .contains("lm_node_routing_peer_rejections_total{reason=\"too_many_addresses\"} 4")
        );
        assert!(
            metrics
                .contains("lm_node_routing_peer_rejections_total{reason=\"address_too_large\"} 5")
        );
        assert!(metrics.contains("lm_node_routing_peer_rejections_total{reason=\"all\"} 14"));
        assert!(metrics.contains("lm_node_state_db_pages{kind=\"total\"} 10"));
        assert!(metrics.contains("lm_node_state_db_pages{kind=\"free\"} 2"));
        assert!(metrics.contains("lm_node_state_db_page_size_bytes 4096"));
        assert!(metrics.contains("lm_node_state_db_file_bytes 40960"));
        assert!(metrics.contains("lm_node_state_db_encrypted 0"));
        assert!(metrics.contains("lm_node_state_db_permissions_hardened 1"));
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
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        let leftovers = std::fs::read_dir(&dir).unwrap().count();
        assert_eq!(leftovers, 1);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn sqlite_state_db_sets_operational_pragmas() {
        let dir = std::env::temp_dir().join(format!(
            "lm-node-sqlite-pragmas-test-{}-{}",
            std::process::id(),
            current_unix_timestamp()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.sqlite3");
        let conn = open_state_db(path.to_str().unwrap()).unwrap();
        let busy_timeout: i64 = conn
            .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
            .unwrap();
        let foreign_keys: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(busy_timeout, 5000);
        assert_eq!(foreign_keys, 1);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn sqlite_state_roundtrip_persists_node_tables() {
        let dir = std::env::temp_dir().join(format!(
            "lm-node-sqlite-state-test-{}-{}",
            std::process::id(),
            current_unix_timestamp()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.sqlite3");
        let (alice, _) = Identity::create_with_passphrase("sqlite alice").unwrap();
        let (bob, _) = Identity::create_with_passphrase("sqlite bob").unwrap();
        let mut node = NativeNode::new(NodeConfig {
            peer_id: "sqlite-node".into(),
            ..Default::default()
        });
        let announce = NodeConfig {
            peer_id: "sqlite-peer".into(),
            ..Default::default()
        }
        .create_announce(&alice)
        .unwrap();
        node.routing_table
            .insert_verified(announce.clone(), &alice.identity_public_key())
            .unwrap();
        node.kademlia
            .insert_verified(announce, &alice.identity_public_key())
            .unwrap();
        let message = MailboxMessage::new(
            &alice,
            bob.user_id().clone(),
            MailboxMessageKind::DirectEnvelope,
            "ciphertext".into(),
            3600,
        )
        .unwrap();
        let delivery_id = node
            .mailbox
            .push_verified(message, &alice.identity_public_key())
            .unwrap();
        let page = node.mailbox.take_for(bob.user_id());
        assert_eq!(page.len(), 1);
        assert_eq!(
            node.mailbox
                .ack(bob.user_id(), std::slice::from_ref(&delivery_id)),
            1
        );
        let (bundle, _, records) =
            PreKeyBundle::new_with_signed_one_time_prekey_records(&alice, 7, 2, 3600).unwrap();
        node.prekeys
            .publish_verified_with_signed_one_time_prekey_records(bundle.clone(), records)
            .unwrap();
        assert!(node.prekeys.take_for(alice.user_id(), true).is_some());
        node.dht_records.store(DhtRecord::prekey(
            alice.user_id(),
            bundle.to_export_text().unwrap(),
            3600,
        ));
        node.maintenance.mailbox_push_rejects.invalid_json = 3;

        save_node_state_db(path.to_str().unwrap(), &node).unwrap();
        let restored = load_node_state_db(path.to_str().unwrap()).unwrap();

        assert_eq!(restored.config.peer_id, "sqlite-node");
        assert_eq!(restored.routing_table.len(), 1);
        assert_eq!(
            restored
                .routing_table
                .identity_public_key_for("sqlite-peer"),
            Some(BASE64.encode(alice.identity_public_key()).as_str())
        );
        assert_eq!(
            restored.kademlia.all_peers()[0]
                .identity_public_key
                .as_deref(),
            Some(BASE64.encode(alice.identity_public_key()).as_str())
        );
        assert_eq!(restored.mailbox.pending_for(bob.user_id()), 0);
        assert_eq!(
            restored
                .mailbox
                .delivery_status(bob.user_id(), &delivery_id)
                .status,
            lm_node::MailboxDeliveryState::Acked
        );
        assert_eq!(restored.prekeys.len(), 1);
        assert_eq!(restored.prekeys.consumed_for(alice.user_id()), vec![0]);
        assert_eq!(
            restored
                .prekeys
                .signed_one_time_prekey_records_for(alice.user_id())
                .len(),
            2
        );
        assert_eq!(restored.dht_records.len(), 1);
        assert_eq!(restored.maintenance.mailbox_push_rejects.invalid_json, 3);

        let conn = rusqlite::Connection::open(&path).unwrap();
        let mailbox_rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM mailbox_deliveries", [], |row| {
                row.get(0)
            })
            .unwrap();
        let mailbox_ack_receipt_rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM mailbox_ack_receipts", [], |row| {
                row.get(0)
            })
            .unwrap();
        let prekey_rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM prekey_bundles", [], |row| row.get(0))
            .unwrap();
        let signed_prekey_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM signed_one_time_prekey_records",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let peer_rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM public_peers", [], |row| row.get(0))
            .unwrap();
        let routing_peer_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM public_peers WHERE routing_peer_json IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(mailbox_rows, 0);
        assert_eq!(mailbox_ack_receipt_rows, 1);
        assert_eq!(prekey_rows, 1);
        assert_eq!(signed_prekey_rows, 2);
        assert_eq!(peer_rows, 1);
        assert_eq!(routing_peer_rows, 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn state_db_encryption_requirement_fails_closed_until_supported() {
        assert!(validate_state_db_encryption_requirement(false).is_ok());
        let err = validate_state_db_encryption_requirement(true).unwrap_err();
        assert!(err.to_string().contains("plain SQLite"));
    }

    #[test]
    fn secret_file_loader_trims_and_rejects_empty_files() {
        let path = std::env::temp_dir().join(format!(
            "lm-node-secret-test-{}-{}.txt",
            std::process::id(),
            current_unix_timestamp()
        ));
        std::fs::write(&path, "  secret-value\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = std::fs::metadata(&path).unwrap().permissions();
            permissions.set_mode(0o600);
            std::fs::set_permissions(&path, permissions).unwrap();
        }
        assert_eq!(
            read_secret_file(path.to_str().unwrap()).unwrap(),
            "secret-value"
        );
        std::fs::write(&path, "  \n").unwrap();
        assert!(read_secret_file(path.to_str().unwrap()).is_err());
        let _ = std::fs::remove_file(path);
    }

    #[cfg(unix)]
    #[test]
    fn secret_file_loader_rejects_group_or_world_readable_files() {
        use std::os::unix::fs::PermissionsExt;
        let path = std::env::temp_dir().join(format!(
            "lm-node-secret-perms-test-{}-{}.txt",
            std::process::id(),
            current_unix_timestamp()
        ));
        std::fs::write(&path, "secret-value\n").unwrap();
        let mut permissions = std::fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o644);
        std::fs::set_permissions(&path, permissions).unwrap();
        let err = read_secret_file(path.to_str().unwrap()).unwrap_err();
        assert!(err.to_string().contains("permissions too broad"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn serve_control_config_file_parses_sync_and_security() {
        let config: ServeControlConfigFile = serde_json::from_str(
            r#"{
                "bind": "127.0.0.1:9999",
                "peer_id": "cfg-node",
                "state_file": "state.json",
                "state_db": "state.sqlite3",
                "state_db_require_encryption": true,
                "control_token": "control",
                "control_token_file": "control.secret",
                "control_previous_tokens": ["old-control"],
                "cors_allow_origins": ["https://allowed.example"],
                "sync_interval_seconds": 5,
                "sync_max_backoff_seconds": 60,
                "dht_replication_factor": 5,
                "dht_routing_refresh_limit": 12,
                "dht_routing_refresh_max_targets": 16,
                "dht_transport": "libp2p",
                "dht_peer_quarantine_consecutive_failures": 9,
                "rate_limit_window_seconds": 30,
                "rate_limit_max_requests": 120,
                "log_format": "json",
                "mailbox_sender_rate_limit_window_seconds": 60,
                "mailbox_sender_rate_limit_max_messages": 20,
                "mailbox_global_rate_limit_window_seconds": 60,
                "mailbox_global_rate_limit_max_messages": 200,
                "max_mailbox_bytes": 987654,
                "max_mailbox_bytes_per_user": 123456,
                "max_mailbox_messages_per_user": 321,
                "sync_peers": [
                    { "url": "http://127.0.0.1:8787", "peer_id": "peer-8787", "token": "peer-token", "token_file": "peer.secret" },
                    { "url": "http://127.0.0.1:8788", "peer_id": "peer-8788", "token_file": "peer-8788.secret" }
                ]
            }"#,
        )
        .unwrap();
        assert_eq!(config.bind.as_deref(), Some("127.0.0.1:9999"));
        assert_eq!(config.peer_id.as_deref(), Some("cfg-node"));
        assert_eq!(config.state_db.as_deref(), Some("state.sqlite3"));
        assert_eq!(config.state_db_require_encryption, Some(true));
        assert_eq!(config.control_token.as_deref(), Some("control"));
        assert_eq!(config.control_token_file.as_deref(), Some("control.secret"));
        assert_eq!(
            config.control_previous_tokens.as_deref(),
            Some(["old-control".to_string()].as_slice())
        );
        assert_eq!(
            config.cors_allow_origins.unwrap(),
            vec!["https://allowed.example"]
        );
        assert_eq!(config.sync_interval_seconds, Some(5));
        assert_eq!(config.sync_max_backoff_seconds, Some(60));
        assert_eq!(config.dht_replication_factor, Some(5));
        assert_eq!(config.dht_routing_refresh_limit, Some(12));
        assert_eq!(config.dht_routing_refresh_max_targets, Some(16));
        assert_eq!(config.dht_transport.as_deref(), Some("libp2p"));
        assert_eq!(config.dht_peer_quarantine_consecutive_failures, Some(9));
        assert_eq!(config.rate_limit_window_seconds, Some(30));
        assert_eq!(config.rate_limit_max_requests, Some(120));
        assert_eq!(config.log_format.as_deref(), Some("json"));
        assert_eq!(config.mailbox_sender_rate_limit_window_seconds, Some(60));
        assert_eq!(config.mailbox_sender_rate_limit_max_messages, Some(20));
        assert_eq!(config.mailbox_global_rate_limit_window_seconds, Some(60));
        assert_eq!(config.mailbox_global_rate_limit_max_messages, Some(200));
        assert_eq!(config.max_mailbox_bytes, Some(987654));
        assert_eq!(config.max_mailbox_bytes_per_user, Some(123456));
        assert_eq!(config.max_mailbox_messages_per_user, Some(321));
        let sync_peers = config.sync_peers.unwrap();
        assert_eq!(sync_peers.len(), 2);
        let peer = &sync_peers[0];
        assert_eq!(peer.url, "http://127.0.0.1:8787");
        assert_eq!(peer.peer_id.as_deref(), Some("peer-8787"));
        assert_eq!(peer.token.as_deref(), Some("peer-token"));
        assert_eq!(peer.token_file.as_deref(), Some("peer.secret"));
        let peer = &sync_peers[1];
        assert_eq!(peer.url, "http://127.0.0.1:8788");
        assert_eq!(peer.peer_id.as_deref(), Some("peer-8788"));
        assert_eq!(peer.token, None);
        assert_eq!(peer.token_file.as_deref(), Some("peer-8788.secret"));
    }
}
