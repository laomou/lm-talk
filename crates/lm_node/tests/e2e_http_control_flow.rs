use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use lm_core::{
    Identity, MailboxMessage, MailboxMessageKind, PreKeyBundle, SignedOneTimePreKeyRecord,
};
use lm_node::{DhtRecord, NodeConfig, NodeStateSnapshot};
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
        cleanup_state_path(&self.state_file);
    }
}

#[derive(Debug)]
struct HttpResponse {
    status: u16,
    body: String,
}

#[test]
fn real_http_control_plane_dht_find_value_runs_iterative_query() {
    let port_a = free_port();
    let port_b = free_port();
    let base_a = format!("127.0.0.1:{port_a}");
    let base_b = format!("127.0.0.1:{port_b}");
    let _node_a = spawn_node(&base_a, "http-dht-find-source");
    let _node_b = spawn_node_with_args(
        &base_b,
        "http-dht-find-query",
        &[
            "--sync-peer",
            &format!("http://{base_a}"),
            "--sync-interval-seconds",
            "0",
        ],
    );
    wait_for_health(&base_a);
    wait_for_health(&base_b);

    let (identity, _) = Identity::create_with_passphrase("http dht find owner").unwrap();
    let announce = NodeConfig {
        peer_id: "http-dht-found-peer".into(),
        ..Default::default()
    }
    .create_announce(&identity)
    .unwrap();
    let record = DhtRecord::public_peer(&announce, announce.to_export_text().unwrap(), 3600);
    let key = record.key;
    let store = http_json(
        &base_a,
        "POST",
        "/api/dht/record",
        json!({ "record": record }),
    );
    assert_eq!(store.status, 201, "{}", store.body);

    let find = http_request(
        &base_b,
        "GET",
        &format!("/api/dht/find-value?key={key}&limit=8&max_peers=2"),
        "",
    );
    assert_eq!(find.status, 200, "{}", find.body);
    let body: serde_json::Value = serde_json::from_str(&find.body).unwrap();
    assert_eq!(body["found"], true);
    assert_eq!(body["stats"]["attempts"], 1);
    assert_eq!(body["stats"]["found_records"], 1);

    let get = http_request(&base_b, "GET", &format!("/api/dht/record?key={key}"), "");
    assert_eq!(get.status, 200, "{}", get.body);
    let get_body: serde_json::Value = serde_json::from_str(&get.body).unwrap();
    assert_eq!(get_body["found"], true);
    assert_eq!(
        get_body["record"]["value"],
        announce.to_export_text().unwrap()
    );

    let stats = http_request(&base_b, "GET", "/api/control/stats", "");
    assert_eq!(stats.status, 200, "{}", stats.body);
    let stats_body: serde_json::Value = serde_json::from_str(&stats.body).unwrap();
    assert_eq!(stats_body["dht_find_value_runs"], 1);
    assert_eq!(stats_body["dht_find_value_attempts"], 1);
    assert_eq!(stats_body["dht_find_value_successes"], 1);
    assert_eq!(stats_body["dht_find_value_found_records"], 1);
    assert!(stats_body["last_dht_find_value_at"].as_u64().is_some());

    let metrics = http_request(&base_b, "GET", "/api/control/metrics", "");
    assert_eq!(metrics.status, 200, "{}", metrics.body);
    assert!(metrics.body.contains("lm_node_dht_find_value_runs_total 1"));
    assert!(
        metrics
            .body
            .contains("lm_node_dht_find_value_attempts_total{result=\"success\"} 1")
    );
    assert!(
        metrics
            .body
            .contains("lm_node_dht_find_value_records_total{kind=\"found\"} 1")
    );
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

    let health = http_request(&base_a, "GET", "/api/health", "");
    assert_eq!(health.status, 200);
    assert!(health.body.contains("http-node-a"));
    let health_body: serde_json::Value = serde_json::from_str(&health.body).unwrap();
    assert_eq!(health_body["dht_record_capacity"].as_u64().unwrap(), 4096);

    let cors = http_request(&base_a, "OPTIONS", "/api/prekey/get", "");
    assert_eq!(cors.status, 200);

    let (alice, _) = Identity::create_with_passphrase("http alice").unwrap();
    let (bob, _) = Identity::create_with_passphrase("http bob").unwrap();

    // Bob publishes a signed prekey bundle to node B over the real HTTP listener.
    let (bob_bundle, _, bob_signed_otks) =
        PreKeyBundle::new_with_signed_one_time_prekey_records(&bob, 7, 2, 3600).unwrap();
    let publish = http_json(
        &base_b,
        "POST",
        "/api/prekey/publish",
        json!({
            "prekey_bundle_text": bob_bundle.to_export_text().unwrap(),
            "signed_one_time_prekey_record_texts": bob_signed_otks
                .iter()
                .map(|record| record.to_export_text().unwrap())
                .collect::<Vec<_>>(),
        }),
    );
    assert_eq!(publish.status, 201, "{}", publish.body);

    // Node A imports node B's snapshot and can then serve Bob's prekey.
    let snapshot_b = http_request(&base_b, "GET", "/api/sync/snapshot", "");
    assert_eq!(snapshot_b.status, 200);
    let snapshot: NodeStateSnapshot = serde_json::from_str(&snapshot_b.body).unwrap();
    let import = http_json(
        &base_a,
        "POST",
        "/api/sync/import",
        json!({ "snapshot": snapshot }),
    );
    assert_eq!(import.status, 200, "{}", import.body);

    let get_prekey = http_request(
        &base_a,
        "GET",
        &format!("/api/prekey/get?user_id={}&consume=true", bob.user_id()),
        "",
    );
    assert_eq!(get_prekey.status, 200, "{}", get_prekey.body);
    let get_body: serde_json::Value = serde_json::from_str(&get_prekey.body).unwrap();
    assert_eq!(get_body["found"], true);
    assert_eq!(get_body["selected_one_time_prekey_id"], 0);
    let selected_record = SignedOneTimePreKeyRecord::from_export_text(
        get_body["selected_signed_one_time_prekey_record_text"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(selected_record.key_id, 0);
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
        "/api/mailbox/push",
        json!({
            "message_text": mailbox.to_export_text().unwrap(),
            "from_identity_public_key": BASE64.encode(alice.identity_public_key()),
        }),
        &[("authorization", "Bearer sync-secret")],
    );
    assert_eq!(push.status, 201, "{}", push.body);

    // Node B imports node A's snapshot and Bob can take + ack the delivery.
    let snapshot_a = http_request(&base_a, "GET", "/api/sync/snapshot", "");
    assert_eq!(snapshot_a.status, 200);
    let snapshot: NodeStateSnapshot = serde_json::from_str(&snapshot_a.body).unwrap();
    let import = http_json(
        &base_b,
        "POST",
        "/api/sync/import",
        json!({ "snapshot": snapshot }),
    );
    assert_eq!(import.status, 200, "{}", import.body);

    let take = http_request(
        &base_b,
        "GET",
        &format!("/api/mailbox/take?user_id={}", bob.user_id()),
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
        "/api/mailbox/ack",
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
        &format!("/api/mailbox/take?user_id={}", bob.user_id()),
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
        "/api/mailbox/push",
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
            &format!("/api/mailbox/take?user_id={}", bob.user_id()),
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

    let sync_status = http_request(&base_b, "GET", "/api/sync/status", "");
    assert_eq!(sync_status.status, 200, "{}", sync_status.body);
    let sync_body: serde_json::Value = serde_json::from_str(&sync_status.body).unwrap();
    let peer_status = &sync_body["peers"][format!("http://{base_a}")];
    assert!(peer_status["successes"].as_u64().unwrap() >= 1);
    assert!(peer_status["last_success_at"].as_u64().is_some());
    assert_eq!(peer_status["last_error"], serde_json::Value::Null);
}

#[test]
fn real_http_control_plane_state_file_recovers_mailbox_push_take_and_ack() {
    let state_file = env::temp_dir().join(format!(
        "lm-node-http-crash-recovery-{}-{}.json",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let (alice, _) = Identity::create_with_passphrase("http recovery alice").unwrap();
    let (bob, _) = Identity::create_with_passphrase("http recovery bob").unwrap();
    let mailbox = MailboxMessage::new(
        &alice,
        bob.user_id().clone(),
        MailboxMessageKind::DirectEnvelope,
        "recover-after-restart".into(),
        3600,
    )
    .unwrap();

    // push 后模拟进程崩溃：state-file 已保存，重启后 delivery 仍可读取。
    let port_push = free_port();
    let base_push = format!("127.0.0.1:{port_push}");
    let mut node = spawn_node_with_state_file(&base_push, "http-recovery-node", &state_file, &[]);
    wait_for_health(&base_push);
    let push = http_json(
        &base_push,
        "POST",
        "/api/mailbox/push",
        json!({
            "message_text": mailbox.to_export_text().unwrap(),
            "from_identity_public_key": BASE64.encode(alice.identity_public_key()),
        }),
    );
    assert_eq!(push.status, 201, "{}", push.body);
    kill_child(&mut node.child);

    let port_take = free_port();
    let base_take = format!("127.0.0.1:{port_take}");
    let mut node = spawn_node_with_state_file(&base_take, "http-recovery-node", &state_file, &[]);
    wait_for_health(&base_take);
    let take = http_request(
        &base_take,
        "GET",
        &format!("/api/mailbox/take?user_id={}", bob.user_id()),
        "",
    );
    assert_eq!(take.status, 200, "{}", take.body);
    let take_body: serde_json::Value = serde_json::from_str(&take.body).unwrap();
    let messages = take_body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0]["message"]["ciphertext"].as_str().unwrap(),
        "recover-after-restart"
    );
    let delivery_id = messages[0]["delivery_id"].as_str().unwrap().to_string();
    kill_child(&mut node.child);

    // take 未 ack 后模拟崩溃：重启后仍可再次 take，避免消息丢失。
    let port_retake = free_port();
    let base_retake = format!("127.0.0.1:{port_retake}");
    let mut node = spawn_node_with_state_file(&base_retake, "http-recovery-node", &state_file, &[]);
    wait_for_health(&base_retake);
    let retake = http_request(
        &base_retake,
        "GET",
        &format!("/api/mailbox/take?user_id={}", bob.user_id()),
        "",
    );
    assert_eq!(retake.status, 200, "{}", retake.body);
    let retake_body: serde_json::Value = serde_json::from_str(&retake.body).unwrap();
    assert_eq!(retake_body["messages"].as_array().unwrap().len(), 1);

    let ack = http_json(
        &base_retake,
        "POST",
        "/api/mailbox/ack",
        json!({
            "user_id": bob.user_id().to_string(),
            "delivery_ids": [delivery_id],
        }),
    );
    assert_eq!(ack.status, 200, "{}", ack.body);
    let ack_body: serde_json::Value = serde_json::from_str(&ack.body).unwrap();
    assert_eq!(ack_body["removed"], 1);
    kill_child(&mut node.child);

    // ack 后模拟崩溃：重启后 delivery 不再出现。
    let port_after_ack = free_port();
    let base_after_ack = format!("127.0.0.1:{port_after_ack}");
    let mut node =
        spawn_node_with_state_file(&base_after_ack, "http-recovery-node", &state_file, &[]);
    wait_for_health(&base_after_ack);
    let after_ack = http_request(
        &base_after_ack,
        "GET",
        &format!("/api/mailbox/take?user_id={}", bob.user_id()),
        "",
    );
    assert_eq!(after_ack.status, 200, "{}", after_ack.body);
    let after_ack_body: serde_json::Value = serde_json::from_str(&after_ack.body).unwrap();
    assert_eq!(after_ack_body["messages"].as_array().unwrap().len(), 0);
    kill_child(&mut node.child);
    let _ = std::fs::remove_file(&state_file);
}

#[test]
fn real_http_control_plane_state_db_recovers_mailbox_push_take_and_ack() {
    let state_db = env::temp_dir().join(format!(
        "lm-node-http-crash-recovery-{}-{}.sqlite3",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let (alice, _) = Identity::create_with_passphrase("http sqlite recovery alice").unwrap();
    let (bob, _) = Identity::create_with_passphrase("http sqlite recovery bob").unwrap();
    let mailbox = MailboxMessage::new(
        &alice,
        bob.user_id().clone(),
        MailboxMessageKind::DirectEnvelope,
        "recover-from-sqlite-after-restart".into(),
        3600,
    )
    .unwrap();

    // 空 state-db 首次启动时应采用 CLI/config 里的 peer_id，并在 push 后持久化 delivery。
    let port_push = free_port();
    let base_push = format!("127.0.0.1:{port_push}");
    let mut node =
        spawn_node_with_state_db(&base_push, "http-sqlite-recovery-node", &state_db, &[]);
    wait_for_health(&base_push);
    let health = http_request(&base_push, "GET", "/api/health", "");
    assert_eq!(health.status, 200, "{}", health.body);
    assert!(health.body.contains("http-sqlite-recovery-node"));
    let push = http_json(
        &base_push,
        "POST",
        "/api/mailbox/push",
        json!({
            "message_text": mailbox.to_export_text().unwrap(),
            "from_identity_public_key": BASE64.encode(alice.identity_public_key()),
        }),
    );
    assert_eq!(push.status, 201, "{}", push.body);
    kill_child(&mut node.child);

    let port_take = free_port();
    let base_take = format!("127.0.0.1:{port_take}");
    let mut node =
        spawn_node_with_state_db(&base_take, "http-sqlite-recovery-node", &state_db, &[]);
    wait_for_health(&base_take);
    let take = http_request(
        &base_take,
        "GET",
        &format!("/api/mailbox/take?user_id={}", bob.user_id()),
        "",
    );
    assert_eq!(take.status, 200, "{}", take.body);
    let take_body: serde_json::Value = serde_json::from_str(&take.body).unwrap();
    let messages = take_body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0]["message"]["ciphertext"].as_str().unwrap(),
        "recover-from-sqlite-after-restart"
    );
    let delivery_id = messages[0]["delivery_id"].as_str().unwrap().to_string();
    kill_child(&mut node.child);

    // take 未 ack 后模拟崩溃：重启后仍可再次 take，避免消息丢失。
    let port_retake = free_port();
    let base_retake = format!("127.0.0.1:{port_retake}");
    let mut node =
        spawn_node_with_state_db(&base_retake, "http-sqlite-recovery-node", &state_db, &[]);
    wait_for_health(&base_retake);
    let retake = http_request(
        &base_retake,
        "GET",
        &format!("/api/mailbox/take?user_id={}", bob.user_id()),
        "",
    );
    assert_eq!(retake.status, 200, "{}", retake.body);
    let retake_body: serde_json::Value = serde_json::from_str(&retake.body).unwrap();
    assert_eq!(retake_body["messages"].as_array().unwrap().len(), 1);

    let ack = http_json(
        &base_retake,
        "POST",
        "/api/mailbox/ack",
        json!({
            "user_id": bob.user_id().to_string(),
            "delivery_ids": [delivery_id],
        }),
    );
    assert_eq!(ack.status, 200, "{}", ack.body);
    let ack_body: serde_json::Value = serde_json::from_str(&ack.body).unwrap();
    assert_eq!(ack_body["removed"], 1);
    kill_child(&mut node.child);

    // ack 后模拟崩溃：重启后 delivery 不再出现。
    let port_after_ack = free_port();
    let base_after_ack = format!("127.0.0.1:{port_after_ack}");
    let mut node =
        spawn_node_with_state_db(&base_after_ack, "http-sqlite-recovery-node", &state_db, &[]);
    wait_for_health(&base_after_ack);
    let after_ack = http_request(
        &base_after_ack,
        "GET",
        &format!("/api/mailbox/take?user_id={}", bob.user_id()),
        "",
    );
    assert_eq!(after_ack.status, 200, "{}", after_ack.body);
    let after_ack_body: serde_json::Value = serde_json::from_str(&after_ack.body).unwrap();
    assert_eq!(after_ack_body["messages"].as_array().unwrap().len(), 0);
    kill_child(&mut node.child);
    cleanup_state_path(&state_db);
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
    restrict_secret_file_permissions(&token_file);
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

    let unauthorized = http_request(&base, "GET", "/api/sync/snapshot", "");
    assert_eq!(unauthorized.status, 401);
    let authorized = http_request_with_headers(
        &base,
        "GET",
        "/api/sync/snapshot",
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
fn real_serve_control_rejects_required_state_db_encryption_until_supported() {
    let port = free_port();
    let base = format!("127.0.0.1:{port}");
    let state_db = env::temp_dir().join(format!(
        "lm-node-http-require-encryption-{}-{}.sqlite3",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let output = Command::new(lm_node_binary())
        .args([
            "serve-control",
            "--bind",
            &base,
            "--peer-id",
            "http-require-encryption-node",
            "--state-db",
        ])
        .arg(&state_db)
        .args(["--state-db-require-encryption", "true"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to execute lm_node serve-control");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("encryption_mode is plain"), "{stderr}");
    cleanup_state_path(&state_db);
}

#[test]
fn real_libp2p_dht_rejects_required_state_db_encryption_until_supported() {
    let state_db = env::temp_dir().join(format!(
        "lm-node-libp2p-require-encryption-{}-{}.sqlite3",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let output = Command::new(lm_node_binary())
        .args([
            "serve-dht-libp2p",
            "--listen",
            "/ip4/127.0.0.1/tcp/0",
            "--peer-id",
            "libp2p-require-encryption-node",
            "--state-db",
        ])
        .arg(&state_db)
        .args(["--state-db-require-encryption", "true"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to execute lm_node serve-dht-libp2p");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("encryption_mode is plain"), "{stderr}");
    cleanup_state_path(&state_db);
}

#[test]
fn real_http_control_plane_rejects_parser_abuse_with_precise_status() {
    let port = free_port();
    let base = format!("127.0.0.1:{port}");
    let _node = spawn_node(&base, "http-parser-hardening-node");
    wait_for_health(&base);

    let body_too_large = raw_http_request(
        &base,
        &format!(
            "POST /api/sync/import HTTP/1.1\r\nhost: {base}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
            4 * 1024 * 1024 + 1
        ),
    );
    assert_eq!(body_too_large.status, 413, "{}", body_too_large.body);
    assert!(body_too_large.body.contains("request body too large"));

    let path_too_large = raw_http_request(
        &base,
        &format!(
            "GET /{} HTTP/1.1\r\nhost: {base}\r\nconnection: close\r\n\r\n",
            "a".repeat(4097)
        ),
    );
    assert_eq!(path_too_large.status, 431, "{}", path_too_large.body);
    assert!(path_too_large.body.contains("request path too large"));

    let conflicting_content_length = raw_http_request(
        &base,
        &format!(
            "POST /api/sync/import HTTP/1.1\r\nhost: {base}\r\ncontent-length: 2\r\ncontent-length: 3\r\nconnection: close\r\n\r\n{{}}",
        ),
    );
    assert_eq!(
        conflicting_content_length.status, 400,
        "{}",
        conflicting_content_length.body
    );
    assert!(
        conflicting_content_length
            .body
            .contains("conflicting content-length")
    );

    let transfer_encoding = raw_http_request(
        &base,
        &format!(
            "POST /api/sync/import HTTP/1.1\r\nhost: {base}\r\ntransfer-encoding: chunked\r\nconnection: close\r\n\r\n0\r\n\r\n",
        ),
    );
    assert_eq!(transfer_encoding.status, 400, "{}", transfer_encoding.body);
    assert!(
        transfer_encoding
            .body
            .contains("unsupported transfer-encoding")
    );
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
    assert_eq!(http_request(&base, "GET", "/api/health", "").status, 200);
    assert_eq!(http_request(&base, "GET", "/api/health", "").status, 200);

    let first = http_request(&base, "GET", "/api/sync/status", "");
    assert_eq!(first.status, 200, "{}", first.body);
    let limited = http_request(&base, "GET", "/api/sync/status", "");
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
        "/api/control/stats",
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
    let baseline_stats_endpoint = baseline["endpoints"]["GET /api/control/stats"]["requests"]
        .as_u64()
        .unwrap_or(0);
    let baseline_sync_export_bytes = baseline["sync_snapshot_export_bytes"].as_u64().unwrap();
    let baseline_sync_exports = baseline["sync_snapshot_exports"].as_u64().unwrap();

    let unauthorized = http_request(&base, "GET", "/api/sync/status", "");
    assert_eq!(unauthorized.status, 401);
    let forbidden_origin = http_request_with_headers(
        &base,
        "GET",
        "/api/sync/status",
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
        "/api/sync/status",
        "",
        &[("authorization", "Bearer stats-secret")],
    );
    assert_eq!(ok.status, 200, "{}", ok.body);
    let snapshot = http_request_with_headers(
        &base,
        "GET",
        "/api/sync/snapshot",
        "",
        &[("authorization", "Bearer stats-secret")],
    );
    assert_eq!(snapshot.status, 200, "{}", snapshot.body);

    let stats = http_request_with_headers(
        &base,
        "GET",
        "/api/control/stats",
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
    let sync_endpoint = &body["endpoints"]["GET /api/sync/status"];
    assert_eq!(sync_endpoint["requests"].as_u64().unwrap(), 3);
    assert_eq!(sync_endpoint["responses_2xx"].as_u64().unwrap(), 1);
    assert_eq!(sync_endpoint["responses_4xx"].as_u64().unwrap(), 2);
    assert!(sync_endpoint["total_duration_micros"].as_u64().is_some());
    assert!(sync_endpoint["max_duration_micros"].as_u64().is_some());
    assert_eq!(sync_endpoint["last_status"].as_u64().unwrap(), 200);
    assert_eq!(
        body["endpoints"]["GET /api/control/stats"]["requests"]
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
        "/api/control/metrics",
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
        "lm_node_control_endpoint_requests_total{endpoint=\"GET /api/sync/status\",class=\"4xx\"}"
    ));
    assert!(metrics.body.contains(
        "lm_node_control_endpoint_duration_micros_total{endpoint=\"GET /api/sync/status\"}"
    ));
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

    let unauthorized = http_request(&base, "GET", "/api/sync/snapshot", "");
    assert_eq!(unauthorized.status, 401);

    let forbidden_origin = http_request_with_headers(
        &base,
        "GET",
        "/api/sync/snapshot",
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
        "/api/sync/snapshot",
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

fn spawn_node_with_state_file(
    bind: &str,
    peer_id: &str,
    state_file: &std::path::Path,
    extra_args: &[&str],
) -> TestNodeProcess {
    let child = Command::new(lm_node_binary())
        .args([
            "serve-control",
            "--bind",
            bind,
            "--peer-id",
            peer_id,
            "--state-file",
        ])
        .arg(state_file)
        .args(extra_args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|err| panic!("failed to spawn lm_node serve-control: {err}"));
    TestNodeProcess {
        child,
        state_file: state_file.to_path_buf(),
    }
}

fn spawn_node_with_state_db(
    bind: &str,
    peer_id: &str,
    state_db: &std::path::Path,
    extra_args: &[&str],
) -> TestNodeProcess {
    let child = Command::new(lm_node_binary())
        .args([
            "serve-control",
            "--bind",
            bind,
            "--peer-id",
            peer_id,
            "--state-db",
        ])
        .arg(state_db)
        .args(extra_args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|err| panic!("failed to spawn lm_node serve-control: {err}"));
    TestNodeProcess {
        child,
        state_file: state_db.to_path_buf(),
    }
}

fn kill_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn cleanup_state_path(path: &std::path::Path) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(path_with_suffix(path, "-wal"));
    let _ = std::fs::remove_file(path_with_suffix(path, "-shm"));
}

fn path_with_suffix(path: &std::path::Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

#[cfg(unix)]
fn restrict_secret_file_permissions(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o600);
    std::fs::set_permissions(path, permissions).unwrap();
}

#[cfg(not(unix))]
fn restrict_secret_file_permissions(_path: &std::path::Path) {}

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
        if let Ok(response) = try_http_request(addr, "GET", "/api/health", "")
            && response.status == 200
        {
            return;
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

fn raw_http_request(addr: &str, request: &str) -> HttpResponse {
    let mut stream = TcpStream::connect(addr).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    parse_http_response(&raw)
}

fn parse_http_response(raw: &[u8]) -> HttpResponse {
    let header_end = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .unwrap_or_else(|| panic!("missing headers in {}", String::from_utf8_lossy(raw)));
    let headers = String::from_utf8_lossy(&raw[..header_end]);
    let status = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or_else(|| panic!("missing status in {headers}"));
    let body = String::from_utf8_lossy(&raw[header_end + 4..]).into_owned();
    HttpResponse { status, body }
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
    Ok(parse_http_response(&raw))
}
