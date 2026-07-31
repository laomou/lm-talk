type TestHookHandlers = {
  flushPersist: () => Promise<void>
  appendLog: (line: string) => void
  setDhtDiagnostics: (status: string, history?: string[]) => void
  getPersistMetrics: () => { flushes: number; pending: boolean; running: boolean }
  mergeMessages: (...args: any[]) => any
  resetRtc: () => void
  mergeContactDeviceAndTrustState: (...args: any[]) => any
  contactAllKnownDevicesRevoked: (...args: any[]) => any
  takeMailbox: (...args: any[]) => any
  seedFriendContacts: (...args: any[]) => any
  openFirstContactConversation: (...args: any[]) => any
}

export function installTestHooks(handlers: TestHookHandlers): void {
  if (typeof window === 'undefined') return
  const target = window as any
  target.flushPersistForTests = handlers.flushPersist
  target.appendLogForTests = handlers.appendLog
  target.setDhtDiagnosticsForTests = handlers.setDhtDiagnostics
  target.getPersistMetricsForTests = handlers.getPersistMetrics
  target.mergeMessagesForTests = handlers.mergeMessages
  target.resetRtcForTests = handlers.resetRtc
  target.mergeContactDeviceAndTrustStateForTests = handlers.mergeContactDeviceAndTrustState
  target.contactAllKnownDevicesRevokedForTests = handlers.contactAllKnownDevicesRevoked
  target.takeMailboxForTests = handlers.takeMailbox
  target.seedFriendContactsForTests = handlers.seedFriendContacts
  target.openFirstContactConversationForTests = handlers.openFirstContactConversation
}
