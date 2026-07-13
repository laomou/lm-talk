use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use lm_core::{Identity, MailboxMessage, MailboxMessageKind, PreKeyBundle};
use lm_node::NodeStateSnapshot;
use serde_json::json;
use std::{
    env,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct TestNodeProcess {
    child: Child,
    state_file: PathBuf,
}

impl Drop for TestNodeProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.state_file);
    }
}

#[derive(Debug)]
struct HttpResponse {
    status: u16,
    body: String,
}

#[test]
fn real_http_control_plane_syncs_prekeys_and_mailbox_between_nodes() {
    let port_a = free_port();
    let port_b = free_port();
    let base_a = format!("127.0.0.1:{port_a}");
    let base_b = format!("127.0.0.1:{port_b}");
    let _node_a = spawn_node(&base_a, "http-node-a");
    let _node_b = spawn_node(&base_b, "http-node-b");
    wait_for_health(&base_a);
    wait_for_health(&base_b);

    let health = http_request(&base_a, "GET", "/health", "");
    assert_eq!(health.status, 200);
    assert!(health.body.contains("http-node-a"));

    let cors = http_request(&base_a, "OPTIONS", "/prekey/get", "");
    assert_eq!(cors.status, 200);

    let (alice, _) = Identity::create_with_passphrase("http alice").unwrap();
    let (bob, _) = Identity::create_with_passphrase("http bob").unwrap();

    // Bob publishes a signed prekey bundle to node B over the real HTTP listener.
    let (bob_bundle, _) = PreKeyBundle::new(&bob, 7, 2, 3600).unwrap();
    let publish = http_json(
        &base_b,
        "POST",
        "/prekey/publish",
        json!({ "prekey_bundle_text": bob_bundle.to_export_text().unwrap() }),
    );
    assert_eq!(publish.status, 201, "{}", publish.body);

    // Node A imports node B's snapshot and can then serve Bob's prekey.
    let snapshot_b = http_request(&base_b, "GET", "/sync/snapshot", "");
    assert_eq!(snapshot_b.status, 200);
    let snapshot: NodeStateSnapshot = serde_json::from_str(&snapshot_b.body).unwrap();
    let import = http_json(
        &base_a,
        "POST",
        "/sync/import",
        json!({ "snapshot": snapshot }),
    );
    assert_eq!(import.status, 200, "{}", import.body);

    let get_prekey = http_request(
        &base_a,
        "GET",
        &format!("/prekey/get?user_id={}&consume=true", bob.user_id()),
        "",
    );
    assert_eq!(get_prekey.status, 200, "{}", get_prekey.body);
    let get_body: serde_json::Value = serde_json::from_str(&get_prekey.body).unwrap();
    assert_eq!(get_body["found"], true);
    assert_eq!(get_body["selected_one_time_prekey_id"], 0);
    assert_eq!(get_body["consumed_one_time_prekey_ids"], json!([0]));

    // Alice stores an encrypted envelope placeholder in node A's mailbox for Bob.
    let mailbox = MailboxMessage::new(
        &alice,
        bob.user_id().clone(),
        MailboxMessageKind::DirectEnvelope,
        "ratchet-envelope-json-placeholder".into(),
        3600,
    )
    .unwrap();
    let push = http_json_with_headers(
        &base_a,
        "POST",
        "/mailbox/push",
        json!({
            "message_text": mailbox.to_export_text().unwrap(),
            "from_identity_public_key": BASE64.encode(alice.identity_public_key()),
        }),
        &[("authorization", "Bearer sync-secret")],
    );
    assert_eq!(push.status, 201, "{}", push.body);

    // Node B imports node A's snapshot and Bob can take + ack the delivery.
    let snapshot_a = http_request(&base_a, "GET", "/sync/snapshot", "");
    assert_eq!(snapshot_a.status, 200);
    let snapshot: NodeStateSnapshot = serde_json::from_str(&snapshot_a.body).unwrap();
    let import = http_json(
        &base_b,
        "POST",
        "/sync/import",
        json!({ "snapshot": snapshot }),
    );
    assert_eq!(import.status, 200, "{}", import.body);

    let take = http_request(
        &base_b,
        "GET",
        &format!("/mailbox/take?user_id={}", bob.user_id()),
        "",
    );
    assert_eq!(take.status, 200, "{}", take.body);
    let take_body: serde_json::Value = serde_json::from_str(&take.body).unwrap();
    let messages = take_body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0]["message"]["ciphertext"].as_str().unwrap(),
        "ratchet-envelope-json-placeholder"
    );
    let delivery_id = messages[0]["delivery_id"].as_str().unwrap();

    let ack = http_json(
        &base_b,
        "POST",
        "/mailbox/ack",
        json!({
            "user_id": bob.user_id().to_string(),
            "delivery_ids": [delivery_id],
        }),
    );
    assert_eq!(ack.status, 200, "{}", ack.body);
    let ack_body: serde_json::Value = serde_json::from_str(&ack.body).unwrap();
    assert_eq!(ack_body["removed"], 1);
    assert_eq!(ack_body["pending"], 0);

    let take_again = http_request(
        &base_b,
        "GET",
        &format!("/mailbox/take?user_id={}", bob.user_id()),
        "",
    );
    assert_eq!(take_again.status, 200);
    let take_again_body: serde_json::Value = serde_json::from_str(&take_again.body).unwrap();
    assert_eq!(take_again_body["messages"].as_array().unwrap().len(), 0);
}

#[test]
fn real_http_control_plane_auto_snapshot_sync_imports_mailbox() {
    let port_a = free_port();
    let port_b = free_port();
    let base_a = format!("127.0.0.1:{port_a}");
    let base_b = format!("127.0.0.1:{port_b}");
    let _node_a = spawn_node_with_args(
        &base_a,
        "http-auto-node-a",
        &["--control-token", "sync-secret"],
    );
    let _node_b = spawn_node_with_args(
        &base_b,
        "http-auto-node-b",
        &[
            "--sync-peer",
            &format!("http://{base_a}"),
            "--sync-peer-token",
            "sync-secret",
            "--sync-interval-seconds",
            "1",
        ],
    );
    wait_for_health(&base_a);
    wait_for_health(&base_b);

    let (alice, _) = Identity::create_with_passphrase("http auto alice").unwrap();
    let (bob, _) = Identity::create_with_passphrase("http auto bob").unwrap();
    let mailbox = MailboxMessage::new(
        &alice,
        bob.user_id().clone(),
        MailboxMessageKind::DirectEnvelope,
        "auto-sync-mailbox".into(),
        3600,
    )
    .unwrap();
    let push = http_json_with_headers(
        &base_a,
        "POST",
        "/mailbox/push",
        json!({
            "message_text": mailbox.to_export_text().unwrap(),
            "from_identity_public_key": BASE64.encode(alice.identity_public_key()),
        }),
        &[("authorization", "Bearer sync-secret")],
    );
    assert_eq!(push.status, 201, "{}", push.body);

    let deadline = Instant::now() + Duration::from_secs(6);
    loop {
        let take = http_request(
            &base_b,
            "GET",
            &format!("/mailbox/take?user_id={}", bob.user_id()),
            "",
        );
        assert_eq!(take.status, 200, "{}", take.body);
        let body: serde_json::Value = serde_json::from_str(&take.body).unwrap();
        let messages = body["messages"].as_array().unwrap();
        if messages.len() == 1 {
            assert_eq!(
                messages[0]["message"]["ciphertext"].as_str().unwrap(),
                "auto-sync-mailbox"
            );
            break;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for automatic snapshot sync"
        );
        thread::sleep(Duration::from_millis(100));
    }

    let sync_status = http_request(&base_b, "GET", "/sync/status", "");
    assert_eq!(sync_status.status, 200, "{}", sync_status.body);
    let sync_body: serde_json::Value = serde_json::from_str(&sync_status.body).unwrap();
    let peer_status = &sync_body["peers"][format!("http://{base_a}")];
    assert!(peer_status["successes"].as_u64().unwrap() >= 1);
    assert!(peer_status["last_success_at"].as_u64().is_some());
    assert_eq!(peer_status["last_error"], serde_json::Value::Null);
}

#[test]
fn real_http_control_plane_loads_config_file() {
    let port = free_port();
    let base = format!("127.0.0.1:{port}");
    let config_file = env::temp_dir().join(format!(
        "lm-node-http-config-test-{}-{}.json",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let state_file = config_file.with_extension("state.json");
    let token_file = config_file.with_extension("token");
    std::fs::write(&token_file, "config-secret\n").unwrap();
    std::fs::write(
        &config_file,
        json!({
            "bind": base,
            "peer_id": "http-config-node",
            "state_file": state_file,
            "control_token_file": token_file,
            "cors_allow_origins": ["https://allowed.example"],
            "sync_interval_seconds": 0,
            "sync_max_backoff_seconds": 7,
            "sync_peers": []
        })
        .to_string(),
    )
    .unwrap();
    let _node = spawn_node_config(&config_file);
    wait_for_health(&base);

    let unauthorized = http_request(&base, "GET", "/sync/snapshot", "");
    assert_eq!(unauthorized.status, 401);
    let authorized = http_request_with_headers(
        &base,
        "GET",
        "/sync/snapshot",
        "",
        &[
            ("authorization", "Bearer config-secret"),
            ("origin", "https://allowed.example"),
        ],
    );
    assert_eq!(authorized.status, 200, "{}", authorized.body);
    assert!(authorized.body.contains("http-config-node"));
    let _ = std::fs::remove_file(&config_file);
    let _ = std::fs::remove_file(&state_file);
    let _ = std::fs::remove_file(&token_file);
}

#[test]
fn real_http_control_plane_rate_limits_non_health_requests() {
    let port = free_port();
    let base = format!("127.0.0.1:{port}");
    let _node = spawn_node_with_args(
        &base,
        "http-rate-limit-node",
        &[
            "--rate-limit-window-seconds",
            "60",
            "--rate-limit-max-requests",
            "1",
        ],
    );
    wait_for_health(&base);

    // Health checks stay outside the limiter so supervisors can keep probing.
    assert_eq!(http_request(&base, "GET", "/health", "").status, 200);
    assert_eq!(http_request(&base, "GET", "/health", "").status, 200);

    let first = http_request(&base, "GET", "/sync/status", "");
    assert_eq!(first.status, 200, "{}", first.body);
    let limited = http_request(&base, "GET", "/sync/status", "");
    assert_eq!(limited.status, 429, "{}", limited.body);
}

#[test]
fn real_http_control_plane_exposes_runtime_stats() {
    let port = free_port();
    let base = format!("127.0.0.1:{port}");
    let _node = spawn_node_with_args(
        &base,
        "http-stats-node",
        &[
            "--control-token",
            "stats-secret",
            "--cors-allow-origin",
            "https://allowed.example",
        ],
    );
    wait_for_health(&base);

    let baseline = http_request_with_headers(
        &base,
        "GET",
        "/control/stats",
        "",
        &[("authorization", "Bearer stats-secret")],
    );
    assert_eq!(baseline.status, 200, "{}", baseline.body);
    let baseline: serde_json::Value = serde_json::from_str(&baseline.body).unwrap();
    let baseline_requests = baseline["requests_total"].as_u64().unwrap();
    let baseline_2xx = baseline["responses_2xx"].as_u64().unwrap();
    let baseline_4xx = baseline["responses_4xx"].as_u64().unwrap();
    let baseline_unauthorized = baseline["unauthorized"].as_u64().unwrap();
    let baseline_cors_rejected = baseline["cors_rejected"].as_u64().unwrap();
    let baseline_stats_endpoint = baseline["endpoints"]["GET /control/stats"]["requests"]
        .as_u64()
        .unwrap_or(0);
    let baseline_sync_export_bytes = baseline["sync_snapshot_export_bytes"].as_u64().unwrap();
    let baseline_sync_exports = baseline["sync_snapshot_exports"].as_u64().unwrap();

    let unauthorized = http_request(&base, "GET", "/sync/status", "");
    assert_eq!(unauthorized.status, 401);
    let forbidden_origin = http_request_with_headers(
        &base,
        "GET",
        "/sync/status",
        "",
        &[
            ("authorization", "Bearer stats-secret"),
            ("origin", "https://evil.example"),
        ],
    );
    assert_eq!(forbidden_origin.status, 403);
    let ok = http_request_with_headers(
        &base,
        "GET",
        "/sync/status",
        "",
        &[("authorization", "Bearer stats-secret")],
    );
    assert_eq!(ok.status, 200, "{}", ok.body);
    let snapshot = http_request_with_headers(
        &base,
        "GET",
        "/sync/snapshot",
        "",
        &[("authorization", "Bearer stats-secret")],
    );
    assert_eq!(snapshot.status, 200, "{}", snapshot.body);

    let stats = http_request_with_headers(
        &base,
        "GET",
        "/control/stats",
        "",
        &[("authorization", "Bearer stats-secret")],
    );
    assert_eq!(stats.status, 200, "{}", stats.body);
    let body: serde_json::Value = serde_json::from_str(&stats.body).unwrap();
    assert!(body["started_at"].as_u64().unwrap() > 0);
    assert_eq!(
        body["requests_total"].as_u64().unwrap(),
        baseline_requests + 5
    );
    assert_eq!(body["responses_2xx"].as_u64().unwrap(), baseline_2xx + 3);
    assert_eq!(body["responses_4xx"].as_u64().unwrap(), baseline_4xx + 2);
    assert_eq!(
        body["unauthorized"].as_u64().unwrap(),
        baseline_unauthorized + 1
    );
    assert_eq!(
        body["cors_rejected"].as_u64().unwrap(),
        baseline_cors_rejected + 1
    );
    let sync_endpoint = &body["endpoints"]["GET /sync/status"];
    assert_eq!(sync_endpoint["requests"].as_u64().unwrap(), 3);
    assert_eq!(sync_endpoint["responses_2xx"].as_u64().unwrap(), 1);
    assert_eq!(sync_endpoint["responses_4xx"].as_u64().unwrap(), 2);
    assert!(sync_endpoint["total_duration_micros"].as_u64().is_some());
    assert!(sync_endpoint["max_duration_micros"].as_u64().is_some());
    assert_eq!(sync_endpoint["last_status"].as_u64().unwrap(), 200);
    assert_eq!(
        body["endpoints"]["GET /control/stats"]["requests"]
            .as_u64()
            .unwrap(),
        baseline_stats_endpoint + 1
    );
    assert_eq!(
        body["sync_snapshot_exports"].as_u64().unwrap(),
        baseline_sync_exports + 1
    );
    assert_eq!(
        body["sync_snapshot_export_bytes"].as_u64().unwrap(),
        baseline_sync_export_bytes + snapshot.body.len() as u64
    );
    assert!(body["maintenance"]["prune_runs"].as_u64().unwrap() >= 1);
    assert!(
        body["maintenance"]["mailbox_expired_deliveries"]
            .as_u64()
            .is_some()
    );
    assert!(
        body["maintenance"]["prekey_expired_bundles"]
            .as_u64()
            .is_some()
    );
    assert!(body["maintenance"]["last_pruned_at"].as_u64().is_some());

    let metrics = http_request_with_headers(
        &base,
        "GET",
        "/control/metrics",
        "",
        &[("authorization", "Bearer stats-secret")],
    );
    assert_eq!(metrics.status, 200, "{}", metrics.body);
    assert!(
        metrics
            .body
            .contains("# TYPE lm_node_control_requests_total counter")
    );
    assert!(
        metrics
            .body
            .contains("lm_node_control_security_events_total{event=\"unauthorized\"}")
    );
    assert!(metrics.body.contains(
        "lm_node_control_endpoint_requests_total{endpoint=\"GET /sync/status\",class=\"4xx\"}"
    ));
    assert!(
        metrics.body.contains(
            "lm_node_control_endpoint_duration_micros_total{endpoint=\"GET /sync/status\"}"
        )
    );
    assert!(metrics.body.ends_with("# EOF\n"));
}

#[test]
fn real_http_control_plane_requires_token_and_enforces_cors() {
    let port = free_port();
    let base = format!("127.0.0.1:{port}");
    let _node = spawn_node_with_args(
        &base,
        "http-secure-node",
        &[
            "--control-token",
            "secret-token",
            "--cors-allow-origin",
            "https://allowed.example",
        ],
    );
    wait_for_health(&base);

    let unauthorized = http_request(&base, "GET", "/sync/snapshot", "");
    assert_eq!(unauthorized.status, 401);

    let forbidden_origin = http_request_with_headers(
        &base,
        "GET",
        "/sync/snapshot",
        "",
        &[
            ("authorization", "Bearer secret-token"),
            ("origin", "https://evil.example"),
        ],
    );
    assert_eq!(forbidden_origin.status, 403);

    let authorized = http_request_with_headers(
        &base,
        "GET",
        "/sync/snapshot",
        "",
        &[
            ("authorization", "Bearer secret-token"),
            ("origin", "https://allowed.example"),
        ],
    );
    assert_eq!(authorized.status, 200, "{}", authorized.body);
    assert!(authorized.body.contains("http-secure-node"));
}

fn spawn_node_config(config_file: &std::path::Path) -> TestNodeProcess {
    let state_file = env::temp_dir().join(format!(
        "lm-node-http-config-dummy-state-{}-{}.json",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let child = Command::new(lm_node_binary())
        .args(["serve-control", "--config-file"])
        .arg(config_file)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|err| panic!("failed to spawn lm_node serve-control with config: {err}"));
    TestNodeProcess { child, state_file }
}

fn spawn_node(bind: &str, peer_id: &str) -> TestNodeProcess {
    spawn_node_with_args(bind, peer_id, &[])
}

fn spawn_node_with_args(bind: &str, peer_id: &str, extra_args: &[&str]) -> TestNodeProcess {
    let state_file = env::temp_dir().join(format!(
        "lm-node-http-test-{peer_id}-{}-{}.json",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let child = Command::new(lm_node_binary())
        .args([
            "serve-control",
            "--bind",
            bind,
            "--peer-id",
            peer_id,
            "--state-file",
        ])
        .arg(&state_file)
        .args(extra_args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|err| panic!("failed to spawn lm_node serve-control: {err}"));
    TestNodeProcess { child, state_file }
}

fn lm_node_binary() -> PathBuf {
    if let Some(path) = option_env!("CARGO_BIN_EXE_lm_node") {
        return PathBuf::from(path);
    }
    let mut path = env::current_exe().unwrap();
    path.pop();
    if path.file_name().and_then(|v| v.to_str()) == Some("deps") {
        path.pop();
    }
    path.push("lm_node");
    path
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn wait_for_health(addr: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(response) = try_http_request(addr, "GET", "/health", "") {
            if response.status == 200 {
                return;
            }
        }
        if Instant::now() >= deadline {
            panic!("timed out waiting for node health at {addr}");
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn http_json(addr: &str, method: &str, path: &str, value: serde_json::Value) -> HttpResponse {
    http_json_with_headers(addr, method, path, value, &[])
}

fn http_json_with_headers(
    addr: &str,
    method: &str,
    path: &str,
    value: serde_json::Value,
    headers: &[(&str, &str)],
) -> HttpResponse {
    http_request_with_headers(addr, method, path, &value.to_string(), headers)
}

fn http_request(addr: &str, method: &str, path: &str, body: &str) -> HttpResponse {
    http_request_with_headers(addr, method, path, body, &[])
}

fn http_request_with_headers(
    addr: &str,
    method: &str,
    path: &str,
    body: &str,
    headers: &[(&str, &str)],
) -> HttpResponse {
    try_http_request_with_headers(addr, method, path, body, headers).unwrap_or_else(|err| {
        panic!("HTTP request {method} {path} to {addr} failed: {err}");
    })
}

fn try_http_request(
    addr: &str,
    method: &str,
    path: &str,
    body: &str,
) -> std::io::Result<HttpResponse> {
    try_http_request_with_headers(addr, method, path, body, &[])
}

fn try_http_request_with_headers(
    addr: &str,
    method: &str,
    path: &str,
    body: &str,
    headers: &[(&str, &str)],
) -> std::io::Result<HttpResponse> {
    let mut stream = TcpStream::connect(addr)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let extra_headers = headers
        .iter()
        .map(|(name, value)| format!("{name}: {value}\r\n"))
        .collect::<String>();
    let request = format!(
        "{method} {path} HTTP/1.1\r\nhost: {addr}\r\ncontent-type: application/json\r\n{extra_headers}content-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes())?;

    let mut raw = Vec::new();
    stream.read_to_end(&mut raw)?;
    let header_end = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "missing headers"))?;
    let headers = String::from_utf8_lossy(&raw[..header_end]);
    let status = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "missing status"))?;
    let body = String::from_utf8_lossy(&raw[header_end + 4..]).into_owned();
    Ok(HttpResponse { status, body })
}
