import { createI18n } from 'vue-i18n'
import enUS from './locales/en-US'
import zhCN from './locales/zh-CN'

export const SUPPORTED_LOCALES = ['zh-CN', 'en-US'] as const
export type SupportedLocale = typeof SUPPORTED_LOCALES[number]
const STORAGE_KEY = 'lm-talk-locale-v1'

function browserLocale(): SupportedLocale {
  if (typeof navigator === 'undefined') return 'zh-CN'
  return navigator.language.toLowerCase().startsWith('zh') ? 'zh-CN' : 'en-US'
}

function initialLocale(): SupportedLocale {
  if (typeof localStorage === 'undefined') return browserLocale()
  const saved = localStorage.getItem(STORAGE_KEY)
  return SUPPORTED_LOCALES.includes(saved as SupportedLocale) ? saved as SupportedLocale : browserLocale()
}

export const i18n = createI18n({
  legacy: false,
  globalInjection: true,
  locale: initialLocale(),
  fallbackLocale: 'zh-CN',
  messages: { 'zh-CN': zhCN, 'en-US': enUS },
})

export function setLocale(locale: SupportedLocale) {
  i18n.global.locale.value = locale
  if (typeof localStorage !== 'undefined') localStorage.setItem(STORAGE_KEY, locale)
  if (typeof document !== 'undefined') document.documentElement.lang = locale
}

if (typeof document !== 'undefined') document.documentElement.lang = i18n.global.locale.value
