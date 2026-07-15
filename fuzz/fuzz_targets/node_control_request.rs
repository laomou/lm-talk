#![no_main]

use libfuzzer_sys::fuzz_target;
use lm_node::{ControlRequest, NativeNode, NodeConfig};

fuzz_target!(|data: &[u8]| {
    if data.len() < 3 {
        return;
    }
    let first = data[0] as usize;
    let second = data[1] as usize;
    let method_end = 2 + first.min(data.len().saturating_sub(2));
    let path_len = second.min(data.len().saturating_sub(method_end));
    let path_end = method_end + path_len;
    let method = String::from_utf8_lossy(&data[2..method_end]).into_owned();
    let path = String::from_utf8_lossy(&data[method_end..path_end]).into_owned();
    let body = String::from_utf8_lossy(&data[path_end..]).into_owned();
    let request = ControlRequest {
        method,
        path,
        body,
        headers: Vec::new(),
    };
    let mut node = NativeNode::new(NodeConfig::default());
    let response = node.handle_control_request(request);
    let _ = response.status;
    let _ = response.body.len();
});
