use lm_core::PublicPeerAnnounce;
use lm_node::{
    ControlRequest, NativeNode, NodeConfig, NodeStateSnapshot, decode_identity_public_key_base64,
    parse_capabilities_csv, restore_identity_from_backup_text,
};
use serde::Deserialize;
use std::{
    env, fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
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
    fs::write(path, text)?;
    Ok(())
}

fn serve_control(
    bind: &str,
    node: &mut NativeNode,
    state_file: Option<&str>,
    sync_peers: Vec<SyncPeerConfig>,
    sync_interval_seconds: u64,
    sync_max_backoff_seconds: u64,
    security: ControlSecurityConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(bind)?;
    listener.set_nonblocking(true)?;
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
        "endpoints: GET /health, POST /announce, GET /peers/closest, POST /mailbox/push, GET /mailbox/take, POST /mailbox/ack, POST /prekey/publish, GET /prekey/get, GET /sync/snapshot, GET /sync/status, POST /sync/import"
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
        match listener.accept() {
            Ok((mut stream, _addr)) => {
                if let Err(err) = handle_stream(&mut stream, node, &security) {
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
) -> Result<(), Box<dyn std::error::Error>> {
    let peer_addr = stream.peer_addr().ok();
    let request = read_http_request(stream)?;
    let origin = request.header("origin").map(str::to_string);
    let response = if !security.allows_origin(origin.as_deref()) {
        ControlHttpResponse::text(403, "cors origin not allowed")
    } else if request.method == "OPTIONS" {
        ControlHttpResponse::from_control(node.handle_control_request(request))
    } else if !request_is_authorized(&request, security, peer_addr.as_ref()) {
        ControlHttpResponse::text(401, "unauthorized")
    } else {
        ControlHttpResponse::from_control(node.handle_control_request(request))
    };
    let allow_origin = security.access_control_origin(origin.as_deref());
    stream.write_all(response.to_http_string(&allow_origin).as_bytes())?;
    Ok(())
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
serve-control [--config-file <json>] [--bind <host:port>] [--peer-id <id>] [--state-file <file>] [--sync-peer <url,csv>] [--sync-interval-seconds <n>]\n"
    );
}

#[cfg(test)]
mod tests {
    use super::{
        ServeControlConfigFile, current_unix_timestamp, read_secret_file,
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
        let peer = &config.sync_peers.unwrap()[0];
        assert_eq!(peer.url, "http://127.0.0.1:8787");
        assert_eq!(peer.token.as_deref(), Some("peer-token"));
        assert_eq!(peer.token_file.as_deref(), Some("peer.secret"));
    }
}
