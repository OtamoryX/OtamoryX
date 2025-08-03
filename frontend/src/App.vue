<template>
  <div id="app" class="h-screen w-full flex flex-col">
    <!-- 导航栏 - 只在认证后显示 -->
    <nav v-if="authStore.isAuthenticated" class="bg-white shadow-sm border-b flex-shrink-0">
      <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div class="flex justify-between h-16">
          <div class="flex items-center">
            <RouterLink 
              to="/library" 
              class="text-xl font-semibold text-gray-900 hover:text-blue-600 transition-colors cursor-pointer"
            >
              OtamoryX
            </RouterLink>
          </div>
          <div class="flex items-center space-x-4">
            <RouterLink 
              to="/library" 
              class="text-gray-700 hover:text-gray-900 px-3 py-2 rounded-md text-sm font-medium"
            >
              书库
            </RouterLink>
            <RouterLink 
              to="/settings" 
              class="text-gray-700 hover:text-gray-900 px-3 py-2 rounded-md text-sm font-medium"
            >
              设置
            </RouterLink>
            
            <!-- 管理员菜单 - 只对管理员显示 -->
            <div v-if="authStore.isAdmin" class="relative">
              <button
                @click="showAdminMenu = !showAdminMenu"
                class="flex items-center text-gray-700 hover:text-gray-900 px-3 py-2 rounded-md text-sm font-medium"
              >
                <svg class="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                </svg>
                管理
              </button>
              <div v-if="showAdminMenu" class="absolute right-0 mt-2 w-48 bg-white rounded-md shadow-lg z-50">
                <div class="py-1">
                  <RouterLink
                    to="/admin/users"
                    @click="showAdminMenu = false"
                    class="block px-4 py-2 text-sm text-gray-700 hover:bg-gray-100"
                  >
                    用户管理
                  </RouterLink>
                  <RouterLink
                    to="/admin/plugins"
                    @click="showAdminMenu = false"
                    class="block px-4 py-2 text-sm text-gray-700 hover:bg-gray-100"
                  >
                    插件管理
                  </RouterLink>
                </div>
              </div>
            </div>
            
            <!-- 用户菜单 -->
            <div class="relative">
              <button
                @click="showUserMenu = !showUserMenu"
                class="flex items-center text-gray-700 hover:text-gray-900 px-3 py-2 rounded-md text-sm font-medium"
              >
                <svg class="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
                </svg>
                {{ authStore.user?.username || '用户' }}
              </button>
              <div v-if="showUserMenu" class="absolute right-0 mt-2 w-48 bg-white rounded-md shadow-lg z-50">
                <div class="py-1">
                  <div class="px-4 py-2 text-sm text-gray-500 border-b">
                    <div class="font-medium">{{ authStore.user?.username }}</div>
                    <div class="text-xs">{{ authStore.user?.role === 'admin' ? '管理员' : '普通用户' }}</div>
                  </div>
                  <button
                    @click="handleLogout"
                    class="block w-full text-left px-4 py-2 text-sm text-gray-700 hover:bg-gray-100"
                  >
                    退出登录
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </nav>
    
    <main class="flex-1 w-full overflow-hidden">
      <RouterView class="w-full h-full" />
    </main>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { RouterLink, RouterView, useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const router = useRouter()
const authStore = useAuthStore()
const showUserMenu = ref(false)
const showAdminMenu = ref(false)

const handleLogout = () => {
  authStore.logout()
  showUserMenu.value = false
  showAdminMenu.value = false
  router.push('/login')
}

// 点击外部关闭菜单
const handleClickOutside = (event: Event) => {
  if (showUserMenu.value) {
    showUserMenu.value = false
  }
  if (showAdminMenu.value) {
    showAdminMenu.value = false
  }
}

onMounted(() => {
  // 初始化认证状态
  authStore.initAuth()
  
  // 添加点击外部事件监听
  document.addEventListener('click', handleClickOutside)
})
</script>

<style scoped>
.router-link-active {
  @apply text-blue-600 bg-blue-50;
}
</style>