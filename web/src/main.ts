import { createApp } from 'vue'
import { createPinia } from 'pinia'
import ElementPlus from 'element-plus'
import 'element-plus/dist/index.css'
import VueApexCharts from 'vue3-apexcharts'
import '../fonts/fonts.css'
import App from './app/App.vue'
import { router } from './app/router'

createApp(App).use(createPinia()).use(router).use(ElementPlus).use(VueApexCharts).mount('#app')
