import ElementPlus from 'element-plus'
import zhCn from 'element-plus/es/locale/lang/zh-cn'
import 'element-plus/dist/index.css'

import { createApp } from 'vue'

import SetupWizard from './setup/SetupWizard.vue'

const app = createApp(SetupWizard)

app.use(ElementPlus, {
    locale: zhCn
  })
app.mount('#app')