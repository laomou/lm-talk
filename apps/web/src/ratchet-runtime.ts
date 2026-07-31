import type { RatchetSessionItem } from './app-types'

export function ratchetSessionFor(
  sessions: RatchetSessionItem[],
  peerUserId: string,
): RatchetSessionItem | null {
  return sessions.find((session) => session.peer_user_id === peerUserId) ?? null
}

export function saveRatchetSession(
  sessions: RatchetSessionItem[],
  peerUserId: string,
  stateText: string,
  updatedAt = Date.now(),
): RatchetSessionItem[] {
  const item: RatchetSessionItem = {
    peer_user_id: peerUserId,
    state_text: stateText,
    updated_at: updatedAt,
  }
  const index = sessions.findIndex((session) => session.peer_user_id === peerUserId)
  if (index < 0) return [...sessions, item]
  const next = [...sessions]
  next[index] = item
  return next
}

export function removeRatchetSession(
  sessions: RatchetSessionItem[],
  peerUserId: string,
): RatchetSessionItem[] {
  return sessions.filter((session) => session.peer_user_id !== peerUserId)
}

export class KeyedTaskQueue {
  private readonly chains = new Map<string, Promise<unknown>>()

  run<T>(key: string, work: () => Promise<T>): Promise<T> {
    const previous = this.chains.get(key) ?? Promise.resolve()
    const task = previous.catch(() => undefined).then(work)
    this.chains.set(key, task)
    void task.finally(() => {
      if (this.chains.get(key) === task) this.chains.delete(key)
    }).catch(() => undefined)
    return task
  }

  clear(key?: string) {
    if (key === undefined) this.chains.clear()
    else this.chains.delete(key)
  }
}
