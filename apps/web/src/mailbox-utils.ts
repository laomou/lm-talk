import type { MailboxFailureCategory } from './app-types'

type MailboxEventKind = 'message' | 'file' | 'friend-request' | 'friend-response' | 'group-invite' | 'delivery-ack' | 'read-receipt' | 'device-revoke' | 'contact-update' | 'secure-session' | 'data-backup' | 'self-sync' | 'other'

export function mailboxFailureCategory(reason: string): MailboxFailureCategory {
  if (/本地安全会话尚未建立|Ratchet Session/i.test(reason)) return 'session'
  if (/未知联系人|not-a-friend|还不是好友|已拉黑联系人/.test(reason)) return 'contact'
  if (/安全策略|sealed slot|设备|device/i.test(reason)) return 'security'
  if (/过期|expired/i.test(reason)) return 'expired'
  if (/解密|签名|invalid|signature|cryptographic operation failed/i.test(reason)) return 'decrypt'
  return 'other'
}

export function mailboxFailureRecoveryHint(reason: string): string {
  if (/本地安全会话尚未建立|Ratchet Session/i.test(reason)) return '正在自动恢复安全会话；恢复后可继续接收新消息，请对方重发这条消息。'
  if (/未知联系人|not-a-friend|还不是好友/.test(reason)) return '先添加/恢复联系人，再重试该 Mailbox 项。'
  if (/安全策略/.test(reason)) return '确认联系人状态和设备信息后再重试。'
  if (/sealed slot|设备|device/i.test(reason)) return '刷新 ContactCard DHT 或等待设备证书更新后重试。'
  if (/群|group/i.test(reason)) return '先接受群邀请或修复群成员状态，再重试。'
  if (/过期|expired/i.test(reason)) return '该载荷可能已过期；建议清空失败项。'
  if (/解密|签名|invalid|signature/i.test(reason)) return '载荷验签/解密失败；保留诊断报告后可清空。'
  return '修复对应联系人/群聊/同步状态后点击重试。'
}

export function mailboxFailureDisplayText(reason: string): string {
  switch (mailboxFailureCategory(reason)) {
    case 'session': return '安全会话暂未就绪'
    case 'contact': return '联系人状态需要处理'
    case 'security': return '联系人安全信息需要处理'
    case 'expired': return '消息已过期'
    case 'decrypt': return '消息暂时无法解密'
    default: return '消息暂时无法处理'
  }
}

export function mailboxEventSummaryText(events: MailboxEventKind[]): string {
  const count = (kind: MailboxEventKind) => events.filter((event) => event === kind).length
  const parts = [
    ['消息', count('message')],
    ['文件', count('file')],
    ['好友请求', count('friend-request')],
    ['好友通过', count('friend-response')],
    ['群邀请', count('group-invite')],
    ['安全会话', count('secure-session')],
    ['身份与安全备份', count('data-backup')],
    ['自同步', count('self-sync')],
    ['回执', count('delivery-ack') + count('read-receipt')],
  ].filter(([, n]) => Number(n) > 0).map(([label, n]) => `${label} ${n}`)
  return parts.length ? parts.join('，') : `已处理 ${events.length} 条`
}

export function mailboxDedupeIds(deliveryId?: string, messageId?: string, protocolMessageId?: string, profileUpdateId?: string): string[] {
  return [
    deliveryId,
    messageId ? `message:${messageId}` : '',
    protocolMessageId ? `protocol:${protocolMessageId}` : '',
    profileUpdateId ? `profile:${profileUpdateId}` : '',
  ]
    .map((id) => (id || '').trim())
    .filter(Boolean)
}
