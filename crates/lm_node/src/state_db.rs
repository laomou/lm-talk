use super::*;

pub(super) fn open_state_db(path: &str) -> Result<Connection, Box<dyn std::error::Error>> {
    if let Some(parent) = Path::new(path)
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    init_state_db(&conn)?;
    set_state_db_private_permissions(Path::new(path))?;
    Ok(conn)
}

pub(super) fn init_state_db(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = FULL;
        PRAGMA busy_timeout = 5000;
        PRAGMA foreign_keys = ON;
        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS public_peers (
            peer_id TEXT PRIMARY KEY,
            announce_json TEXT NOT NULL,
            routing_peer_json TEXT
        );
        CREATE TABLE IF NOT EXISTS mailbox_deliveries (
            delivery_id TEXT PRIMARY KEY,
            to_user_id TEXT NOT NULL,
            message_id TEXT NOT NULL,
            expires_at INTEGER NOT NULL,
            delivery_json TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_mailbox_deliveries_to_user
            ON mailbox_deliveries(to_user_id);
        CREATE INDEX IF NOT EXISTS idx_mailbox_deliveries_expires_at
            ON mailbox_deliveries(expires_at);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_mailbox_deliveries_to_user_message_id
            ON mailbox_deliveries(to_user_id, message_id);
        CREATE TABLE IF NOT EXISTS mailbox_ack_receipts (
            delivery_id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            acked_at INTEGER NOT NULL,
            expires_at INTEGER NOT NULL,
            receipt_json TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_mailbox_ack_receipts_user_id
            ON mailbox_ack_receipts(user_id);
        CREATE INDEX IF NOT EXISTS idx_mailbox_ack_receipts_expires_at
            ON mailbox_ack_receipts(expires_at);
        CREATE TABLE IF NOT EXISTS prekey_bundles (
            user_id TEXT PRIMARY KEY,
            expires_at INTEGER NOT NULL,
            signed_prekey_expires_at INTEGER NOT NULL,
            bundle_json TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_prekey_bundles_expires_at
            ON prekey_bundles(expires_at);
        CREATE TABLE IF NOT EXISTS signed_one_time_prekey_records (
            user_id TEXT NOT NULL,
            signed_prekey_id INTEGER NOT NULL,
            key_id INTEGER NOT NULL,
            expires_at INTEGER NOT NULL,
            record_json TEXT NOT NULL,
            PRIMARY KEY(user_id, signed_prekey_id, key_id)
        );
        CREATE INDEX IF NOT EXISTS idx_signed_one_time_prekey_records_expires_at
            ON signed_one_time_prekey_records(expires_at);
        CREATE TABLE IF NOT EXISTS consumed_one_time_prekeys (
            user_id TEXT NOT NULL,
            key_id INTEGER NOT NULL,
            PRIMARY KEY(user_id, key_id)
        );
        CREATE TABLE IF NOT EXISTS dht_records (
            record_key TEXT PRIMARY KEY,
            kind TEXT NOT NULL,
            expires_at INTEGER NOT NULL,
            republish_at INTEGER NOT NULL,
            record_json TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_dht_records_expires_at
            ON dht_records(expires_at);
        "#,
    )?;
    ensure_column(
        conn,
        "public_peers",
        "routing_peer_json",
        "ALTER TABLE public_peers ADD COLUMN routing_peer_json TEXT",
    )?;
    Ok(())
}

pub(super) fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    alter_sql: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for existing in columns {
        if existing? == column {
            return Ok(());
        }
    }
    conn.execute(alter_sql, [])?;
    Ok(())
}

pub(super) fn load_node_state_db(path: &str) -> Result<NativeNode, Box<dyn std::error::Error>> {
    let conn = open_state_db(path)?;
    let version = db_get_json::<u16>(&conn, "version")?.unwrap_or(1);
    let config = db_get_json::<NodeConfig>(&conn, "config")?
        .ok_or_else(|| format!("state db has no saved config: {path}"))?;
    let sync_status = db_get_json::<NodeSyncStatus>(&conn, "sync_status")?.unwrap_or_default();
    let maintenance =
        db_get_json::<NodeMaintenanceStats>(&conn, "maintenance")?.unwrap_or_default();
    let public_peers = db_get_all_json(&conn, "SELECT announce_json FROM public_peers")?;
    let routing_peers = db_get_all_json::<RoutingPeer>(
        &conn,
        "SELECT routing_peer_json FROM public_peers WHERE routing_peer_json IS NOT NULL",
    )?;
    let mailbox_deliveries =
        db_get_all_json::<MailboxDelivery>(&conn, "SELECT delivery_json FROM mailbox_deliveries")?;
    let prekey_bundles = db_get_all_json(&conn, "SELECT bundle_json FROM prekey_bundles")?;
    let signed_one_time_prekey_records = db_get_all_json(
        &conn,
        "SELECT record_json FROM signed_one_time_prekey_records",
    )?;
    let consumed_one_time_prekeys = db_get_consumed_prekeys(&conn)?;
    let dht_records = db_get_all_json::<DhtRecord>(&conn, "SELECT record_json FROM dht_records")?;
    Ok(NativeNode::from_state_snapshot(NodeStateSnapshot {
        version,
        config,
        public_peers,
        routing_peers,
        mailbox_deliveries,
        mailbox_ack_receipts: db_get_all_json::<lm_node::MailboxAckReceipt>(
            &conn,
            "SELECT receipt_json FROM mailbox_ack_receipts",
        )?,
        mailbox_messages: Vec::new(),
        prekey_bundles,
        signed_one_time_prekey_records,
        consumed_one_time_prekeys,
        dht_records,
        sync_status,
        maintenance,
    }))
}

pub(super) fn save_node_state_db(
    path: &str,
    node: &NativeNode,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = open_state_db(path)?;
    let snapshot = node.to_state_snapshot();
    let tx = conn.transaction()?;
    db_set_json_tx(&tx, "version", &snapshot.version)?;
    db_set_json_tx(&tx, "config", &snapshot.config)?;
    db_set_json_tx(&tx, "sync_status", &snapshot.sync_status)?;
    db_set_json_tx(&tx, "maintenance", &snapshot.maintenance)?;
    tx.execute("DELETE FROM public_peers", [])?;
    let routing_peers_by_id = snapshot
        .routing_peers
        .iter()
        .map(|peer| (peer.announce.peer_id.as_str(), peer))
        .collect::<HashMap<_, _>>();
    for peer in &snapshot.public_peers {
        let routing_peer_json = routing_peers_by_id
            .get(peer.peer_id.as_str())
            .map(serde_json::to_string)
            .transpose()?;
        tx.execute(
            "INSERT INTO public_peers(peer_id, announce_json, routing_peer_json) VALUES (?1, ?2, ?3)",
            params![peer.peer_id, serde_json::to_string(peer)?, routing_peer_json],
        )?;
    }
    tx.execute("DELETE FROM mailbox_deliveries", [])?;
    for delivery in &snapshot.mailbox_deliveries {
        tx.execute(
            "INSERT INTO mailbox_deliveries(delivery_id, to_user_id, message_id, expires_at, delivery_json)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                delivery.delivery_id,
                delivery.message.to_user_id.to_string(),
                delivery.message.message_id.to_string(),
                delivery.message.expires_at as i64,
                serde_json::to_string(delivery)?,
            ],
        )?;
    }
    tx.execute("DELETE FROM mailbox_ack_receipts", [])?;
    for receipt in &snapshot.mailbox_ack_receipts {
        tx.execute(
            "INSERT INTO mailbox_ack_receipts(delivery_id, user_id, acked_at, expires_at, receipt_json)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                receipt.delivery_id,
                receipt.user_id.to_string(),
                receipt.acked_at as i64,
                receipt.expires_at as i64,
                serde_json::to_string(receipt)?,
            ],
        )?;
    }
    tx.execute("DELETE FROM prekey_bundles", [])?;
    for bundle in &snapshot.prekey_bundles {
        tx.execute(
            "INSERT INTO prekey_bundles(user_id, expires_at, signed_prekey_expires_at, bundle_json)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                bundle.user_id.to_string(),
                bundle.expires_at as i64,
                bundle.signed_prekey_expires_at as i64,
                serde_json::to_string(bundle)?,
            ],
        )?;
    }
    tx.execute("DELETE FROM signed_one_time_prekey_records", [])?;
    for record in &snapshot.signed_one_time_prekey_records {
        tx.execute(
            "INSERT INTO signed_one_time_prekey_records(user_id, signed_prekey_id, key_id, expires_at, record_json)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                record.user_id.to_string(),
                record.signed_prekey_id as i64,
                record.key_id as i64,
                record.expires_at as i64,
                serde_json::to_string(record)?,
            ],
        )?;
    }
    tx.execute("DELETE FROM consumed_one_time_prekeys", [])?;
    for item in &snapshot.consumed_one_time_prekeys {
        tx.execute(
            "INSERT INTO consumed_one_time_prekeys(user_id, key_id) VALUES (?1, ?2)",
            params![item.user_id.to_string(), item.key_id as i64],
        )?;
    }
    tx.execute("DELETE FROM dht_records", [])?;
    for record in &snapshot.dht_records {
        tx.execute(
            "INSERT INTO dht_records(record_key, kind, expires_at, republish_at, record_json)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                record.key.to_hex(),
                format!("{:?}", record.kind),
                record.expires_at as i64,
                record.republish_at as i64,
                serde_json::to_string(record)?,
            ],
        )?;
    }
    tx.commit()?;
    set_state_db_private_permissions(Path::new(path))?;
    Ok(())
}

pub(super) fn query_pragma_u64(
    conn: &Connection,
    pragma: &str,
) -> Result<u64, Box<dyn std::error::Error>> {
    let value = conn.query_row(pragma, [], |row| {
        if let Ok(value) = row.get::<_, i64>(0) {
            return Ok(value as u64);
        }
        let text = row.get::<_, String>(0)?;
        text.trim().parse::<u64>().map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
        })
    })?;
    Ok(value)
}

pub(super) fn state_db_stats(path: &str) -> Result<StateDbStats, Box<dyn std::error::Error>> {
    let conn = open_state_db(path)?;
    let page_count = query_pragma_u64(&conn, "PRAGMA page_count")?;
    let page_size_bytes = query_pragma_u64(&conn, "PRAGMA page_size")?;
    let freelist_count = query_pragma_u64(&conn, "PRAGMA freelist_count")?;
    let file_bytes = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    Ok(StateDbStats {
        page_count,
        page_size_bytes,
        freelist_count,
        file_bytes,
        permissions_hardened: true,
    })
}

pub(super) fn db_get_json<T: DeserializeOwned>(
    conn: &Connection,
    key: &str,
) -> Result<Option<T>, Box<dyn std::error::Error>> {
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()?;
    value
        .map(|value| serde_json::from_str(&value).map_err(Into::into))
        .transpose()
}

pub(super) fn db_set_json_tx<T: Serialize>(
    tx: &rusqlite::Transaction<'_>,
    key: &str,
    value: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    tx.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, serde_json::to_string(value)?],
    )?;
    Ok(())
}

pub(super) fn db_get_all_json<T: DeserializeOwned>(
    conn: &Connection,
    sql: &str,
) -> Result<Vec<T>, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare(sql)?;
    let values = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    values
        .into_iter()
        .map(|value| serde_json::from_str(&value).map_err(Into::into))
        .collect()
}

pub(super) fn db_get_consumed_prekeys(
    conn: &Connection,
) -> Result<Vec<ConsumedOneTimePreKey>, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare(
        "SELECT user_id, key_id FROM consumed_one_time_prekeys ORDER BY user_id, key_id",
    )?;
    let rows = stmt
        .query_map([], |row| {
            let user_id: String = row.get(0)?;
            let key_id: i64 = row.get(1)?;
            Ok((user_id, key_id))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(|(user_id, key_id)| {
            Ok(ConsumedOneTimePreKey {
                user_id: lm_core::UserId::from_raw(user_id)?,
                key_id: key_id as u32,
            })
        })
        .collect()
}
