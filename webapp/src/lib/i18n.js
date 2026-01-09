import zh from '../i18n/locales/zh-CN.json'
import en from '../i18n/locales/en.json'

const messages = {
  zh,
  en
}

let currentLocale = localStorage.getItem('locale') || 'zh'

export const setLocale = (locale) => {
  currentLocale = locale
  localStorage.setItem('locale', locale)
}

const resolvePath = (obj, path) => {
  return path.split('.').reduce((acc, key) => (acc ? acc[key] : undefined), obj)
}

export const t = (key, params = {}) => {
  const message = resolvePath(messages[currentLocale], key) ?? resolvePath(messages.en, key) ?? key
  if (typeof message !== 'string') {
    return String(message ?? key)
  }
  return Object.keys(params).reduce(
    (result, paramKey) => result.replace(new RegExp(`{${paramKey}}`, 'g'), params[paramKey]),
    message
  )
}
