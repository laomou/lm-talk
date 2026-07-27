import type { ContactInfo, ContactItem, DeviceCertItem, DeviceOutput, DeviceRevokeInfo } from './app-types'
import { base64UrlToString } from './app-utils'

export function contactCardDeviceCerts(cardText: string): DeviceCertItem[] {
  try {
    const payload = cardText.slice('lm-contact-card-v1:'.length)
    const parsed = JSON.parse(base64UrlToString(payload)) as { device_certs?: DeviceCertItem[] }
    return Array.isArray(parsed.device_certs) ? parsed.device_certs : []
  } catch { return [] }
}

export function mergeContactCard(existing: ContactItem | undefined, info: ContactInfo, cardText: string): ContactItem {
  if (existing && existing.identity_public_key !== info.identity_public_key) {
    throw new Error('拒绝更新：Contact Card identity_public_key 与已有联系人不一致')
  }
  return {
    ...(existing ?? {}),
    ...info,
    contact_card_text: cardText,
    kind: 'contact',
    state: existing?.state ?? 'LocalOnly',
    pending_request_id: existing?.pending_request_id,
    revoked_device_ids: existing?.revoked_device_ids,
    device_revocations: existing?.device_revocations,
    block_reason: existing?.block_reason,
    read_receipts: existing?.read_receipts ?? 'default',
    fingerprint_verified_at: existing?.fingerprint_verified_at,
    fingerprint_verified_note: existing?.fingerprint_verified_note,
    device_certs: info.device_certs ?? contactCardDeviceCerts(cardText),
    avatar_data_url: existing?.avatar_data_url,
  }
}

export function contactRevokedDeviceIds(contact: ContactItem): string[] {
  const revoked = new Set(contact.revoked_device_ids ?? [])
  for (const item of contact.device_revocations ?? []) revoked.add(item.device_id)
  const known = (contact.device_certs ?? []).map((cert) => cert.device_id).filter((deviceId) => revoked.has(deviceId))
  const knownSet = new Set(known)
  const unknown = [...revoked].filter((deviceId) => !knownSet.has(deviceId))
  return [...known, ...unknown]
}

export function contactRevokedDeviceDetails(contact: ContactItem): DeviceRevokeInfo[] {
  const byId = new Map((contact.device_revocations ?? []).map((item) => [item.device_id, item]))
  return contactRevokedDeviceIds(contact).map((deviceId) => byId.get(deviceId) ?? {
    user_id: contact.user_id,
    device_id: deviceId,
    created_at: 0,
  })
}

export function contactActiveDeviceIds(contact: ContactItem): string[] {
  const revoked = new Set(contact.revoked_device_ids ?? [])
  return (contact.device_certs ?? [])
    .map((cert) => cert.device_id)
    .filter((deviceId) => !revoked.has(deviceId))
}

export function contactRevokedDeviceCount(contact: ContactItem): number {
  return contactRevokedDeviceIds(contact).length
}

export function contactKnownRevokedDeviceCount(contact: ContactItem): number {
  const revoked = new Set(contact.revoked_device_ids ?? [])
  return (contact.device_certs ?? []).filter((cert) => revoked.has(cert.device_id)).length
}

export function contactAllKnownDevicesRevoked(contact: ContactItem): boolean {
  const certs = contact.device_certs ?? []
  if (certs.length === 0) return false
  const revoked = new Set(contact.revoked_device_ids ?? [])
  return certs.every((cert) => revoked.has(cert.device_id))
}

export function contactSealedSlotStatusText(contact: ContactItem): string {
  const activeDeviceIds = contactActiveDeviceIds(contact)
  if (activeDeviceIds.length === 0) return '联系人没有活跃设备证书，无法使用 sealed slot 投递。'
  const certs = contact.device_certs ?? []
  const sealed = activeDeviceIds.filter((deviceId) => certs.find((cert) => cert.device_id === deviceId)?.device_box_public_key).length
  if (sealed === activeDeviceIds.length) return `sealed slot 就绪：${sealed}/${activeDeviceIds.length} 个活跃设备支持设备级加密。`
  return `兼容模式风险：${activeDeviceIds.length - sealed}/${activeDeviceIds.length} 个活跃设备缺少 device_box_public_key，将使用 placeholder/fallback；可在设置中开启“仅发送到支持 sealed slot 的设备”阻止降级。`
}

export function activeContactSealedSlotRiskFor(contact: ContactItem): 'ok' | 'high' {
  const activeDeviceIds = contactActiveDeviceIds(contact)
  if (activeDeviceIds.length === 0) return 'high'
  const certs = contact.device_certs ?? []
  return activeDeviceIds.some((deviceId) => !certs.find((cert) => cert.device_id === deviceId)?.device_box_public_key) ? 'high' : 'ok'
}
