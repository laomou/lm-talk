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
    let push = http_json(
        &base_a,
        "POST",
        "/mailbox/push",
        json!({
            "message_text": mailbox.to_export_text().unwrap(),
            "from_identity_public_key": BASE64.encode(alice.identity_public_key()),
        }),
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
    let _node_a = spawn_node(&base_a, "http-auto-node-a");
    let _node_b = spawn_node_with_args(
        &base_b,
        "http-auto-node-b",
        &[
            "--sync-peer",
            &format!("http://{base_a}"),
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
    let push = http_json(
        &base_a,
        "POST",
        "/mailbox/push",
        json!({
            "message_text": mailbox.to_export_text().unwrap(),
            "from_identity_public_key": BASE64.encode(alice.identity_public_key()),
        }),
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
    http_request(addr, method, path, &value.to_string())
}

fn http_request(addr: &str, method: &str, path: &str, body: &str) -> HttpResponse {
    try_http_request(addr, method, path, body).unwrap_or_else(|err| {
        panic!("HTTP request {method} {path} to {addr} failed: {err}");
    })
}

fn try_http_request(
    addr: &str,
    method: &str,
    path: &str,
    body: &str,
) -> std::io::Result<HttpResponse> {
    let mut stream = TcpStream::connect(addr)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let request = format!(
        "{method} {path} HTTP/1.1\r\nhost: {addr}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
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
