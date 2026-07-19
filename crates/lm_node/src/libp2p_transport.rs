use super::*;
use futures::StreamExt;
use libp2p::{
    Multiaddr, PeerId, StreamProtocol, connection_limits, noise, request_response,
    swarm::NetworkBehaviour, tcp, yamux,
};

#[allow(dead_code)]
const LIBP2P_DHT_RPC_PROTOCOL: &str = "/lm-talk/dht-rpc/1";
const MAX_LIBP2P_DHT_RPC_REQUEST_BYTES: u64 = 1024 * 1024;
const MAX_LIBP2P_DHT_RPC_RESPONSE_BYTES: u64 = MAX_CONTROL_PEER_RESPONSE_BYTES as u64;
const MAX_LIBP2P_DHT_RPC_CONCURRENT_STREAMS: usize = 32;
const MAX_LIBP2P_DHT_PENDING_INCOMING: u32 = 64;
const MAX_LIBP2P_DHT_PENDING_OUTGOING: u32 = 64;
const MAX_LIBP2P_DHT_ESTABLISHED_INCOMING: u32 = 128;
const MAX_LIBP2P_DHT_ESTABLISHED_OUTGOING: u32 = 128;
const MAX_LIBP2P_DHT_ESTABLISHED_TOTAL: u32 = 256;
const MAX_LIBP2P_DHT_ESTABLISHED_PER_PEER: u32 = 4;

#[allow(dead_code)]
type Libp2pDhtRpcBehaviour = request_response::json::Behaviour<DhtRpcRequest, DhtRpcResponse>;

#[allow(dead_code)]
#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "Libp2pDhtEvent")]
pub(super) struct Libp2pDhtBehaviour {
    dht_rpc: Libp2pDhtRpcBehaviour,
    limits: connection_limits::Behaviour,
}

#[allow(dead_code)]
#[derive(Debug)]
pub(super) enum Libp2pDhtEvent {
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

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub(super) struct Libp2pDhtTransport {
    timeout: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(super) struct Libp2pBootstrapPeer {
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

pub(super) fn libp2p_dht_swarm()
-> Result<libp2p::Swarm<Libp2pDhtBehaviour>, Box<dyn std::error::Error>> {
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
                && pending_discovery.remove(request_id)
            {
                let returned = nodes.len();
                let merged = node.merge_verified_routing_peers(nodes.clone());
                println!("libp2p_dht_discovery_nodes={returned} merged={merged}");
                return None;
            }
            handle_libp2p_dht_rpc_event(node, &mut swarm.behaviour_mut().dht_rpc, event)
        }
        _ => None,
    }
}

#[allow(dead_code)]
fn persist_libp2p_dht_state(
    node: &NativeNode,
    state_db: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(path) = state_db {
        save_node_state_db(path, node)?;
    }
    Ok(())
}

#[allow(dead_code)]
pub(super) fn serve_libp2p_dht(
    listen: &str,
    bootstrap_peers: &[Libp2pBootstrapPeer],
    node: &mut NativeNode,
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
                persist_libp2p_dht_state(node, state_db)?;
            }
        }
        #[allow(unreachable_code)]
        Ok::<(), Box<dyn std::error::Error>>(())
    })
}

#[allow(dead_code)]
pub(super) fn parse_libp2p_bootstrap_peers(
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

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::swarm::SwarmEvent;
    use lm_core::Identity;
    use lm_node::{
        DhtRecord, DhtRecordKey, DhtRecordKind, DhtRpcRequest, DhtRpcResponse, NativeNode,
        NodeConfig,
    };
    use std::collections::HashSet;
    use std::time::Duration;

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
                &[Libp2pBootstrapPeer {
                    peer_id: seed_peer,
                    address: seed_addr,
                }],
            )
            .unwrap();
            let connected = tokio::time::timeout(Duration::from_secs(30), async {
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
                &[Libp2pBootstrapPeer {
                    peer_id: seed_peer,
                    address: seed_addr,
                }],
            )
            .unwrap();

            let mut seed_pending = HashSet::new();
            let mut joining_pending = HashSet::new();
            tokio::time::timeout(Duration::from_secs(30), async {
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
            let response = libp2p_dht_roundtrip(
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
}
