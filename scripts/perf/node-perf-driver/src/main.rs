use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use lm_core::{Identity, MailboxMessage, MailboxMessageKind, PreKeyBundle};
use lm_node::{DhtRecord, NodeConfig};
use serde_json::{Value, json};
use std::{
    env,
    io::{Read, Write},
    net::TcpStream,
    thread,
    time::{Duration, Instant},
};

#[derive(Clone)]
struct Options {
    target: String,
    scenario: String,
    messages: usize,
    concurrency: usize,
    samples: usize,
    token: Option<String>,
}

fn main() {
    let options = parse_args();
    match options.scenario.as_str() {
        "api" => run_api(&options),
        "chat" => run_chat(&options),
        "mixed" => {
            run_api(&options);
            run_chat(&options);
        }
        other => panic!("unknown --scenario {other}; use api, chat, or mixed"),
    }
}

fn parse_args() -> Options {
    let mut target = String::new();
    let mut scenario = "mixed".to_owned();
    let mut messages = 500;
    let mut concurrency = 8;
    let mut samples = 100;
    let mut token = env::var("LM_NODE_PERF_TOKEN")
        .ok()
        .filter(|v| !v.is_empty());
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--target" => target = next_arg_value(&mut args, &arg),
            "--scenario" => scenario = next_arg_value(&mut args, &arg),
            "--messages" => {
                messages = next_arg_value(&mut args, &arg)
                    .parse()
                    .expect("invalid --messages")
            }
            "--concurrency" => {
                concurrency = next_arg_value(&mut args, &arg)
                    .parse()
                    .expect("invalid --concurrency")
            }
            "--samples" => {
                samples = next_arg_value(&mut args, &arg)
                    .parse()
                    .expect("invalid --samples")
            }
            "--token" => token = Some(next_arg_value(&mut args, &arg)),
            "-h" | "--help" => {
                println!(
                    "node-perf-driver --target http://127.0.0.1:8787 [--token TOKEN] [--scenario api|chat|mixed] [--messages 500] [--concurrency 8] [--samples 100]"
                );
                std::process::exit(0);
            }
            _ => panic!("unknown argument: {arg}"),
        }
    }
    assert!(
        target.starts_with("http://"),
        "--target currently supports http:// only; use node-perf.sh default isolated mode for local measurement"
    );
    assert!(!target.is_empty(), "--target is required");
    Options {
        target: target.trim_end_matches('/').to_owned(),
        scenario,
        messages: messages.max(1),
        concurrency: concurrency.max(1),
        samples: samples.max(1),
        token,
    }
}

fn next_arg_value(args: &mut impl Iterator<Item = String>, option: &str) -> String {
    args.next()
        .unwrap_or_else(|| panic!("{option} requires a value"))
}

fn run_api(options: &Options) {
    let health = samples(options.samples, || {
        assert_ok(request(options, "GET", "/api/health", None), 200);
    });
    let (owner, _) = Identity::create_with_passphrase("lm-node-perf-prekey").unwrap();
    let (bundle, _, otks) =
        PreKeyBundle::new_with_signed_one_time_prekey_records(&owner, 1, 2, 3600).unwrap();
    let started = Instant::now();
    assert_ok(
        request(
            options,
            "POST",
            "/api/prekey/publish",
            Some(
                json!({"prekey_bundle_text": bundle.to_export_text().unwrap(), "signed_one_time_prekey_record_texts": otks.iter().map(|r| r.to_export_text().unwrap()).collect::<Vec<_>>() }),
            ),
        ),
        201,
    );
    let prekey_publish = started.elapsed();
    let prekey_get = samples(options.samples, || {
        assert_ok(
            request(
                options,
                "GET",
                &format!("/api/prekey/get?user_id={}", owner.user_id()),
                None,
            ),
            200,
        );
    });
    let (identity, _) = Identity::create_with_passphrase("lm-node-perf-dht").unwrap();
    let dht_store = (0..options.samples)
        .map(|i| {
            let announce = NodeConfig {
                peer_id: format!("perf-dht-{i}"),
                ..Default::default()
            }
            .create_announce(&identity)
            .unwrap();
            let record =
                DhtRecord::public_peer(&announce, announce.to_export_text().unwrap(), 3600);
            let started = Instant::now();
            assert_ok(
                request(
                    options,
                    "POST",
                    "/api/dht/record",
                    Some(json!({"record": record})),
                ),
                201,
            );
            started.elapsed()
        })
        .collect::<Vec<_>>();
    let snapshot_export = samples(10, || {
        assert_ok(request(options, "GET", "/api/sync/snapshot", None), 200);
    });
    println!(
        "[api]\n  health: {}\n  prekey publish: {}\n  prekey get: {}\n  dht record store: {}\n  snapshot export: {}",
        summary(&health),
        duration(prekey_publish),
        summary(&prekey_get),
        summary(&dht_store),
        summary(&snapshot_export),
    );
}

fn run_chat(options: &Options) {
    let (sender, _) = Identity::create_with_passphrase("lm-node-perf-sender").unwrap();
    let (recipient, _) = Identity::create_with_passphrase("lm-node-perf-recipient").unwrap();
    let public_key = BASE64.encode(sender.identity_public_key());
    let sequential = (0..options.messages)
        .map(|i| push(options, &sender, &recipient, &public_key, i))
        .collect::<Vec<_>>();
    let take_started = Instant::now();
    let body = assert_ok(
        request(
            options,
            "GET",
            &format!("/api/mailbox/take?user_id={}&limit=64", recipient.user_id()),
            None,
        ),
        200,
    );
    let take = take_started.elapsed();
    ack_all(options, recipient.user_id().as_str(), &body);
    drain(options, recipient.user_id().as_str());

    let work = options.messages;
    let workers = options.concurrency.min(work);
    let started = Instant::now();
    let mut handles = Vec::new();
    for worker in 0..workers {
        let o = options.clone();
        let s = sender.clone();
        let r = recipient.clone();
        let key = public_key.clone();
        handles.push(thread::spawn(move || {
            (worker..work)
                .step_by(workers)
                .map(|i| push(&o, &s, &r, &key, 1_000_000 + i))
                .collect::<Vec<_>>()
        }));
    }
    let concurrent = handles
        .into_iter()
        .flat_map(|h| h.join().unwrap())
        .collect::<Vec<_>>();
    let concurrent_total = started.elapsed();
    drain(options, recipient.user_id().as_str());

    let poll_options = options.clone();
    let poll_user = recipient.user_id().to_string();
    let receiver = thread::spawn(move || {
        let response = request(
            &poll_options,
            "GET",
            &format!("/api/mailbox/take?user_id={poll_user}&limit=1&wait_seconds=2"),
            None,
        );
        (Instant::now(), response)
    });
    thread::sleep(Duration::from_millis(30));
    let wake_started = Instant::now();
    push(options, &sender, &recipient, &public_key, 9_999_999);
    let (received, response) = receiver.join().unwrap();
    assert_ok(response, 200);
    drain(options, recipient.user_id().as_str());
    println!(
        "[chat]\n  mailbox push sequential: {}\n  mailbox take batch(64): {}\n  mailbox push concurrent: {}\n  mailbox concurrent total: {} ({:.1} msg/s)\n  long-poll push→take: {}",
        summary(&sequential),
        duration(take),
        summary(&concurrent),
        duration(concurrent_total),
        work as f64 / concurrent_total.as_secs_f64(),
        duration(received.duration_since(wake_started))
    );
}

fn push(
    options: &Options,
    sender: &Identity,
    recipient: &Identity,
    public_key: &str,
    index: usize,
) -> Duration {
    let message = MailboxMessage::new(
        sender,
        recipient.user_id().clone(),
        MailboxMessageKind::DirectEnvelope,
        format!("perf-envelope-{index}"),
        3600,
    )
    .unwrap();
    let started = Instant::now();
    assert_ok(
        request(
            options,
            "POST",
            "/api/mailbox/push",
            Some(
                json!({"message_text": message.to_export_text().unwrap(), "from_identity_public_key": public_key}),
            ),
        ),
        201,
    );
    started.elapsed()
}
fn drain(options: &Options, user_id: &str) {
    loop {
        let body = assert_ok(
            request(
                options,
                "GET",
                &format!("/api/mailbox/take?user_id={user_id}&limit=64"),
                None,
            ),
            200,
        );
        let ids = ids(&body);
        if ids.is_empty() {
            return;
        }
        ack(options, user_id, ids);
    }
}
fn ack_all(options: &Options, user_id: &str, body: &Value) {
    let ids = ids(body);
    if !ids.is_empty() {
        ack(options, user_id, ids);
    }
}
fn ids(body: &Value) -> Vec<String> {
    body["messages"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|m| m["delivery_id"].as_str().map(str::to_owned))
        .collect()
}
fn ack(options: &Options, user_id: &str, delivery_ids: Vec<String>) {
    assert_ok(
        request(
            options,
            "POST",
            "/api/mailbox/ack",
            Some(json!({"user_id": user_id, "delivery_ids": delivery_ids})),
        ),
        200,
    );
}
fn samples(count: usize, mut action: impl FnMut()) -> Vec<Duration> {
    (0..count)
        .map(|_| {
            let started = Instant::now();
            action();
            started.elapsed()
        })
        .collect()
}
fn request(options: &Options, method: &str, path: &str, body: Option<Value>) -> (u16, Value) {
    let addr = options.target.trim_start_matches("http://");
    let text = body.map(|v| v.to_string()).unwrap_or_default();
    let auth = options
        .token
        .as_ref()
        .map(|v| format!("authorization: Bearer {v}\r\n"))
        .unwrap_or_default();
    let mut stream = TcpStream::connect(addr).unwrap_or_else(|e| panic!("connect {addr}: {e}"));
    stream
        .set_read_timeout(Some(Duration::from_secs(15)))
        .unwrap();
    let raw = format!(
        "{method} {path} HTTP/1.1\r\nhost: {addr}\r\ncontent-type: application/json\r\n{auth}content-length: {}\r\nconnection: close\r\n\r\n{text}",
        text.len()
    );
    stream.write_all(raw.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    let (head, body) = response
        .split_once("\r\n\r\n")
        .expect("invalid HTTP response");
    let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
    let value = serde_json::from_str(body).unwrap_or_else(|_| json!({"raw": body}));
    (status, value)
}
fn assert_ok((status, body): (u16, Value), expected: u16) -> Value {
    assert_eq!(status, expected, "{body}");
    body
}
fn duration(value: Duration) -> String {
    format!("{:.2} ms", value.as_secs_f64() * 1_000.0)
}
fn summary(samples: &[Duration]) -> String {
    let mut values = samples
        .iter()
        .map(|d| d.as_micros() as u64)
        .collect::<Vec<_>>();
    values.sort_unstable();
    let n = values.len();
    let p = |percent: usize| values[((n * percent).div_ceil(100)).saturating_sub(1)];
    format!(
        "n={n} avg={:.2} ms p50={:.2} ms p95={:.2} ms p99={:.2} ms max={:.2} ms",
        values.iter().sum::<u64>() as f64 / n as f64 / 1000.0,
        p(50) as f64 / 1000.0,
        p(95) as f64 / 1000.0,
        p(99) as f64 / 1000.0,
        *values.last().unwrap() as f64 / 1000.0
    )
}
