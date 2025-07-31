import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export const useAuthStore = defineStore('auth', () => {
  const apiKey = ref<string>('')
  const isAuthenticated = computed(() => !!apiKey.value)
  
  const login = async (key: string) => {
    apiKey.value = key
    localStorage.setItem('apiKey', key)
  }
  
  const logout = () => {
    apiKey.value = ''
    localStorage.removeItem('apiKey')
  }

  // 初始化时从localStorage恢复认证状态
  const initAuth = () => {
    const savedKey = localStorage.getItem('apiKey')
    if (savedKey) {
      apiKey.value = savedKey
    }
  }
  
  return { 
    apiKey, 
    isAuthenticated, 
    login, 
    logout, 
    initAuth 
  }
})