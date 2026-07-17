# Fuzzing / 模糊测试

LM Talk provides a `cargo-fuzz`/libFuzzer scaffold for untrusted protocol and node inputs. These harnesses are not a replacement for external security review, but they are intended to be part of the production release gate.

## Prerequisites

```bash
cargo install cargo-fuzz
```

## Smoke check

Before a release candidate, verify that all fuzz harnesses can start and execute at least a small number of inputs:

```bash
./scripts/fuzz-smoke.sh
```

This is only a harness-startup smoke test. It does not replace long-running fuzz campaigns.

## Targets

```bash
# Core text import parsers: contact/friend/backup/prekey/signal/mailbox/message receipt/group/ratchet/file/device revoke
./scripts/fuzz.sh core_imports -- -max_total_time=60

# Native node DHT RPC JSON and snapshot merge inputs
./scripts/fuzz.sh node_dht_rpc -- -max_total_time=60

# Native node control request dispatch with arbitrary method/path/body splits
./scripts/fuzz.sh node_control_request -- -max_total_time=60
```

For longer release candidates, run each target for multiple hours with saved corpora and commit only small, meaningful regression seeds. Any crash or timeout must be triaged before release.

## Current scope

- `core_imports` checks that all public text import parsers, including signed delivery/read receipts, tolerate arbitrary byte input without panics and still rely on their size/signature/format validation.
- `node_dht_rpc` exercises `DhtRpcRequest` deserialization, `NativeNode::handle_dht_rpc`, `DhtRpcResponse` serialization, and `NodeStateSnapshot` merge paths.
- `node_control_request` exercises `NativeNode::handle_control_request` with arbitrary method/path/body strings.

## Remaining release work

- Add persistent corpora from production-like captures with secrets stripped.
- Add CI/nightly fuzz jobs with time budgets and artifact upload for crashes.
- Add lower-level parser/crypto envelope fuzz targets as protocol formats stabilize.
- Run independent AFL/libFuzzer campaigns and external security audit before claiming production readiness.
