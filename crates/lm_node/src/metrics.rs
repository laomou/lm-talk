use super::*;

#[derive(Debug, Serialize)]
pub(super) struct ControlStatsResponse<'a> {
    #[serde(flatten)]
    pub(crate) runtime: &'a ControlRuntimeStats,
    pub(crate) maintenance: NodeMaintenanceStats,
    pub(crate) state_db: Option<StateDbStats>,
    pub(crate) state_file: Option<StateFileStats>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(super) struct StateFileStats {
    pub(crate) file_bytes: u64,
    pub(crate) encrypted: bool,
    pub(crate) permissions_hardened: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(super) struct StateDbStats {
    pub(crate) page_count: u64,
    pub(crate) page_size_bytes: u64,
    pub(crate) freelist_count: u64,
    pub(crate) file_bytes: u64,
    pub(crate) encrypted: bool,
    pub(crate) encryption_mode: String,
    pub(crate) permissions_hardened: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(super) struct ControlRuntimeStats {
    pub(crate) started_at: u64,
    pub(crate) requests_total: u64,
    pub(crate) responses_2xx: u64,
    pub(crate) responses_4xx: u64,
    pub(crate) responses_5xx: u64,
    pub(crate) cors_rejected: u64,
    pub(crate) unauthorized: u64,
    pub(crate) rate_limited: u64,
    pub(crate) bad_requests: u64,
    pub(crate) sync_snapshot_exports: u64,
    pub(crate) sync_snapshot_export_bytes: u64,
    pub(crate) sync_snapshot_imports: u64,
    pub(crate) sync_snapshot_import_bytes: u64,
    pub(crate) dht_replication_runs: u64,
    pub(crate) dht_replication_records: u64,
    pub(crate) dht_replication_attempts: u64,
    pub(crate) dht_replication_successes: u64,
    pub(crate) dht_replication_failures: u64,
    pub(crate) dht_replication_peers_quarantined: u64,
    pub(crate) last_dht_replication_at: Option<u64>,
    pub(crate) dht_routing_refresh_runs: u64,
    pub(crate) dht_routing_refresh_targets: u64,
    pub(crate) dht_routing_refresh_attempts: u64,
    pub(crate) dht_routing_refresh_successes: u64,
    pub(crate) dht_routing_refresh_failures: u64,
    pub(crate) dht_routing_refresh_nodes_returned: u64,
    pub(crate) dht_routing_refresh_nodes_merged: u64,
    pub(crate) dht_routing_refresh_nodes_rejected_non_closer: u64,
    pub(crate) dht_routing_refresh_nodes_rejected_duplicate: u64,
    pub(crate) dht_routing_refresh_peers_quarantined: u64,
    pub(crate) last_dht_routing_refresh_at: Option<u64>,
    pub(crate) dht_find_value_runs: u64,
    pub(crate) dht_find_value_attempts: u64,
    pub(crate) dht_find_value_successes: u64,
    pub(crate) dht_find_value_failures: u64,
    pub(crate) dht_find_value_found_records: u64,
    pub(crate) dht_find_value_invalid_found_records: u64,
    pub(crate) dht_find_value_closer_records: u64,
    pub(crate) dht_find_value_closer_nodes_returned: u64,
    pub(crate) dht_find_value_closer_nodes_merged: u64,
    pub(crate) dht_find_value_closer_nodes_rejected_non_closer: u64,
    pub(crate) dht_find_value_closer_nodes_rejected_duplicate: u64,
    pub(crate) dht_find_value_peers_quarantined: u64,
    pub(crate) dht_find_value_query_rounds: u64,
    pub(crate) dht_find_value_alpha_max: usize,
    pub(crate) dht_find_value_exhausted: u64,
    pub(crate) last_dht_find_value_at: Option<u64>,
    pub(crate) sync_schedule_delay_micros_total: u128,
    pub(crate) sync_schedule_delay_micros_max: u128,
    pub(crate) last_sync_schedule_delay_micros: Option<u128>,
    pub(crate) dht_replication_schedule_delay_micros_total: u128,
    pub(crate) dht_replication_schedule_delay_micros_max: u128,
    pub(crate) last_dht_replication_schedule_delay_micros: Option<u128>,
    pub(crate) dht_routing_refresh_schedule_delay_micros_total: u128,
    pub(crate) dht_routing_refresh_schedule_delay_micros_max: u128,
    pub(crate) last_dht_routing_refresh_schedule_delay_micros: Option<u128>,
    pub(crate) endpoints: HashMap<String, ControlEndpointStats>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub(super) struct ControlEndpointStats {
    pub(crate) requests: u64,
    pub(crate) responses_2xx: u64,
    pub(crate) responses_4xx: u64,
    pub(crate) responses_5xx: u64,
    pub(crate) total_duration_micros: u128,
    pub(crate) max_duration_micros: u128,
    pub(crate) last_status: Option<u16>,
}

impl ControlRuntimeStats {
    pub(crate) fn new(started_at: u64) -> Self {
        Self {
            started_at,
            requests_total: 0,
            responses_2xx: 0,
            responses_4xx: 0,
            responses_5xx: 0,
            cors_rejected: 0,
            unauthorized: 0,
            rate_limited: 0,
            bad_requests: 0,
            sync_snapshot_exports: 0,
            sync_snapshot_export_bytes: 0,
            sync_snapshot_imports: 0,
            sync_snapshot_import_bytes: 0,
            dht_replication_runs: 0,
            dht_replication_records: 0,
            dht_replication_attempts: 0,
            dht_replication_successes: 0,
            dht_replication_failures: 0,
            dht_replication_peers_quarantined: 0,
            last_dht_replication_at: None,
            dht_routing_refresh_runs: 0,
            dht_routing_refresh_targets: 0,
            dht_routing_refresh_attempts: 0,
            dht_routing_refresh_successes: 0,
            dht_routing_refresh_failures: 0,
            dht_routing_refresh_nodes_returned: 0,
            dht_routing_refresh_nodes_merged: 0,
            dht_routing_refresh_nodes_rejected_non_closer: 0,
            dht_routing_refresh_nodes_rejected_duplicate: 0,
            dht_routing_refresh_peers_quarantined: 0,
            last_dht_routing_refresh_at: None,
            dht_find_value_runs: 0,
            dht_find_value_attempts: 0,
            dht_find_value_successes: 0,
            dht_find_value_failures: 0,
            dht_find_value_found_records: 0,
            dht_find_value_invalid_found_records: 0,
            dht_find_value_closer_records: 0,
            dht_find_value_closer_nodes_returned: 0,
            dht_find_value_closer_nodes_merged: 0,
            dht_find_value_closer_nodes_rejected_non_closer: 0,
            dht_find_value_closer_nodes_rejected_duplicate: 0,
            dht_find_value_peers_quarantined: 0,
            dht_find_value_query_rounds: 0,
            dht_find_value_alpha_max: 0,
            dht_find_value_exhausted: 0,
            last_dht_find_value_at: None,
            sync_schedule_delay_micros_total: 0,
            sync_schedule_delay_micros_max: 0,
            last_sync_schedule_delay_micros: None,
            dht_replication_schedule_delay_micros_total: 0,
            dht_replication_schedule_delay_micros_max: 0,
            last_dht_replication_schedule_delay_micros: None,
            dht_routing_refresh_schedule_delay_micros_total: 0,
            dht_routing_refresh_schedule_delay_micros_max: 0,
            last_dht_routing_refresh_schedule_delay_micros: None,
            endpoints: HashMap::new(),
        }
    }

    pub(crate) fn to_openmetrics(
        &self,
        maintenance: &NodeMaintenanceStats,
        state_db: Option<&StateDbStats>,
        state_file: Option<&StateFileStats>,
        sync_status: Option<&NodeSyncStatus>,
        dht_peer_quarantine_threshold: u32,
    ) -> String {
        let mut out = String::new();
        push_metric_help(
            &mut out,
            "lm_node_control_started_at",
            "Unix timestamp when the control runtime started.",
        );
        push_metric_type(&mut out, "lm_node_control_started_at", "gauge");
        push_metric_value(&mut out, "lm_node_control_started_at", self.started_at);
        push_metric_help(
            &mut out,
            "lm_node_control_requests_total",
            "Total control HTTP responses by status class and security outcome.",
        );
        push_metric_type(&mut out, "lm_node_control_requests_total", "counter");
        push_metric_value(
            &mut out,
            "lm_node_control_requests_total",
            self.requests_total,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_control_responses_total",
            "class",
            "2xx",
            self.responses_2xx,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_control_responses_total",
            "class",
            "4xx",
            self.responses_4xx,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_control_responses_total",
            "class",
            "5xx",
            self.responses_5xx,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_control_security_events_total",
            "event",
            "bad_request",
            self.bad_requests,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_control_security_events_total",
            "event",
            "unauthorized",
            self.unauthorized,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_control_security_events_total",
            "event",
            "cors_rejected",
            self.cors_rejected,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_control_security_events_total",
            "event",
            "rate_limited",
            self.rate_limited,
        );
        push_metric_help(
            &mut out,
            "lm_node_control_sync_snapshots_total",
            "Successful snapshot sync import/export operations through the control plane.",
        );
        push_metric_type(&mut out, "lm_node_control_sync_snapshots_total", "counter");
        push_labeled_metric_value(
            &mut out,
            "lm_node_control_sync_snapshots_total",
            "direction",
            "export",
            self.sync_snapshot_exports,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_control_sync_snapshots_total",
            "direction",
            "import",
            self.sync_snapshot_imports,
        );
        push_metric_help(
            &mut out,
            "lm_node_control_sync_snapshot_bytes_total",
            "Snapshot sync import/export payload bytes through the control plane.",
        );
        push_metric_type(
            &mut out,
            "lm_node_control_sync_snapshot_bytes_total",
            "counter",
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_control_sync_snapshot_bytes_total",
            "direction",
            "export",
            self.sync_snapshot_export_bytes,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_control_sync_snapshot_bytes_total",
            "direction",
            "import",
            self.sync_snapshot_import_bytes,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_replication_runs_total",
            "Total DHT control-peer replication runner executions.",
        );
        push_metric_type(&mut out, "lm_node_dht_replication_runs_total", "counter");
        push_metric_value(
            &mut out,
            "lm_node_dht_replication_runs_total",
            self.dht_replication_runs,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_replication_records_total",
            "DHT records included in replication runner plans.",
        );
        push_metric_type(&mut out, "lm_node_dht_replication_records_total", "counter");
        push_metric_value(
            &mut out,
            "lm_node_dht_replication_records_total",
            self.dht_replication_records,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_replication_attempts_total",
            "DHT StoreRecord replication attempts by result.",
        );
        push_metric_type(
            &mut out,
            "lm_node_dht_replication_attempts_total",
            "counter",
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_replication_attempts_total",
            "result",
            "success",
            self.dht_replication_successes,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_replication_attempts_total",
            "result",
            "failure",
            self.dht_replication_failures,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_replication_attempts_total",
            "result",
            "all",
            self.dht_replication_attempts,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_replication_peers_quarantined_total",
            "DHT replication peers skipped by quarantine.",
        );
        push_metric_type(
            &mut out,
            "lm_node_dht_replication_peers_quarantined_total",
            "counter",
        );
        push_metric_value(
            &mut out,
            "lm_node_dht_replication_peers_quarantined_total",
            self.dht_replication_peers_quarantined,
        );
        if let Some(last_dht_replication_at) = self.last_dht_replication_at {
            push_metric_help(
                &mut out,
                "lm_node_dht_replication_last_run_at",
                "Unix timestamp of the last DHT replication runner execution.",
            );
            push_metric_type(&mut out, "lm_node_dht_replication_last_run_at", "gauge");
            push_metric_value(
                &mut out,
                "lm_node_dht_replication_last_run_at",
                last_dht_replication_at,
            );
        }
        push_metric_help(
            &mut out,
            "lm_node_dht_routing_refresh_runs_total",
            "Total DHT routing refresh runner executions.",
        );
        push_metric_type(
            &mut out,
            "lm_node_dht_routing_refresh_runs_total",
            "counter",
        );
        push_metric_value(
            &mut out,
            "lm_node_dht_routing_refresh_runs_total",
            self.dht_routing_refresh_runs,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_routing_refresh_targets_total",
            "DHT routing refresh targets queried by the runner.",
        );
        push_metric_type(
            &mut out,
            "lm_node_dht_routing_refresh_targets_total",
            "counter",
        );
        push_metric_value(
            &mut out,
            "lm_node_dht_routing_refresh_targets_total",
            self.dht_routing_refresh_targets,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_routing_refresh_attempts_total",
            "DHT FindNode routing refresh attempts by result.",
        );
        push_metric_type(
            &mut out,
            "lm_node_dht_routing_refresh_attempts_total",
            "counter",
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_routing_refresh_attempts_total",
            "result",
            "success",
            self.dht_routing_refresh_successes,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_routing_refresh_attempts_total",
            "result",
            "failure",
            self.dht_routing_refresh_failures,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_routing_refresh_attempts_total",
            "result",
            "all",
            self.dht_routing_refresh_attempts,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_routing_refresh_nodes_returned_total",
            "Routing peers returned by DHT FindNode refresh responses.",
        );
        push_metric_type(
            &mut out,
            "lm_node_dht_routing_refresh_nodes_returned_total",
            "counter",
        );
        push_metric_value(
            &mut out,
            "lm_node_dht_routing_refresh_nodes_returned_total",
            self.dht_routing_refresh_nodes_returned,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_routing_refresh_nodes_merged_total",
            "Trusted routing peers merged from DHT FindNode refresh responses.",
        );
        push_metric_type(
            &mut out,
            "lm_node_dht_routing_refresh_nodes_merged_total",
            "counter",
        );
        push_metric_value(
            &mut out,
            "lm_node_dht_routing_refresh_nodes_merged_total",
            self.dht_routing_refresh_nodes_merged,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_routing_refresh_nodes_rejected_total",
            "Routing peers rejected from DHT FindNode refresh responses before merge.",
        );
        push_metric_type(
            &mut out,
            "lm_node_dht_routing_refresh_nodes_rejected_total",
            "counter",
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_routing_refresh_nodes_rejected_total",
            "reason",
            "non_closer",
            self.dht_routing_refresh_nodes_rejected_non_closer,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_routing_refresh_nodes_rejected_total",
            "reason",
            "duplicate",
            self.dht_routing_refresh_nodes_rejected_duplicate,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_routing_refresh_peers_quarantined_total",
            "DHT routing refresh peers skipped by quarantine.",
        );
        push_metric_type(
            &mut out,
            "lm_node_dht_routing_refresh_peers_quarantined_total",
            "counter",
        );
        push_metric_value(
            &mut out,
            "lm_node_dht_routing_refresh_peers_quarantined_total",
            self.dht_routing_refresh_peers_quarantined,
        );
        if let Some(last_dht_routing_refresh_at) = self.last_dht_routing_refresh_at {
            push_metric_help(
                &mut out,
                "lm_node_dht_routing_refresh_last_run_at",
                "Unix timestamp of the last DHT routing refresh runner execution.",
            );
            push_metric_type(&mut out, "lm_node_dht_routing_refresh_last_run_at", "gauge");
            push_metric_value(
                &mut out,
                "lm_node_dht_routing_refresh_last_run_at",
                last_dht_routing_refresh_at,
            );
        }
        push_metric_help(
            &mut out,
            "lm_node_dht_find_value_runs_total",
            "Total manual DHT FindValue runner executions.",
        );
        push_metric_type(&mut out, "lm_node_dht_find_value_runs_total", "counter");
        push_metric_value(
            &mut out,
            "lm_node_dht_find_value_runs_total",
            self.dht_find_value_runs,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_find_value_attempts_total",
            "Manual DHT FindValue attempts by result.",
        );
        push_metric_type(&mut out, "lm_node_dht_find_value_attempts_total", "counter");
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_find_value_attempts_total",
            "result",
            "success",
            self.dht_find_value_successes,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_find_value_attempts_total",
            "result",
            "failure",
            self.dht_find_value_failures,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_find_value_attempts_total",
            "result",
            "all",
            self.dht_find_value_attempts,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_find_value_records_total",
            "Manual DHT FindValue records merged by kind.",
        );
        push_metric_type(&mut out, "lm_node_dht_find_value_records_total", "counter");
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_find_value_records_total",
            "kind",
            "found",
            self.dht_find_value_found_records,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_find_value_records_total",
            "kind",
            "invalid_found",
            self.dht_find_value_invalid_found_records,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_find_value_records_total",
            "kind",
            "closer",
            self.dht_find_value_closer_records,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_find_value_closer_nodes_total",
            "Manual DHT FindValue closer nodes returned and merged.",
        );
        push_metric_type(
            &mut out,
            "lm_node_dht_find_value_closer_nodes_total",
            "counter",
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_find_value_closer_nodes_total",
            "kind",
            "returned",
            self.dht_find_value_closer_nodes_returned,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_find_value_closer_nodes_total",
            "kind",
            "merged",
            self.dht_find_value_closer_nodes_merged,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_find_value_closer_nodes_total",
            "kind",
            "rejected_non_closer",
            self.dht_find_value_closer_nodes_rejected_non_closer,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_find_value_closer_nodes_total",
            "kind",
            "rejected_duplicate",
            self.dht_find_value_closer_nodes_rejected_duplicate,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_find_value_peers_quarantined_total",
            "Manual DHT FindValue peers skipped by quarantine.",
        );
        push_metric_type(
            &mut out,
            "lm_node_dht_find_value_peers_quarantined_total",
            "counter",
        );
        push_metric_value(
            &mut out,
            "lm_node_dht_find_value_peers_quarantined_total",
            self.dht_find_value_peers_quarantined,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_find_value_query_rounds_total",
            "Total query rounds used by manual DHT FindValue runs.",
        );
        push_metric_type(
            &mut out,
            "lm_node_dht_find_value_query_rounds_total",
            "counter",
        );
        push_metric_value(
            &mut out,
            "lm_node_dht_find_value_query_rounds_total",
            self.dht_find_value_query_rounds,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_find_value_alpha_max",
            "Maximum alpha requested by manual DHT FindValue runs.",
        );
        push_metric_type(&mut out, "lm_node_dht_find_value_alpha_max", "gauge");
        push_metric_value(
            &mut out,
            "lm_node_dht_find_value_alpha_max",
            self.dht_find_value_alpha_max,
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_find_value_exhausted_total",
            "Manual DHT FindValue runs that exhausted candidates without finding a record.",
        );
        push_metric_type(
            &mut out,
            "lm_node_dht_find_value_exhausted_total",
            "counter",
        );
        push_metric_value(
            &mut out,
            "lm_node_dht_find_value_exhausted_total",
            self.dht_find_value_exhausted,
        );
        if let Some(last_dht_find_value_at) = self.last_dht_find_value_at {
            push_metric_help(
                &mut out,
                "lm_node_dht_find_value_last_run_at",
                "Unix timestamp of the last manual DHT FindValue execution.",
            );
            push_metric_type(&mut out, "lm_node_dht_find_value_last_run_at", "gauge");
            push_metric_value(
                &mut out,
                "lm_node_dht_find_value_last_run_at",
                last_dht_find_value_at,
            );
        }
        push_metric_help(
            &mut out,
            "lm_node_background_schedule_delay_micros_total",
            "Total scheduler delay in microseconds for background jobs.",
        );
        push_metric_type(
            &mut out,
            "lm_node_background_schedule_delay_micros_total",
            "counter",
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_background_schedule_delay_micros_total",
            "job",
            "snapshot_sync",
            self.sync_schedule_delay_micros_total,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_background_schedule_delay_micros_total",
            "job",
            "dht_replication",
            self.dht_replication_schedule_delay_micros_total,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_background_schedule_delay_micros_total",
            "job",
            "dht_routing_refresh",
            self.dht_routing_refresh_schedule_delay_micros_total,
        );
        push_metric_help(
            &mut out,
            "lm_node_background_schedule_delay_micros_max",
            "Maximum scheduler delay in microseconds for background jobs.",
        );
        push_metric_type(
            &mut out,
            "lm_node_background_schedule_delay_micros_max",
            "gauge",
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_background_schedule_delay_micros_max",
            "job",
            "snapshot_sync",
            self.sync_schedule_delay_micros_max,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_background_schedule_delay_micros_max",
            "job",
            "dht_replication",
            self.dht_replication_schedule_delay_micros_max,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_background_schedule_delay_micros_max",
            "job",
            "dht_routing_refresh",
            self.dht_routing_refresh_schedule_delay_micros_max,
        );
        push_metric_help(
            &mut out,
            "lm_node_background_schedule_delay_micros_last",
            "Last scheduler delay in microseconds for background jobs.",
        );
        push_metric_type(
            &mut out,
            "lm_node_background_schedule_delay_micros_last",
            "gauge",
        );
        if let Some(value) = self.last_sync_schedule_delay_micros {
            push_labeled_metric_value(
                &mut out,
                "lm_node_background_schedule_delay_micros_last",
                "job",
                "snapshot_sync",
                value,
            );
        }
        if let Some(value) = self.last_dht_replication_schedule_delay_micros {
            push_labeled_metric_value(
                &mut out,
                "lm_node_background_schedule_delay_micros_last",
                "job",
                "dht_replication",
                value,
            );
        }
        if let Some(value) = self.last_dht_routing_refresh_schedule_delay_micros {
            push_labeled_metric_value(
                &mut out,
                "lm_node_background_schedule_delay_micros_last",
                "job",
                "dht_routing_refresh",
                value,
            );
        }
        if let Some(sync_status) = sync_status {
            push_sync_status_metrics(&mut out, sync_status, dht_peer_quarantine_threshold);
        }
        push_metric_help(
            &mut out,
            "lm_node_maintenance_prune_runs_total",
            "Total node expired-record prune runs.",
        );
        push_metric_type(&mut out, "lm_node_maintenance_prune_runs_total", "counter");
        push_metric_value(
            &mut out,
            "lm_node_maintenance_prune_runs_total",
            maintenance.prune_runs,
        );
        push_metric_help(
            &mut out,
            "lm_node_maintenance_expired_records_total",
            "Expired records removed by node maintenance prune jobs.",
        );
        push_metric_type(
            &mut out,
            "lm_node_maintenance_expired_records_total",
            "counter",
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_maintenance_expired_records_total",
            "kind",
            "mailbox_delivery",
            maintenance.mailbox_expired_deliveries,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_maintenance_expired_records_total",
            "kind",
            "prekey_bundle",
            maintenance.prekey_expired_bundles,
        );
        push_metric_help(
            &mut out,
            "lm_node_mailbox_push_rejections_total",
            "Rejected mailbox push attempts by reason.",
        );
        push_metric_type(&mut out, "lm_node_mailbox_push_rejections_total", "counter");
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_push_rejections_total",
            "reason",
            "invalid_json",
            maintenance.mailbox_push_rejects.invalid_json,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_push_rejections_total",
            "reason",
            "invalid_message_format",
            maintenance.mailbox_push_rejects.invalid_message_format,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_push_rejections_total",
            "reason",
            "invalid_identity_public_key",
            maintenance.mailbox_push_rejects.invalid_identity_public_key,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_push_rejections_total",
            "reason",
            "invalid_signature",
            maintenance.mailbox_push_rejects.invalid_signature,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_push_rejections_total",
            "reason",
            "expired_object",
            maintenance.mailbox_push_rejects.expired_object,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_push_rejections_total",
            "reason",
            "duplicate_message",
            maintenance.mailbox_push_rejects.duplicate_message,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_push_rejections_total",
            "reason",
            "payload_too_large",
            maintenance.mailbox_push_rejects.payload_too_large,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_push_rejections_total",
            "reason",
            "global_rate_limited",
            maintenance.mailbox_push_rejects.global_rate_limited,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_push_rejections_total",
            "reason",
            "sender_rate_limited",
            maintenance.mailbox_push_rejects.sender_rate_limited,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_push_rejections_total",
            "reason",
            "other",
            maintenance.mailbox_push_rejects.other,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_push_rejections_total",
            "reason",
            "all",
            maintenance.mailbox_push_rejects.total(),
        );
        push_metric_help(
            &mut out,
            "lm_node_dht_record_rejections_total",
            "Rejected DHT record store attempts by reason.",
        );
        push_metric_type(&mut out, "lm_node_dht_record_rejections_total", "counter");
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_record_rejections_total",
            "reason",
            "invalid_json",
            maintenance.dht_record_rejects.invalid_json,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_record_rejections_total",
            "reason",
            "expired",
            maintenance.dht_record_rejects.expired,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_record_rejections_total",
            "reason",
            "ttl_too_long",
            maintenance.dht_record_rejects.ttl_too_long,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_record_rejections_total",
            "reason",
            "payload_too_large",
            maintenance.dht_record_rejects.payload_too_large,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_record_rejections_total",
            "reason",
            "invalid_record",
            maintenance.dht_record_rejects.invalid_record,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_dht_record_rejections_total",
            "reason",
            "all",
            maintenance.dht_record_rejects.total(),
        );
        push_metric_help(
            &mut out,
            "lm_node_mailbox_ack_rejections_total",
            "Rejected mailbox ack attempts by reason.",
        );
        push_metric_type(&mut out, "lm_node_mailbox_ack_rejections_total", "counter");
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_ack_rejections_total",
            "reason",
            "invalid_json",
            maintenance.mailbox_ack_rejects.invalid_json,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_ack_rejections_total",
            "reason",
            "invalid_user_id",
            maintenance.mailbox_ack_rejects.invalid_user_id,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_ack_rejections_total",
            "reason",
            "too_many_ids",
            maintenance.mailbox_ack_rejects.too_many_ids,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_ack_rejections_total",
            "reason",
            "empty_id",
            maintenance.mailbox_ack_rejects.empty_id,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_ack_rejections_total",
            "reason",
            "id_too_large",
            maintenance.mailbox_ack_rejects.id_too_large,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_mailbox_ack_rejections_total",
            "reason",
            "all",
            maintenance.mailbox_ack_rejects.total(),
        );
        push_metric_help(
            &mut out,
            "lm_node_routing_peer_rejections_total",
            "Rejected DHT routing peer merge attempts by reason.",
        );
        push_metric_type(&mut out, "lm_node_routing_peer_rejections_total", "counter");
        push_labeled_metric_value(
            &mut out,
            "lm_node_routing_peer_rejections_total",
            "reason",
            "expired",
            maintenance.routing_peer_rejects.expired,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_routing_peer_rejections_total",
            "reason",
            "mismatched_node_id",
            maintenance.routing_peer_rejects.mismatched_node_id,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_routing_peer_rejections_total",
            "reason",
            "local_node",
            maintenance.routing_peer_rejects.local_node,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_routing_peer_rejections_total",
            "reason",
            "missing_identity_public_key",
            maintenance.routing_peer_rejects.missing_identity_public_key,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_routing_peer_rejections_total",
            "reason",
            "invalid_identity_public_key",
            maintenance.routing_peer_rejects.invalid_identity_public_key,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_routing_peer_rejections_total",
            "reason",
            "invalid_signature",
            maintenance.routing_peer_rejects.invalid_signature,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_routing_peer_rejections_total",
            "reason",
            "too_many_addresses",
            maintenance.routing_peer_rejects.too_many_addresses,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_routing_peer_rejections_total",
            "reason",
            "address_too_large",
            maintenance.routing_peer_rejects.address_too_large,
        );
        push_labeled_metric_value(
            &mut out,
            "lm_node_routing_peer_rejections_total",
            "reason",
            "all",
            maintenance.routing_peer_rejects.total(),
        );
        if let Some(last_pruned_at) = maintenance.last_pruned_at {
            push_metric_help(
                &mut out,
                "lm_node_maintenance_last_pruned_at",
                "Unix timestamp of the last expired-record prune run.",
            );
            push_metric_type(&mut out, "lm_node_maintenance_last_pruned_at", "gauge");
            push_metric_value(
                &mut out,
                "lm_node_maintenance_last_pruned_at",
                last_pruned_at,
            );
        }
        if let Some(state_file) = state_file {
            push_metric_help(
                &mut out,
                "lm_node_state_file_bytes",
                "JSON state_file size in bytes.",
            );
            push_metric_type(&mut out, "lm_node_state_file_bytes", "gauge");
            push_metric_value(&mut out, "lm_node_state_file_bytes", state_file.file_bytes);
            push_metric_help(
                &mut out,
                "lm_node_state_file_encrypted",
                "Whether the JSON state_file is encrypted with the application-layer format.",
            );
            push_metric_type(&mut out, "lm_node_state_file_encrypted", "gauge");
            push_metric_value(
                &mut out,
                "lm_node_state_file_encrypted",
                if state_file.encrypted { 1 } else { 0 },
            );
            push_metric_help(
                &mut out,
                "lm_node_state_file_permissions_hardened",
                "Whether the JSON state_file permissions are restricted to the node user.",
            );
            push_metric_type(&mut out, "lm_node_state_file_permissions_hardened", "gauge");
            push_metric_value(
                &mut out,
                "lm_node_state_file_permissions_hardened",
                if state_file.permissions_hardened {
                    1
                } else {
                    0
                },
            );
        }
        if let Some(state_db) = state_db {
            push_metric_help(
                &mut out,
                "lm_node_state_db_pages",
                "SQLite state database page counts.",
            );
            push_metric_type(&mut out, "lm_node_state_db_pages", "gauge");
            push_labeled_metric_value(
                &mut out,
                "lm_node_state_db_pages",
                "kind",
                "total",
                state_db.page_count,
            );
            push_labeled_metric_value(
                &mut out,
                "lm_node_state_db_pages",
                "kind",
                "free",
                state_db.freelist_count,
            );
            push_metric_help(
                &mut out,
                "lm_node_state_db_page_size_bytes",
                "SQLite state database page size in bytes.",
            );
            push_metric_type(&mut out, "lm_node_state_db_page_size_bytes", "gauge");
            push_metric_value(
                &mut out,
                "lm_node_state_db_page_size_bytes",
                state_db.page_size_bytes,
            );
            push_metric_help(
                &mut out,
                "lm_node_state_db_file_bytes",
                "SQLite state database file size in bytes.",
            );
            push_metric_type(&mut out, "lm_node_state_db_file_bytes", "gauge");
            push_metric_value(&mut out, "lm_node_state_db_file_bytes", state_db.file_bytes);
            push_metric_help(
                &mut out,
                "lm_node_state_db_encrypted",
                "Whether the SQLite state database is encrypted at the database layer.",
            );
            push_metric_type(&mut out, "lm_node_state_db_encrypted", "gauge");
            push_metric_value(
                &mut out,
                "lm_node_state_db_encrypted",
                if state_db.encrypted { 1 } else { 0 },
            );
            push_metric_help(
                &mut out,
                "lm_node_state_db_encryption_mode",
                "SQLite state database encryption mode. Current plain mode is not database encryption.",
            );
            push_metric_type(&mut out, "lm_node_state_db_encryption_mode", "gauge");
            push_labeled_metric_value(
                &mut out,
                "lm_node_state_db_encryption_mode",
                "mode",
                &state_db.encryption_mode,
                1,
            );
            push_metric_help(
                &mut out,
                "lm_node_state_db_permissions_hardened",
                "Whether the state database files are expected to be restricted to the node user.",
            );
            push_metric_type(&mut out, "lm_node_state_db_permissions_hardened", "gauge");
            push_metric_value(
                &mut out,
                "lm_node_state_db_permissions_hardened",
                if state_db.permissions_hardened { 1 } else { 0 },
            );
        }
        push_metric_help(
            &mut out,
            "lm_node_control_endpoint_requests_total",
            "Control HTTP responses by endpoint and status class.",
        );
        push_metric_type(
            &mut out,
            "lm_node_control_endpoint_requests_total",
            "counter",
        );
        push_metric_help(
            &mut out,
            "lm_node_control_endpoint_duration_micros_total",
            "Total control HTTP handler duration in microseconds by endpoint.",
        );
        push_metric_type(
            &mut out,
            "lm_node_control_endpoint_duration_micros_total",
            "counter",
        );
        push_metric_help(
            &mut out,
            "lm_node_control_endpoint_duration_micros_max",
            "Maximum observed control HTTP handler duration in microseconds by endpoint.",
        );
        push_metric_type(
            &mut out,
            "lm_node_control_endpoint_duration_micros_max",
            "gauge",
        );
        let mut endpoints = self.endpoints.iter().collect::<Vec<_>>();
        endpoints.sort_by_key(|(left, _)| *left);
        for (endpoint, stats) in endpoints {
            push_endpoint_metric_value(
                &mut out,
                "lm_node_control_endpoint_requests_total",
                endpoint,
                "all",
                stats.requests,
            );
            push_endpoint_metric_value(
                &mut out,
                "lm_node_control_endpoint_requests_total",
                endpoint,
                "2xx",
                stats.responses_2xx,
            );
            push_endpoint_metric_value(
                &mut out,
                "lm_node_control_endpoint_requests_total",
                endpoint,
                "4xx",
                stats.responses_4xx,
            );
            push_endpoint_metric_value(
                &mut out,
                "lm_node_control_endpoint_requests_total",
                endpoint,
                "5xx",
                stats.responses_5xx,
            );
            push_labeled_metric_value(
                &mut out,
                "lm_node_control_endpoint_duration_micros_total",
                "endpoint",
                endpoint,
                stats.total_duration_micros,
            );
            push_labeled_metric_value(
                &mut out,
                "lm_node_control_endpoint_duration_micros_max",
                "endpoint",
                endpoint,
                stats.max_duration_micros,
            );
            if let Some(status) = stats.last_status {
                push_labeled_metric_value(
                    &mut out,
                    "lm_node_control_endpoint_last_status",
                    "endpoint",
                    endpoint,
                    status,
                );
            }
        }
        out.push_str("# EOF\n");
        out
    }

    pub(crate) fn record_dht_replication_run(
        &mut self,
        stats: DhtReplicationRunStats,
        finished_at: u64,
    ) {
        self.dht_replication_runs = self.dht_replication_runs.saturating_add(1);
        self.dht_replication_records = self
            .dht_replication_records
            .saturating_add(stats.records as u64);
        self.dht_replication_attempts = self
            .dht_replication_attempts
            .saturating_add(stats.attempts as u64);
        self.dht_replication_successes = self
            .dht_replication_successes
            .saturating_add(stats.successes as u64);
        self.dht_replication_failures = self
            .dht_replication_failures
            .saturating_add(stats.failures as u64);
        self.dht_replication_peers_quarantined = self
            .dht_replication_peers_quarantined
            .saturating_add(stats.peers_quarantined as u64);
        self.last_dht_replication_at = Some(finished_at);
    }

    pub(crate) fn record_dht_routing_refresh_run(
        &mut self,
        stats: DhtRoutingRefreshRunStats,
        finished_at: u64,
    ) {
        self.dht_routing_refresh_runs = self.dht_routing_refresh_runs.saturating_add(1);
        self.dht_routing_refresh_targets = self
            .dht_routing_refresh_targets
            .saturating_add(stats.targets as u64);
        self.dht_routing_refresh_attempts = self
            .dht_routing_refresh_attempts
            .saturating_add(stats.attempts as u64);
        self.dht_routing_refresh_successes = self
            .dht_routing_refresh_successes
            .saturating_add(stats.successes as u64);
        self.dht_routing_refresh_failures = self
            .dht_routing_refresh_failures
            .saturating_add(stats.failures as u64);
        self.dht_routing_refresh_nodes_returned = self
            .dht_routing_refresh_nodes_returned
            .saturating_add(stats.nodes_returned as u64);
        self.dht_routing_refresh_nodes_merged = self
            .dht_routing_refresh_nodes_merged
            .saturating_add(stats.nodes_merged as u64);
        self.dht_routing_refresh_nodes_rejected_non_closer = self
            .dht_routing_refresh_nodes_rejected_non_closer
            .saturating_add(stats.nodes_rejected_non_closer as u64);
        self.dht_routing_refresh_nodes_rejected_duplicate = self
            .dht_routing_refresh_nodes_rejected_duplicate
            .saturating_add(stats.nodes_rejected_duplicate as u64);
        self.dht_routing_refresh_peers_quarantined = self
            .dht_routing_refresh_peers_quarantined
            .saturating_add(stats.peers_quarantined as u64);
        self.last_dht_routing_refresh_at = Some(finished_at);
    }

    pub(crate) fn record_dht_find_value_run(
        &mut self,
        stats: DhtFindValueRunStats,
        finished_at: u64,
    ) {
        self.dht_find_value_runs = self.dht_find_value_runs.saturating_add(1);
        self.dht_find_value_attempts = self
            .dht_find_value_attempts
            .saturating_add(stats.attempts as u64);
        self.dht_find_value_successes = self
            .dht_find_value_successes
            .saturating_add(stats.successes as u64);
        self.dht_find_value_failures = self
            .dht_find_value_failures
            .saturating_add(stats.failures as u64);
        self.dht_find_value_found_records = self
            .dht_find_value_found_records
            .saturating_add(stats.found_records as u64);
        self.dht_find_value_invalid_found_records = self
            .dht_find_value_invalid_found_records
            .saturating_add(stats.invalid_found_records as u64);
        self.dht_find_value_closer_records = self
            .dht_find_value_closer_records
            .saturating_add(stats.closer_records as u64);
        self.dht_find_value_closer_nodes_returned = self
            .dht_find_value_closer_nodes_returned
            .saturating_add(stats.closer_nodes_returned as u64);
        self.dht_find_value_closer_nodes_merged = self
            .dht_find_value_closer_nodes_merged
            .saturating_add(stats.closer_nodes_merged as u64);
        self.dht_find_value_closer_nodes_rejected_non_closer = self
            .dht_find_value_closer_nodes_rejected_non_closer
            .saturating_add(stats.closer_nodes_rejected_non_closer as u64);
        self.dht_find_value_closer_nodes_rejected_duplicate = self
            .dht_find_value_closer_nodes_rejected_duplicate
            .saturating_add(stats.closer_nodes_rejected_duplicate as u64);
        self.dht_find_value_peers_quarantined = self
            .dht_find_value_peers_quarantined
            .saturating_add(stats.peers_quarantined as u64);
        self.dht_find_value_query_rounds = self
            .dht_find_value_query_rounds
            .saturating_add(stats.query_rounds as u64);
        self.dht_find_value_alpha_max = self.dht_find_value_alpha_max.max(stats.alpha);
        if stats.exhausted {
            self.dht_find_value_exhausted = self.dht_find_value_exhausted.saturating_add(1);
        }
        self.last_dht_find_value_at = Some(finished_at);
    }

    pub(crate) fn record_sync_schedule_delay(&mut self, delay: Duration) {
        let micros = delay.as_micros();
        self.sync_schedule_delay_micros_total =
            self.sync_schedule_delay_micros_total.saturating_add(micros);
        self.sync_schedule_delay_micros_max = self.sync_schedule_delay_micros_max.max(micros);
        self.last_sync_schedule_delay_micros = Some(micros);
    }

    pub(crate) fn record_dht_replication_schedule_delay(&mut self, delay: Duration) {
        let micros = delay.as_micros();
        self.dht_replication_schedule_delay_micros_total = self
            .dht_replication_schedule_delay_micros_total
            .saturating_add(micros);
        self.dht_replication_schedule_delay_micros_max =
            self.dht_replication_schedule_delay_micros_max.max(micros);
        self.last_dht_replication_schedule_delay_micros = Some(micros);
    }

    pub(crate) fn record_dht_routing_refresh_schedule_delay(&mut self, delay: Duration) {
        let micros = delay.as_micros();
        self.dht_routing_refresh_schedule_delay_micros_total = self
            .dht_routing_refresh_schedule_delay_micros_total
            .saturating_add(micros);
        self.dht_routing_refresh_schedule_delay_micros_max = self
            .dht_routing_refresh_schedule_delay_micros_max
            .max(micros);
        self.last_dht_routing_refresh_schedule_delay_micros = Some(micros);
    }

    pub(crate) fn record_sync_snapshot_bytes(
        &mut self,
        endpoint: &str,
        status: u16,
        request_body_bytes: usize,
        response_body_bytes: usize,
    ) {
        if !(200..=299).contains(&status) {
            return;
        }
        match endpoint {
            "GET /sync/snapshot" => {
                self.sync_snapshot_exports = self.sync_snapshot_exports.saturating_add(1);
                self.sync_snapshot_export_bytes = self
                    .sync_snapshot_export_bytes
                    .saturating_add(response_body_bytes as u64);
            }
            "POST /sync/import" => {
                self.sync_snapshot_imports = self.sync_snapshot_imports.saturating_add(1);
                self.sync_snapshot_import_bytes = self
                    .sync_snapshot_import_bytes
                    .saturating_add(request_body_bytes as u64);
            }
            _ => {}
        }
    }

    pub(crate) fn record_response(&mut self, endpoint: &str, status: u16, duration: Duration) {
        self.requests_total = self.requests_total.saturating_add(1);
        match status {
            200..=299 => self.responses_2xx = self.responses_2xx.saturating_add(1),
            400..=499 => self.responses_4xx = self.responses_4xx.saturating_add(1),
            500..=599 => self.responses_5xx = self.responses_5xx.saturating_add(1),
            _ => {}
        }
        match status {
            400 => self.bad_requests = self.bad_requests.saturating_add(1),
            401 => self.unauthorized = self.unauthorized.saturating_add(1),
            403 => self.cors_rejected = self.cors_rejected.saturating_add(1),
            429 => self.rate_limited = self.rate_limited.saturating_add(1),
            _ => {}
        }
        self.endpoints
            .entry(endpoint.to_string())
            .or_default()
            .record(status, duration);
    }
}

pub(super) fn push_sync_status_metrics(
    out: &mut String,
    sync_status: &NodeSyncStatus,
    dht_peer_quarantine_threshold: u32,
) {
    push_metric_help(
        out,
        "lm_node_sync_peer_attempts_total",
        "Snapshot sync attempts by peer.",
    );
    push_metric_type(out, "lm_node_sync_peer_attempts_total", "counter");
    push_metric_help(
        out,
        "lm_node_sync_peer_successes_total",
        "Snapshot sync successes by peer.",
    );
    push_metric_type(out, "lm_node_sync_peer_successes_total", "counter");
    push_metric_help(
        out,
        "lm_node_sync_peer_failures_total",
        "Snapshot sync failures by peer.",
    );
    push_metric_type(out, "lm_node_sync_peer_failures_total", "counter");
    push_metric_help(
        out,
        "lm_node_sync_peer_consecutive_failures",
        "Current consecutive snapshot sync failures by peer.",
    );
    push_metric_type(out, "lm_node_sync_peer_consecutive_failures", "gauge");
    push_metric_help(
        out,
        "lm_node_sync_peer_next_attempt_at",
        "Unix timestamp for the next scheduled snapshot sync attempt by peer.",
    );
    push_metric_type(out, "lm_node_sync_peer_next_attempt_at", "gauge");
    push_metric_help(
        out,
        "lm_node_dht_peer_quarantined",
        "Whether a sync/control peer is currently skipped by DHT runners due to consecutive failures.",
    );
    push_metric_type(out, "lm_node_dht_peer_quarantined", "gauge");
    for (peer, status) in &sync_status.peers {
        push_labeled_metric_value(
            out,
            "lm_node_sync_peer_attempts_total",
            "peer",
            peer,
            status.attempts,
        );
        push_labeled_metric_value(
            out,
            "lm_node_sync_peer_successes_total",
            "peer",
            peer,
            status.successes,
        );
        push_labeled_metric_value(
            out,
            "lm_node_sync_peer_failures_total",
            "peer",
            peer,
            status.failures,
        );
        push_labeled_metric_value(
            out,
            "lm_node_sync_peer_consecutive_failures",
            "peer",
            peer,
            status.consecutive_failures,
        );
        push_labeled_metric_value(
            out,
            "lm_node_dht_peer_quarantined",
            "peer",
            peer,
            if sync_peer_is_dht_quarantined(
                status,
                current_unix_timestamp(),
                dht_peer_quarantine_threshold,
            ) {
                1
            } else {
                0
            },
        );
        if let Some(next_attempt_at) = status.next_attempt_at {
            push_labeled_metric_value(
                out,
                "lm_node_sync_peer_next_attempt_at",
                "peer",
                peer,
                next_attempt_at,
            );
        }
    }
}

pub(super) fn push_metric_help(out: &mut String, name: &str, help: &str) {
    out.push_str("# HELP ");
    out.push_str(name);
    out.push(' ');
    out.push_str(help);
    out.push('\n');
}

pub(super) fn push_metric_type(out: &mut String, name: &str, kind: &str) {
    out.push_str("# TYPE ");
    out.push_str(name);
    out.push(' ');
    out.push_str(kind);
    out.push('\n');
}

pub(super) fn push_metric_value(out: &mut String, name: &str, value: impl std::fmt::Display) {
    out.push_str(name);
    out.push(' ');
    out.push_str(&value.to_string());
    out.push('\n');
}

pub(super) fn push_labeled_metric_value(
    out: &mut String,
    name: &str,
    label_name: &str,
    label_value: &str,
    value: impl std::fmt::Display,
) {
    out.push_str(name);
    out.push('{');
    out.push_str(label_name);
    out.push_str("=\"");
    out.push_str(&escape_openmetrics_label(label_value));
    out.push_str("\"} ");
    out.push_str(&value.to_string());
    out.push('\n');
}

pub(super) fn push_endpoint_metric_value(
    out: &mut String,
    name: &str,
    endpoint: &str,
    class: &str,
    value: impl std::fmt::Display,
) {
    out.push_str(name);
    out.push_str("{endpoint=\"");
    out.push_str(&escape_openmetrics_label(endpoint));
    out.push_str("\",class=\"");
    out.push_str(&escape_openmetrics_label(class));
    out.push_str("\"} ");
    out.push_str(&value.to_string());
    out.push('\n');
}

pub(super) fn escape_openmetrics_label(value: &str) -> String {
    value
        .chars()
        .flat_map(|ch| match ch {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            _ => vec![ch],
        })
        .collect()
}

impl ControlEndpointStats {
    pub(crate) fn record(&mut self, status: u16, duration: Duration) {
        self.requests = self.requests.saturating_add(1);
        match status {
            200..=299 => self.responses_2xx = self.responses_2xx.saturating_add(1),
            400..=499 => self.responses_4xx = self.responses_4xx.saturating_add(1),
            500..=599 => self.responses_5xx = self.responses_5xx.saturating_add(1),
            _ => {}
        }
        let micros = duration.as_micros();
        self.total_duration_micros = self.total_duration_micros.saturating_add(micros);
        self.max_duration_micros = self.max_duration_micros.max(micros);
        self.last_status = Some(status);
    }
}
