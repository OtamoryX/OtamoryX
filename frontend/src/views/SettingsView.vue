<template>
  <div class="settings-view">
    <h1 class="text-2xl font-bold text-gray-900 mb-6">设置</h1>
    
    <div class="max-w-2xl">
      <div class="bg-white shadow rounded-lg p-6">
        <h2 class="text-lg font-medium text-gray-900 mb-4">服务器设置</h2>
        
        <div class="space-y-4">
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-2">
              服务器地址
            </label>
            <input
              v-model="serverUrl"
              type="url"
              placeholder="http://localhost:3000"
              class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>
          
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-2">
              API 密钥
            </label>
            <input
              v-model="apiKey"
              type="password"
              placeholder="输入 API 密钥"
              class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>
          
          <div class="pt-4">
            <button
              @click="saveSettings"
              class="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              保存设置
            </button>
          </div>
        </div>
      </div>
      
      <div class="bg-white shadow rounded-lg p-6 mt-6">
        <h2 class="text-lg font-medium text-gray-900 mb-4">阅读器设置</h2>
        
        <div class="space-y-4">
          <div class="flex items-center">
            <input
              id="dark-mode"
              v-model="darkMode"
              type="checkbox"
              class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
            />
            <label for="dark-mode" class="ml-2 block text-sm text-gray-900">
              暗色模式
            </label>
          </div>
          
          <div class="flex items-center">
            <input
              id="auto-scroll"
              v-model="autoScroll"
              type="checkbox"
              class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
            />
            <label for="auto-scroll" class="ml-2 block text-sm text-gray-900">
              自动滚动
            </label>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const serverUrl = ref('http://localhost:3000')
const apiKey = ref('')
const darkMode = ref(false)
const autoScroll = ref(false)

const saveSettings = () => {
  // TODO: 实现设置保存逻辑
  console.log('Saving settings:', {
    serverUrl: serverUrl.value,
    apiKey: apiKey.value,
    darkMode: darkMode.value,
    autoScroll: autoScroll.value,
  })
  
  // 保存到 localStorage
  localStorage.setItem('otamoryx-settings', JSON.stringify({
    serverUrl: serverUrl.value,
    apiKey: apiKey.value,
    darkMode: darkMode.value,
    autoScroll: autoScroll.value,
  }))
  
  alert('设置已保存')
}
</script>