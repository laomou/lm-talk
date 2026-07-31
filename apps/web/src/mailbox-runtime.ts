import { mailboxFailureDisplayText } from './mailbox-utils'

export type MailboxEventKind =
  | 'message'
  | 'file'
  | 'friend-request'
  | 'friend-response'
  | 'group-invite'
  | 'delivery-ack'
  | 'read-receipt'
  | 'device-revoke'
  | 'contact-update'
  | 'secure-session'
  | 'data-backup'
  | 'self-sync'
  | 'other'

export type MailboxTakeOptions = {
  waitSeconds?: number
  signal?: AbortSignal
  quietEmpty?: boolean
}

export type MailboxPayloadResult = {
  handled: boolean
  deliveryId?: string
  event?: MailboxEventKind
  reason?: string
}

export type MailboxBatchResult = {
  handled: number
  duplicate: number
  duplicateAckResent: number
  failed: number
  failureReasons: string[]
  events: MailboxEventKind[]
  ackIds: string[]
}

export function unwrapMailboxDelivery(item: any): { deliveryId?: string; message: any } {
  if (item && typeof item === 'object' && item.message) {
    return { deliveryId: String(item.delivery_id ?? ''), message: item.message }
  }
  return { message: item }
}

export function mailboxFailedItemId(deliveryId: string, messageId: string): string {
  return deliveryId || messageId || `local-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`
}

export function summarizeMailboxFailures(reasons: string[]): string {
  if (reasons.length === 0) return ''
  const counts = new Map<string, number>()
  for (const reason of reasons) {
    const key = mailboxFailureDisplayText(reason)
    counts.set(key, (counts.get(key) ?? 0) + 1)
  }
  return [...counts.entries()].map(([reason, count]) => `${reason} ${count}`).join('，')
}

export function isAbortError(error: unknown): boolean {
  return error instanceof DOMException && error.name === 'AbortError'
    || String(error instanceof Error ? error.message : error).includes('AbortError')
}

export async function processMailboxBatch(
  items: any[],
  options: {
    dedupeIdsFor: (item: any) => string[]
    isProcessed: (ids: string[]) => boolean
    resendDuplicateAck?: (item: any) => boolean
    rememberProcessed: (ids: string[]) => void
    handle: (item: any) => Promise<MailboxPayloadResult>
    rememberFailure: (item: any, reason: string) => void
    yieldBudgetMs?: number
  },
): Promise<MailboxBatchResult> {
  let handled = 0
  let duplicate = 0
  let duplicateAckResent = 0
  let failed = 0
  const failureReasons: string[] = []
  const events: MailboxEventKind[] = []
  const ackIds: string[] = []
  const yieldBudgetMs = options.yieldBudgetMs ?? 8
  let nextYieldAt = performance.now() + yieldBudgetMs

  for (const [index, item] of items.entries()) {
    const { deliveryId } = unwrapMailboxDelivery(item)
    const dedupeIds = options.dedupeIdsFor(item)
    if (options.isProcessed(dedupeIds)) {
      duplicate += 1
      if (options.resendDuplicateAck?.(item)) duplicateAckResent += 1
      if (deliveryId) ackIds.push(deliveryId)
      options.rememberProcessed(dedupeIds)
    } else {
      const result = await options.handle(item)
      if (result.handled) {
        handled += 1
        if (result.event) events.push(result.event)
        if (deliveryId) ackIds.push(deliveryId)
        options.rememberProcessed(dedupeIds)
      } else {
        failed += 1
        if (result.reason) {
          failureReasons.push(result.reason)
          options.rememberFailure(item, result.reason)
        }
      }
    }
    if (index < items.length - 1 && performance.now() >= nextYieldAt) {
      await new Promise<void>((resolve) => window.setTimeout(resolve, 0))
      nextYieldAt = performance.now() + yieldBudgetMs
    }
  }

  return { handled, duplicate, duplicateAckResent, failed, failureReasons, events, ackIds }
}
