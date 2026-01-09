import './assets/main.css'

// import "v3-easyui/dist/themes/default/easyui.css";
// import "v3-easyui/dist/themes/icon.css";
// import "v3-easyui/dist/themes/vue.css";
// import EasyUI from "v3-easyui";
import ElementPlus from 'element-plus'
import zhCn from 'element-plus/es/locale/lang/zh-cn'
import 'element-plus/dist/index.css'
import 'vxe-table/lib/style.css'

import { createApp } from 'vue'
import { createPinia } from 'pinia'
import 'vue-simple-uploader/dist/style.css'
import uploader from 'vue-simple-uploader'
import { Icon } from '@iconify/vue'

import App from './App.vue'
import router from './router'
import VXETable from "vxe-table";
import i18n from './i18n'

const app = createApp(App)

app.use(ElementPlus, {
  locale: zhCn
})
// app.use(EasyUI)
app.use(VXETable)
app.use(createPinia())
app.use(router)
app.use(uploader)
app.use(i18n)
app.component('Icon', Icon)

app.mount('#app')
