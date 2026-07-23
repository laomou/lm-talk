import init, {
  apply_group_policy_event,
  create_group_event,
  create_group_invite,
  create_group_policy_state,
  inspect_group_event,
  inspect_group_invite,
} from './wasm/lm_wasm.js'

type CreateInviteRequest = {
  id: number
  type: 'createInvite'
  backupText: string
  passphrase: string
  groupId: string
  groupName: string
  memberIdsJson: string
}

type InspectInviteRequest = {
  id: number
  type: 'inspectInvite'
  inviteText: string
  inviterContactCardText: string
}

type CreateEventRequest = {
  id: number
  type: 'createEvent'
  backupText: string
  passphrase: string
  groupId: string
  sequence: string
  actionJson: string
}

type InspectEventRequest = {
  id: number
  type: 'inspectEvent'
  eventText: string
  actorContactCardText: string
}

type ApplyPolicyEventRequest = {
  id: number
  type: 'applyPolicyEvent'
  policyStateJson: string
  eventText: string
  actorContactCardText: string
}

type CreatePolicyStateRequest = {
  id: number
  type: 'createPolicyState'
  groupId: string
  groupName: string
  inviterUserId: string
  memberIdsJson: string
}

let wasmReady: Promise<unknown> | null = null

function ensureWasmReady(): Promise<unknown> {
  wasmReady ??= init()
  return wasmReady
}

self.onmessage = async (event: MessageEvent<CreateInviteRequest | InspectInviteRequest | CreateEventRequest | InspectEventRequest | ApplyPolicyEventRequest | CreatePolicyStateRequest>) => {
  const request = event.data
  try {
    await ensureWasmReady()
    if (request.type === 'createInvite') {
      self.postMessage({
        id: request.id,
        ok: true,
        value: create_group_invite(
          request.backupText,
          request.passphrase,
          request.groupId,
          request.groupName,
          request.memberIdsJson,
        ),
      })
      return
    }
    if (request.type === 'inspectInvite') {
      self.postMessage({ id: request.id, ok: true, value: inspect_group_invite(request.inviteText, request.inviterContactCardText) })
      return
    }
    if (request.type === 'createEvent') {
      self.postMessage({
        id: request.id,
        ok: true,
        value: create_group_event(
          request.backupText,
          request.passphrase,
          request.groupId,
          BigInt(request.sequence),
          request.actionJson,
        ),
      })
      return
    }
    if (request.type === 'inspectEvent') {
      self.postMessage({ id: request.id, ok: true, value: inspect_group_event(request.eventText, request.actorContactCardText) })
      return
    }
    if (request.type === 'applyPolicyEvent') {
      self.postMessage({ id: request.id, ok: true, value: apply_group_policy_event(request.policyStateJson, request.eventText, request.actorContactCardText) })
      return
    }
    self.postMessage({
      id: request.id,
      ok: true,
      value: create_group_policy_state(
        request.groupId,
        request.groupName,
        request.inviterUserId,
        request.memberIdsJson,
      ),
    })
  } catch (error) {
    self.postMessage({
      id: request.id,
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    })
  }
}
