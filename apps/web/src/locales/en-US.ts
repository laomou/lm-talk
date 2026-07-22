export default {
  nav: {
    chat: 'Chats', contacts: 'Contacts', me: 'Me', openChat: 'Open chats', openContacts: 'Open contacts', openMe: 'Open my settings',
  },
  auth: {
    preparingTitle: 'Preparing LM Talk', preparingDescription: 'Loading security components…',
    login: 'Sign in', register: 'Create', import: 'Import', registerTitle: 'Create identity', importTitle: 'Import identity',
    loginDescription: 'Choose a local identity and enter its passphrase.', registerDescription: 'Back up your identity after creating it.', importDescription: 'Paste an identity export and enter its passphrase.',
    passphrase: 'Passphrase', loginPassphrase: 'Login passphrase', registerPassphrase: 'Registration passphrase', importPassphrase: 'Import passphrase',
    enterPassphrase: 'Enter your passphrase', setPassphrase: 'Set a passphrase', enterImportPassphrase: 'Enter the identity passphrase',
    selectIdentity: 'Choose identity', noLocalIdentityTitle: 'No local identity', noLocalIdentityDescription: 'Create a new identity or import one before signing in.',
    noIdentity: 'No identity yet?', registerLink: 'Create one', importLink: 'Import one', backToLogin: 'Back to sign in',
    identityCreated: 'Identity created', identityCreatedDescription: 'Your identity is stored locally. Back it up before returning to sign in.', downloadIdentity: 'Download identity', verifyImport: 'Verify import', goLogin: 'Sign in',
    passphraseNotice: 'Your passphrase is never uploaded or recoverable. Download your identity file after registration.', importNotice: 'Import requires both the identity text and its passphrase. It cannot be recovered if either is lost.',
    identityText: 'Identity text', importIdentityText: 'Imported identity text', pasteIdentityText: 'Paste an exported identity text',
  },
  me: {
    profile: 'Profile', backup: 'Identity backup', security: 'Security & devices', sync: 'Sync & security', settings: 'Settings', about: 'About', logout: 'Sign out',
    language: 'Language', languageDescription: 'Choose the display language. The change applies immediately and is stored only on this device.',
  },
  language: { zhCN: 'Simplified Chinese', enUS: 'English' },
  common: { save: 'Save', cancel: 'Cancel', close: 'Close', backToMe: 'Back to Me' },
} as const
