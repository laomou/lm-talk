type SnapshotRequest = {
  id: number
  entries: Array<[string, unknown]>
}

self.onmessage = (event: MessageEvent<SnapshotRequest>) => {
  const { id, entries } = event.data
  try {
    self.postMessage({
      id,
      ok: true,
      signatures: entries.map(([key, value]) => [key, JSON.stringify(value)]),
    })
  } catch (error) {
    self.postMessage({
      id,
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    })
  }
}
