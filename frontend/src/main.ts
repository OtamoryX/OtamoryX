import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { VueQueryPlugin, QueryClient } from '@tanstack/vue-query'
import App from './App.vue'
import router from './router'
import './assets/main.css'

const app = createApp(App)

// 创建 Pinia 实例
const pinia = createPinia()

// 创建 Query Client
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 5 * 60 * 1000, // 5分钟
      retry: 2,
      refetchOnWindowFocus: false,
    },
    mutations: {
      retry: 1,
    },
  },
})

// 注册插件
app.use(pinia)
app.use(router)
app.use(VueQueryPlugin, { queryClient })

// 全局错误处理
app.config.errorHandler = (error, instance, info) => {
  console.error('Global error:', error, info)
}

// 挂载应用
app.mount('#app')