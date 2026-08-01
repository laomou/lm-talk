import type { ChatMessage } from './app-types'

export function mergeUniqueBy<T>(
  current: T[],
  incoming: T[],
  keyOf: (item: T) => string | undefined,
): { items: T[]; added: number; skipped: number } {
  const seen = new Set(current.map(keyOf).filter(Boolean) as string[])
  const items = [...current]
  let added = 0
  let skipped = 0
  for (const item of incoming) {
    const key = keyOf(item)
    if (!key || seen.has(key)) {
      skipped += 1
      continue
    }
    seen.add(key)
    items.push(item)
    added += 1
  }
  return { items, added, skipped }
}

function messageStateRank(message: ChatMessage): number {
  if (message.status === 'read' || message.read_at) return 5
  if (message.status === 'delivered' || message.delivered_at) return 4
  if (message.status === 'mailbox') return 3
  if (message.status === 'sent') return 2
  if (message.status === 'failed') return 1
  return 0
}

function messageMergeKey(message: ChatMessage): string | undefined {
  if (message.protocol_message_id && message.peer_user_id) {
    return `protocol:${message.peer_user_id}:${message.protocol_message_id}`
  }
  if (message.id) return `id:${message.id}`
  return undefined
}

export function mergeMessageStateInto(target: ChatMessage, incoming: ChatMessage): boolean {
  let changed = false
  if (messageStateRank(incoming) > messageStateRank(target)) {
    target.status = incoming.status
    changed = true
  }
  if (!target.delivered_at && incoming.delivered_at) {
    target.delivered_at = incoming.delivered_at
    changed = true
  }
  if (!target.read_at && incoming.read_at) {
    target.read_at = incoming.read_at
    changed = true
  }
  if (!target.mailbox_delivery_id && incoming.mailbox_delivery_id) {
    target.mailbox_delivery_id = incoming.mailbox_delivery_id
    changed = true
  }
  if (!target.protocol_message_id && incoming.protocol_message_id) {
    target.protocol_message_id = incoming.protocol_message_id
    changed = true
  }
  if (target.read_at && target.status !== 'read') {
    target.status = 'read'
    changed = true
  } else if (target.delivered_at && messageStateRank(target) < 4) {
    target.status = 'delivered'
    changed = true
  }
  return changed
}

export function mergeMessagesForState(
  current: ChatMessage[],
  incoming: ChatMessage[],
): { items: ChatMessage[]; added: number; merged: number; skipped: number } {
  const items = [...current]
  const byKey = new Map<string, ChatMessage>()
  for (const item of items) {
    const key = messageMergeKey(item)
    if (key) byKey.set(key, item)
    if (item.id) byKey.set(`id:${item.id}`, item)
  }
  let added = 0
  let merged = 0
  let skipped = 0
  for (const item of incoming) {
    const key = messageMergeKey(item)
    const existing = key ? byKey.get(key) : item.id ? byKey.get(`id:${item.id}`) : undefined
    if (existing) {
      if (mergeMessageStateInto(existing, item)) merged += 1
      else skipped += 1
      continue
    }
    if (!key && !item.id) {
      skipped += 1
      continue
    }
    items.push(item)
    if (key) byKey.set(key, item)
    if (item.id) byKey.set(`id:${item.id}`, item)
    added += 1
  }
  return { items, added, merged, skipped }
}

export function compareConversationMessageOrder(left: ChatMessage, right: ChatMessage): number {
  return Number(left.created_at || 0) - Number(right.created_at || 0) || left.id.localeCompare(right.id)
}
