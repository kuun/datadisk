import { createI18n } from 'vue-i18n'
import zhCN from './locales/zh-CN.json'
import en  from './locales/en.json'


const i18n = createI18n({
  locale: 'zh', // 设置默认语言
  fallbackLocale: 'en', // 设置备用语言
  messages: {
    zh: zhCN,
    en: en
  }
})

export default i18n
