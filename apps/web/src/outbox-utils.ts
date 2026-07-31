import type { ChatMessage, ContactItem, OutboxItem } from './app-types'

export const OUTBOX_EXPIRY_MS = 7 * 24 * 3600 * 1000

export function createOutboxItem(
  contact: ContactItem,
  payload: string,
  newId: () => string,
  nextDeliveryOrder: number,
  messageId?: string,
  kind: OutboxItem['kind'] = 'direct-envelope',
  now = Date.now(),
): OutboxItem {
  return {
    id: newId(),
    peer_user_id: contact.user_id,
    envelope_json: payload,
    message_id: messageId,
    kind,
    status: 'queued',
    created_at: now,
    delivery_order: nextDeliveryOrder,
    retry_count: 0,
    next_retry_at: now,
    expires_at: now + OUTBOX_EXPIRY_MS,
  }
}

export function mailboxKindForOutboxKind(kind: OutboxItem['kind']): string {
  if (kind === 'group-fanout') return 'group-fanout'
  if (kind === 'delivery-receipt') return 'delivery-receipt'
  if (kind === 'read-receipt') return 'read-receipt'
  if (kind === 'contact-update') return 'contact-update'
  if (kind === 'file-package') return 'other'
  if (kind === 'other') return 'other'
  return 'direct-envelope'
}

export function retryDelayMs(retryCount: number): number {
  return [30_000, 2 * 60_000, 10 * 60_000, 60 * 60_000, 6 * 60 * 60_000][Math.min(retryCount, 4)]
}

export function isRetryableDeliveryError(errorText: string): boolean {
  return errorText === '网络失败'
    || errorText === '节点错误'
    || errorText === '节点拒绝：请求过于频繁'
}

export function outboxDeliveryTimestamp(
  item: OutboxItem,
  messagesById: ReadonlyMap<string, ChatMessage>,
): number {
  if (item.message_id) {
    const message = messagesById.get(item.message_id)
    if (message?.created_at) return message.created_at
  }
  return item.created_at
}

export function compareOutboxDeliveryOrder(
  left: OutboxItem,
  right: OutboxItem,
  messagesById: ReadonlyMap<string, ChatMessage>,
): number {
  if (left.delivery_order !== undefined && right.delivery_order !== undefined) {
    const deliveryOrderDelta = left.delivery_order - right.delivery_order
    if (deliveryOrderDelta !== 0) return deliveryOrderDelta
  }
  const timestampDelta = outboxDeliveryTimestamp(left, messagesById) - outboxDeliveryTimestamp(right, messagesById)
  if (timestampDelta !== 0) return timestampDelta
  const createdAtDelta = left.created_at - right.created_at
  if (createdAtDelta !== 0) return createdAtDelta
  return left.id.localeCompare(right.id)
}
