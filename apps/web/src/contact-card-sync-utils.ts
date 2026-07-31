import type { ContactCardUpdateFanoutRecord } from './app-types'

export function normalizeContactCardUpdateFanoutRecords(
  records: ContactCardUpdateFanoutRecord[],
  limit = 100,
): ContactCardUpdateFanoutRecord[] {
  return (records ?? [])
    .filter((record) => record?.peer_user_id && record?.update_id && record?.sent_at)
    .sort((left, right) => Number(right.sent_at ?? 0) - Number(left.sent_at ?? 0))
    .slice(0, limit)
}

export function contactCardUpdateRecordIsStale(
  record: ContactCardUpdateFanoutRecord,
  staleAfterMs: number,
  now = Date.now(),
): boolean {
  return record.status !== 'acked'
    && now - Number(record.last_retry_at || record.sent_at || 0) >= staleAfterMs
}

export function rememberContactCardUpdateFanout(
  records: ContactCardUpdateFanoutRecord[],
  peerUserId: string,
  updateId: string,
  status: 'sent' | 'queued',
  now = Date.now(),
): ContactCardUpdateFanoutRecord[] {
  const index = records.findIndex((item) => item.peer_user_id === peerUserId && item.update_id === updateId)
  if (index < 0) {
    return normalizeContactCardUpdateFanoutRecords([{
      peer_user_id: peerUserId,
      update_id: updateId,
      status,
      sent_at: now,
      retry_count: 0,
    }, ...records])
  }
  const next = [...records]
  const existing = next[index]
  next[index] = {
    ...existing,
    status: existing.status === 'acked' ? 'acked' : status,
    sent_at: now,
  }
  return next
}

export function acknowledgeContactCardUpdate(
  records: ContactCardUpdateFanoutRecord[],
  peerUserId: string,
  updateId: string,
  now = Date.now(),
): { records: ContactCardUpdateFanoutRecord[]; acknowledged: boolean } {
  const index = records.findIndex((item) => item.peer_user_id === peerUserId && item.update_id === updateId)
  if (index < 0) return { records, acknowledged: false }
  const next = [...records]
  next[index] = {
    ...next[index],
    status: 'acked',
    acked_at: next[index].acked_at || now,
  }
  return { records: next, acknowledged: true }
}
