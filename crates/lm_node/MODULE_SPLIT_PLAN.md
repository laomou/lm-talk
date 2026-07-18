# lm_node Module Split Plan

## Current State
- `lib.rs`: 6097 lines (core node logic)
- `main.rs`: 9251 lines (CLI + HTTP server + runners)

## Target Module Structure

### lib.rs → split into:

```
src/
├── lib.rs              (re-exports + NativeNode struct + top-level impl)
├── kademlia.rs         (KademliaNodeId, KademliaDistance, routing table types)
├── dht_record.rs       (DhtRecordKey, DhtRecordKind, DhtRecord, DhtRecordStore)
├── mailbox.rs          (MailboxDelivery, MailboxStore, delivery status, ack receipts)
├── prekey_store.rs     (PreKeyStore, ConsumedOneTimePreKey, bundle management)
├── config.rs           (NodeConfig, reject enums, stats structs, defaults)
├── state.rs            (NodeStateSnapshot, NodeSyncStatus, merge logic)
├── control.rs          (ControlRequest/Response, handle_control_request + handlers)
└── rate_limit.rs       (MailboxRateLimitConfig, sender/global rate limiters)
```

### main.rs → split into:

```
src/
├── main.rs             (CLI dispatch + main())
├── cli.rs              (argument parsing, subcommands)
├── secrets.rs          (secret file read/validate, passphrase handling)
├── state_db.rs         (SQLite open/init/load/save, StateDbEncryptionMode)
├── state_file.rs       (atomic write, file permissions, JSON state file I/O)
├── control_server.rs   (serve_control TCP loop, HTTP parsing, auth, CORS)
├── metrics.rs          (OpenMetrics text rendering, ControlRuntimeStats)
├── dht_transport.rs    (DhtTransport trait, HttpControlDhtTransport)
├── dht_runner.rs       (replication, routing refresh, find-value runners)
├── libp2p_transport.rs (libp2p swarm, behaviour, Libp2pDhtTransport)
├── sync.rs             (SyncPeerConfig, run_snapshot_sync, fetch_snapshot)
└── logging.rs          (LogFormat, ControlLogger)
```

## Extraction Order (by independence, largest first)

1. **`kademlia.rs`** from lib.rs (lines 32-104) — zero internal deps, only blake3
2. **`dht_record.rs`** from lib.rs (lines 106-620) — depends on kademlia + lm_core types
3. **`mailbox.rs`** from lib.rs (lines 780-1242) — depends on lm_core, uuid
4. **`rate_limit.rs`** from lib.rs (lines 681-725, 1243-1386) — standalone
5. **`prekey_store.rs`** from lib.rs (lines 1388-1793) — depends on lm_core
6. **`config.rs`** from lib.rs — depends on above types
7. **`metrics.rs`** from main.rs (lines 1474-3107, ~1535 lines) — the biggest win
8. **`state_db.rs`** from main.rs (lines 701-871, 3300-3671) — standalone
9. **`dht_runner.rs`** from main.rs (lines 4193-4940) — depends on transport trait
10. **`control_server.rs`** from main.rs — the HTTP loop + handlers
11. Remaining helpers and tests stay in place until further refactoring

## Key Constraints

- Must maintain `pub` visibility for test access (tests live in separate files)
- `NativeNode` methods reference types from multiple modules — keep the struct in lib.rs
- main.rs `serve_control` function references nearly everything — extract last
- Tests can stay in their current files initially; split later

## How to Execute

Each extraction:
1. Create `src/<module>.rs`
2. Move types/functions
3. Add `pub mod <module>;` to lib.rs or main.rs
4. Add `pub use <module>::*;` for backwards compatibility
5. Run `cargo test` to verify

Start from leaf modules (no internal deps) and work inward.
