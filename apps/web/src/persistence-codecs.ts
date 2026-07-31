import type {
  ChatMessage,
  ContactItem,
  EncryptedStringV1,
  FriendRequestItem,
  GroupInviteItem,
  GroupItem,
  GroupSenderKeyItem,
  MailboxFailedItem,
  OutboxItem,
  PendingSecureSessionOfferItem,
  ProcessedMailboxRecord,
  RatchetSessionItem,
} from './app-types'

export type PersistenceStringCodec = {
  encrypt: (value: string, keyId: string | null) => Promise<string | EncryptedStringV1>
  decrypt: (value: unknown, keyId: string | null) => Promise<string>
}

export function createPersistenceCodecs(codec: PersistenceStringCodec) {
  const encryptLocalString = codec.encrypt
  const decryptLocalString = codec.decrypt

  return {
    async encryptContact(contact: ContactItem, key: string | null): Promise<any> {
      return {
        ...contact,
        display_name: contact.display_name ? await encryptLocalString(contact.display_name, key) : contact.display_name,
        contact_card_text: await encryptLocalString(contact.contact_card_text, key),
        block_reason: contact.block_reason ? await encryptLocalString(contact.block_reason, key) : contact.block_reason,
        avatar_data_url: contact.avatar_data_url ? await encryptLocalString(contact.avatar_data_url, key) : contact.avatar_data_url,
      }
    },

    async decryptContact(contact: any, key: string | null): Promise<ContactItem> {
      return {
        ...contact,
        state: contact.state ?? 'LocalOnly',
        display_name: contact.display_name ? await decryptLocalString(contact.display_name, key) : contact.display_name,
        contact_card_text: await decryptLocalString(contact.contact_card_text, key),
        block_reason: contact.block_reason ? await decryptLocalString(contact.block_reason, key) : contact.block_reason,
        avatar_data_url: contact.avatar_data_url ? await decryptLocalString(contact.avatar_data_url, key) : contact.avatar_data_url,
      }
    },

    async encryptGroup(group: GroupItem, key: string | null): Promise<any> {
      return {
        ...group,
        name: await encryptLocalString(group.name, key),
        policy_state_json: group.policy_state_json ? await encryptLocalString(group.policy_state_json, key) : group.policy_state_json,
      }
    },

    async decryptGroup(group: any, key: string | null): Promise<GroupItem> {
      return {
        ...group,
        sequence: group.sequence ?? 0,
        admin_user_ids: group.admin_user_ids ?? [],
        name: await decryptLocalString(group.name, key),
        policy_state_json: group.policy_state_json ? await decryptLocalString(group.policy_state_json, key) : group.policy_state_json,
      }
    },

    async encryptMessage(message: ChatMessage, key: string | null): Promise<any> {
      return {
        ...message,
        text: await encryptLocalString(message.text, key),
        envelope_json: message.envelope_json ? await encryptLocalString(message.envelope_json, key) : message.envelope_json,
      }
    },

    async decryptMessage(message: any, key: string | null): Promise<ChatMessage> {
      return {
        ...message,
        status: message.status ?? (message.direction === 'in' ? 'received' : 'queued'),
        text: await decryptLocalString(message.text, key),
        envelope_json: message.envelope_json ? await decryptLocalString(message.envelope_json, key) : message.envelope_json,
      }
    },

    async encryptOutbox(item: OutboxItem, key: string | null): Promise<any> {
      return { ...item, envelope_json: await encryptLocalString(item.envelope_json, key) }
    },

    async decryptOutbox(item: any, key: string | null): Promise<OutboxItem> {
      return { ...item, status: item.status ?? 'queued', retry_count: item.retry_count ?? 0, envelope_json: await decryptLocalString(item.envelope_json, key) }
    },

    async encryptFriendRequest(item: FriendRequestItem, key: string | null): Promise<any> {
      return {
        ...item,
        note: item.note ? await encryptLocalString(item.note, key) : item.note,
        request_text: await encryptLocalString(item.request_text, key),
        from_contact_card_text: await encryptLocalString(item.from_contact_card_text, key),
        quarantine_reason: item.quarantine_reason ? await encryptLocalString(item.quarantine_reason, key) : item.quarantine_reason,
      }
    },

    async decryptFriendRequest(item: any, key: string | null): Promise<FriendRequestItem> {
      return {
        ...item,
        note: item.note ? await decryptLocalString(item.note, key) : item.note,
        request_text: await decryptLocalString(item.request_text, key),
        from_contact_card_text: await decryptLocalString(item.from_contact_card_text, key),
        quarantine_reason: item.quarantine_reason ? await decryptLocalString(item.quarantine_reason, key) : item.quarantine_reason,
      }
    },

    async encryptGroupInvite(item: GroupInviteItem, key: string | null): Promise<any> {
      return {
        ...item,
        group_name: await encryptLocalString(item.group_name, key),
        invite_text: await encryptLocalString(item.invite_text, key),
      }
    },

    async decryptGroupInvite(item: any, key: string | null): Promise<GroupInviteItem> {
      return {
        ...item,
        group_name: await decryptLocalString(item.group_name, key),
        invite_text: await decryptLocalString(item.invite_text, key),
      }
    },

    async encryptGroupSenderKey(item: GroupSenderKeyItem, key: string | null): Promise<any> {
      return {
        ...item,
        state_json: await encryptLocalString(item.state_json, key),
        distribution_text: item.distribution_text ? await encryptLocalString(item.distribution_text, key) : item.distribution_text,
      }
    },

    async decryptGroupSenderKey(item: any, key: string | null): Promise<GroupSenderKeyItem> {
      return {
        ...item,
        state_json: await decryptLocalString(item.state_json, key),
        distribution_text: item.distribution_text ? await decryptLocalString(item.distribution_text, key) : item.distribution_text,
      }
    },

    async encryptMailboxFailedItem(item: MailboxFailedItem, key: string | null): Promise<any> {
      return {
        ...item,
        message: await encryptLocalString(JSON.stringify(item.message ?? null), key),
        reason: await encryptLocalString(item.reason, key),
      }
    },

    async decryptMailboxFailedItem(item: any, key: string | null): Promise<MailboxFailedItem> {
      const messageText = await decryptLocalString(item.message, key)
      let message: any = null
      try { message = messageText ? JSON.parse(messageText) : null } catch { message = item.message }
      return {
        ...item,
        message,
        reason: await decryptLocalString(item.reason, key),
        retry_count: item.retry_count ?? 0,
      }
    },

    async encryptRatchet(item: RatchetSessionItem, key: string | null): Promise<any> {
      return { ...item, state_text: await encryptLocalString(item.state_text, key) }
    },

    async decryptRatchet(item: any, key: string | null): Promise<RatchetSessionItem> {
      return { ...item, state_text: await decryptLocalString(item.state_text, key) }
    },

    async encryptPendingSecureSessionOffer(item: PendingSecureSessionOfferItem, key: string | null): Promise<any> {
      return {
        ...item,
        prekey_private_bundle_json: await encryptLocalString(item.prekey_private_bundle_json, key),
        ratchet_dh_private_key: await encryptLocalString(item.ratchet_dh_private_key, key),
      }
    },

    async decryptPendingSecureSessionOffer(item: any, key: string | null): Promise<PendingSecureSessionOfferItem> {
      return {
        ...item,
        prekey_private_bundle_json: await decryptLocalString(item.prekey_private_bundle_json, key),
        ratchet_dh_private_key: await decryptLocalString(item.ratchet_dh_private_key, key),
      }
    },
  }
}

export function normalizeProcessedMailboxRecords(
  records: Array<string | ProcessedMailboxRecord> | undefined,
  now = Date.now(),
  retentionMs = 30 * 24 * 3600 * 1000,
  maxRecords = 500,
): ProcessedMailboxRecord[] {
  const seen = new Set<string>()
  const normalized: ProcessedMailboxRecord[] = []
  for (const record of records ?? []) {
    const id = typeof record === 'string' ? record : record.id
    if (!id || seen.has(id)) continue
    const processedAt = typeof record === 'string' ? now : record.processed_at || now
    if (now - processedAt > retentionMs) continue
    seen.add(id)
    normalized.push({ id, processed_at: processedAt })
  }
  return normalized
    .sort((a, b) => b.processed_at - a.processed_at)
    .slice(0, maxRecords)
}
