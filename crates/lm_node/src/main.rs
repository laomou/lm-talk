use lm_core::PublicPeerAnnounce;
use lm_node::{
    ControlRequest, NativeNode, NodeConfig, NodeStateSnapshot, decode_identity_public_key_base64,
    parse_capabilities_csv, restore_identity_from_backup_text,
};
use std::{
    env, fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    process,
};

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let Some(cmd) = args.next() else {
        print_help();
        return Ok(());
    };

    match cmd.as_str() {
        "announce" => {
            let backup_file = required_arg(&mut args, "--backup-file")?;
            let passphrase = required_arg(&mut args, "--passphrase")?;
            let peer_id = optional_arg(&mut args, "--peer-id")?.unwrap_or("lm-node".into());
            let addresses = optional_arg(&mut args, "--addr")?
                .map(|value| value.split(',').map(str::to_string).collect())
                .unwrap_or_else(|| vec!["/ip4/0.0.0.0/tcp/4001".to_string()]);
            let capabilities = optional_arg(&mut args, "--cap")?
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
            let text_file = required_arg(&mut args, "--text-file")?;
            let public_key = required_arg(&mut args, "--identity-public-key")?;
            let text = fs::read_to_string(text_file)?;
            let announce = PublicPeerAnnounce::from_export_text(text.trim())?;
            let pk = decode_identity_public_key_base64(&public_key)?;
            announce.verify(&pk)?;
            println!("{}", serde_json::to_string_pretty(&announce)?);
        }
        "run" => {
            let peer_id = optional_arg(&mut args, "--peer-id")?.unwrap_or("lm-node-dev".into());
            let addr = optional_arg(&mut args, "--addr")?.unwrap_or("/ip4/0.0.0.0/tcp/4001".into());
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
            let a = required_arg(&mut args, "--a")?;
            let b = required_arg(&mut args, "--b")?;
            let a_id = lm_node::KademliaNodeId::from_peer_id(&a);
            let b_id = lm_node::KademliaNodeId::from_peer_id(&b);
            let distance = a_id.xor_distance(&b_id);
            println!("a_id={a_id}");
            println!("b_id={b_id}");
            println!("distance={}", distance.to_hex());
            println!("bucket_index={:?}", a_id.bucket_index(&b_id));
        }
        "serve-control" => {
            let bind = optional_arg(&mut args, "--bind")?.unwrap_or("127.0.0.1:8787".into());
            let peer_id = optional_arg(&mut args, "--peer-id")?.unwrap_or("lm-node-dev".into());
            let state_file = optional_arg(&mut args, "--state-file")?;
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
            serve_control(&bind, &mut node, state_file.as_deref())?;
        }
        "help" | "--help" | "-h" => print_help(),
        _ => {
            print_help();
            return Err(format!("unknown command: {cmd}").into());
        }
    }
    Ok(())
}

fn required_arg(args: &mut impl Iterator<Item = String>, name: &str) -> Result<String, String> {
    optional_arg(args, name)?.ok_or_else(|| format!("missing {name}"))
}

fn optional_arg(
    args: &mut impl Iterator<Item = String>,
    name: &str,
) -> Result<Option<String>, String> {
    let mut rest = Vec::new();
    let mut found = None;
    while let Some(arg) = args.next() {
        if arg == name {
            found = args.next();
            break;
        }
        rest.push(arg);
    }
    Ok(found)
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
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(bind)?;
    println!("LM Talk control plane listening on http://{bind}");
    println!(
        "endpoints: GET /health, POST /announce, GET /peers/closest, POST /mailbox/push, GET /mailbox/take, POST /mailbox/ack, POST /prekey/publish, GET /prekey/get, GET /sync/snapshot, POST /sync/import"
    );
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                if let Err(err) = handle_stream(&mut stream, node) {
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
            Err(err) => eprintln!("connection error: {err}"),
        }
    }
    Ok(())
}

fn handle_stream(
    stream: &mut TcpStream,
    node: &mut NativeNode,
) -> Result<(), Box<dyn std::error::Error>> {
    let request = read_http_request(stream)?;
    let response = node.handle_control_request(request);
    stream.write_all(response.to_http_string().as_bytes())?;
    Ok(())
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
    let headers = String::from_utf8_lossy(&buffer[..header_end]);
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
    Ok(ControlRequest { method, path, body })
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
run [--peer-id <id>] [--addr <multiaddr>]\n"
    );
}
