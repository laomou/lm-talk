#![no_main]

use libfuzzer_sys::fuzz_target;
use lm_node::{DhtRpcRequest, NativeNode, NodeConfig, NodeStateSnapshot};

fuzz_target!(|data: &[u8]| {
    if let Ok(request) = serde_json::from_slice::<DhtRpcRequest>(data) {
        let mut node = NativeNode::new(NodeConfig::default());
        let response = node.handle_dht_rpc(request);
        let _ = serde_json::to_vec(&response);
    }
    if let Ok(snapshot) = serde_json::from_slice::<NodeStateSnapshot>(data) {
        let mut node = NativeNode::new(NodeConfig::default());
        let _ = node.merge_snapshot(snapshot);
    }
});
