import type { FriendRequestItem, FriendRequestRateRecord } from './app-types'

export type FriendRequestRateConfig = {
  shortWindowMs: number
  longWindowMs: number
  longLimit: number
}

export function recordFriendRequestRate(
  records: FriendRequestRateRecord[],
  fromUserId: string,
  config: FriendRequestRateConfig,
  now = Date.now(),
): { records: FriendRequestRateRecord[]; record: FriendRequestRateRecord } {
  const activeRecords = records.filter((record) =>
    now - record.first_seen_at <= config.longWindowMs || record.from_user_id === fromUserId,
  )
  let record = activeRecords.find((item) => item.from_user_id === fromUserId)
  if (!record || now - record.first_seen_at > config.longWindowMs) {
    record = { from_user_id: fromUserId, first_seen_at: now, last_seen_at: now, count: 0 }
    activeRecords.push(record)
  }
  record.count += 1
  record.last_seen_at = now
  return { records: activeRecords, record }
}

export function friendRequestQuarantineReason(
  requests: FriendRequestItem[],
  info: Pick<FriendRequestItem, 'from_user_id' | 'request_id' | 'created_at'>,
  records: FriendRequestRateRecord[],
  recordLongRate: boolean,
  config: FriendRequestRateConfig,
  now = Date.now(),
): { reason?: string; records: FriendRequestRateRecord[] } {
  const recentSameSourceCount = requests.filter((request) =>
    request.from_user_id === info.from_user_id
    && request.request_id !== info.request_id
    && now - request.created_at <= config.shortWindowMs,
  ).length
  if (recentSameSourceCount >= 1) {
    const windowMinutes = Math.round(config.shortWindowMs / 60_000)
    return {
      reason: `同一来源 ${windowMinutes} 分钟内已有 ${recentSameSourceCount} 条未处理请求`,
      records,
    }
  }
  if (!recordLongRate) return { records }
  const next = recordFriendRequestRate(records, info.from_user_id, config, now)
  if (next.record.count > config.longLimit) {
    const windowHours = Math.round(config.longWindowMs / 60 / 60 / 1000)
    return {
      reason: `同一来源 ${windowHours} 小时内已有 ${next.record.count} 条请求`,
      records: next.records,
    }
  }
  return { records: next.records }
}

export function upsertFriendRequest(
  requests: FriendRequestItem[],
  item: FriendRequestItem,
  records: FriendRequestRateRecord[],
  config: FriendRequestRateConfig,
  now = Date.now(),
): { requests: FriendRequestItem[]; records: FriendRequestRateRecord[]; item: FriendRequestItem } {
  const index = requests.findIndex((request) => request.request_id === item.request_id)
  const existing = index >= 0 ? requests[index] : undefined
  const quarantine = friendRequestQuarantineReason(requests, item, records, index < 0, config, now)
  const nextItem: FriendRequestItem = {
    ...item,
    quarantined: existing?.quarantined || Boolean(quarantine.reason),
    quarantine_reason: existing?.quarantine_reason || quarantine.reason,
  }
  const nextRequests = index >= 0
    ? requests.map((request, requestIndex) => requestIndex === index ? nextItem : request)
    : [nextItem, ...requests]
  return { requests: nextRequests, records: quarantine.records, item: nextItem }
}
