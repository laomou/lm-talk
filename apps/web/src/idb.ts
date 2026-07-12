const DB_NAME = 'lm-talk-web-db'
const DB_VERSION = 4
const KV_STORE = 'kv'

export const TABLES = {
  meta: 'meta',
  contacts: 'contacts',
  friendRequests: 'friendRequests',
  groups: 'groups',
  groupInvites: 'groupInvites',
  groupSenderKeys: 'groupSenderKeys',
  messages: 'messages',
  outbox: 'outbox',
  ratchetSessions: 'ratchetSessions',
} as const

export type TableName = typeof TABLES[keyof typeof TABLES]

let dbPromise: Promise<IDBDatabase> | null = null

function ensureStore(db: IDBDatabase, name: string) {
  if (!db.objectStoreNames.contains(name)) db.createObjectStore(name)
}

function openDb(): Promise<IDBDatabase> {
  if (dbPromise) return dbPromise
  dbPromise = new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, DB_VERSION)
    req.onupgradeneeded = () => {
      const db = req.result
      ensureStore(db, KV_STORE)
      for (const store of Object.values(TABLES)) ensureStore(db, store)
    }
    req.onsuccess = () => resolve(req.result)
    req.onerror = () => reject(req.error)
  })
  return dbPromise
}

export async function idbGet<T>(key: string): Promise<T | null> {
  const db = await openDb()
  return new Promise((resolve, reject) => {
    const tx = db.transaction(KV_STORE, 'readonly')
    const req = tx.objectStore(KV_STORE).get(key)
    req.onsuccess = () => resolve((req.result ?? null) as T | null)
    req.onerror = () => reject(req.error)
  })
}

export async function idbSet<T>(key: string, value: T): Promise<void> {
  const db = await openDb()
  await new Promise<void>((resolve, reject) => {
    const tx = db.transaction(KV_STORE, 'readwrite')
    tx.objectStore(KV_STORE).put(value, key)
    tx.oncomplete = () => resolve()
    tx.onerror = () => reject(tx.error)
  })
}

export async function idbDel(key: string): Promise<void> {
  const db = await openDb()
  await new Promise<void>((resolve, reject) => {
    const tx = db.transaction(KV_STORE, 'readwrite')
    tx.objectStore(KV_STORE).delete(key)
    tx.oncomplete = () => resolve()
    tx.onerror = () => reject(tx.error)
  })
}

export async function idbTableGetAll<T>(table: TableName): Promise<T[]> {
  const db = await openDb()
  return new Promise((resolve, reject) => {
    const tx = db.transaction(table, 'readonly')
    const req = tx.objectStore(table).getAll()
    req.onsuccess = () => resolve((req.result ?? []) as T[])
    req.onerror = () => reject(req.error)
  })
}

function cloneForIdb<T>(value: T): T {
  // Vue reactive proxies cannot be structured-cloned by IndexedDB.  Persist only
  // plain JSON-compatible protocol/UI records.
  return value == null ? value : JSON.parse(JSON.stringify(value)) as T
}

export async function idbTableReplace<T>(table: TableName, entries: Array<[IDBValidKey, T]>): Promise<void> {
  const db = await openDb()
  await new Promise<void>((resolve, reject) => {
    const tx = db.transaction(table, 'readwrite')
    const store = tx.objectStore(table)
    store.clear()
    for (const [key, value] of entries) store.put(cloneForIdb(value), key)
    tx.oncomplete = () => resolve()
    tx.onerror = () => reject(tx.error)
  })
}

export async function idbTableClear(table: TableName): Promise<void> {
  const db = await openDb()
  await new Promise<void>((resolve, reject) => {
    const tx = db.transaction(table, 'readwrite')
    tx.objectStore(table).clear()
    tx.oncomplete = () => resolve()
    tx.onerror = () => reject(tx.error)
  })
}


export async function idbTableGet<T>(table: TableName, key: IDBValidKey): Promise<T | null> {
  const db = await openDb()
  return new Promise((resolve, reject) => {
    const tx = db.transaction(table, 'readonly')
    const req = tx.objectStore(table).get(key)
    req.onsuccess = () => resolve((req.result ?? null) as T | null)
    req.onerror = () => reject(req.error)
  })
}

export async function idbTableGetAllByPrefix<T>(table: TableName, prefix: string): Promise<T[]> {
  const db = await openDb()
  return new Promise((resolve, reject) => {
    const tx = db.transaction(table, 'readonly')
    const store = tx.objectStore(table)
    const values: T[] = []
    const req = store.openCursor()
    req.onsuccess = () => {
      const cursor = req.result
      if (!cursor) {
        resolve(values)
        return
      }
      if (typeof cursor.key === 'string' && cursor.key.startsWith(prefix)) values.push(cursor.value as T)
      cursor.continue()
    }
    req.onerror = () => reject(req.error)
  })
}

export async function idbTableReplaceByPrefix<T>(table: TableName, prefix: string, entries: Array<[IDBValidKey, T]>): Promise<void> {
  const db = await openDb()
  await new Promise<void>((resolve, reject) => {
    const tx = db.transaction(table, 'readwrite')
    const store = tx.objectStore(table)
    const cursorReq = store.openCursor()
    cursorReq.onsuccess = () => {
      const cursor = cursorReq.result
      if (cursor) {
        if (typeof cursor.key === 'string' && cursor.key.startsWith(prefix)) cursor.delete()
        cursor.continue()
      } else {
        for (const [key, value] of entries) store.put(cloneForIdb(value), key)
      }
    }
    cursorReq.onerror = () => reject(cursorReq.error)
    tx.oncomplete = () => resolve()
    tx.onerror = () => reject(tx.error)
  })
}
