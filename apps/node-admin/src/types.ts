// Response shapes from the lm_node HTTP control-plane API.
// See crates/lm_node/src/control.rs and metrics.rs for the source structs.

export interface NodeConfig {
  url: string
  token: string
}

export interface HealthResponse {
  status?: string
  peer_id?: string
  node_id?: string
  peers?: number
  prekeys?: number
  mailbox_deliveries?: number
  mailbox_bytes?: number
  mailbox_max_bytes?: number
  mailbox_max_bytes_per_user?: number
  mailbox_max_messages_per_user?: number
  state_db_encrypted?: boolean
  state_db_permissions_hardened?: boolean
  maintenance?: Record<string, unknown>
  sync?: Record<string, unknown>
  [key: string]: unknown
}

export interface EndpointStats {
  requests?: number
  responses_2xx?: number
  responses_4xx?: number
  responses_5xx?: number
  last_status?: number
  max_duration_micros?: number
  [key: string]: unknown
}

export interface EndpointGroupStats {
  endpoints?: number
  requests?: number
  responses_2xx?: number
  responses_4xx?: number
  responses_5xx?: number
  max_duration_micros?: number
  [key: string]: unknown
}

export interface ControlStatsResponse {
  started_at?: number
  requests_total?: number
  responses_2xx?: number
  responses_4xx?: number
  responses_5xx?: number
  bad_requests?: number
  unauthorized?: number
  cors_rejected?: number
  rate_limited?: number
  endpoints?: Record<string, EndpointStats>
  endpoint_groups?: Record<'mailbox' | 'dht' | 'sync' | 'other', EndpointGroupStats>
  maintenance?: Record<string, unknown>
  state_db?: Record<string, unknown> | null
  state_file?: Record<string, unknown> | null
  [key: string]: unknown
}

export interface SyncPeerStatus {
  url: string
  attempts?: number
  successes?: number
  failures?: number
  last_attempt_at?: number | null
  last_success_at?: number | null
  last_error_at?: number | null
  last_error?: string | null
  next_attempt_at?: number | null
  consecutive_failures?: number
  [key: string]: unknown
}

export interface SyncStatusResponse {
  peers?: Record<string, SyncPeerStatus>
  [key: string]: unknown
}
