<template>
  <div id="app" class="h-screen w-full flex flex-col">
    <!-- 导航栏 - 只在认证后显示 -->
    <nav v-if="authStore.isAuthenticated" class="nav-glass relative z-30 backdrop-blur-lg bg-slate-900/90 border-b border-white/20 flex-shrink-0 shadow-lg">
      <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div class="flex justify-between h-16">
          <div class="flex items-center">
            <RouterLink 
              to="/library" 
              class="text-xl font-semibold text-white hover:text-blue-300 transition-colors cursor-pointer flex items-center"
            >
              <svg class="w-6 h-6 mr-2 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.746 0 3.332.477 4.5 1.253v13C20.832 18.477 19.246 18 17.5 18c-1.746 0-3.332.477-4.5 1.253z" />
              </svg>
              OtamoryX
            </RouterLink>
          </div>
          <div class="flex items-center space-x-4">
            <RouterLink 
              to="/library" 
              class="text-white/80 hover:text-white hover:bg-white/10 px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200 backdrop-blur-sm border border-transparent hover:border-white/20"
            >
              书库
            </RouterLink>
            <RouterLink 
              to="/settings" 
              class="text-white/80 hover:text-white hover:bg-white/10 px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200 backdrop-blur-sm border border-transparent hover:border-white/20"
            >
              设置
            </RouterLink>
            
            <!-- 管理员菜单 - 只对管理员显示 -->
            <div v-if="authStore.isAdmin" class="relative">
              <button
                @click="showAdminMenu = !showAdminMenu"
                data-menu="admin"
                class="flex items-center text-white/80 hover:text-white hover:bg-white/10 px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200 backdrop-blur-sm border border-transparent hover:border-white/20"
              >
                <svg class="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                </svg>
                管理
              </button>
              <div v-if="showAdminMenu" data-dropdown="admin" class="absolute right-0 mt-2 w-48 bg-white/10 backdrop-blur-lg border border-white/20 rounded-lg shadow-2xl z-20">
                <div class="py-1">
                  <RouterLink
                    to="/admin/users"
                    @click="showAdminMenu = false"
                    class="block px-4 py-2 text-sm text-white hover:bg-white/20 rounded-lg mx-1 mt-1 transition-colors"
                  >
                    用户管理
                  </RouterLink>
                  <RouterLink
                    to="/admin/plugins"
                    @click="showAdminMenu = false"
                    class="block px-4 py-2 text-sm text-white hover:bg-white/20 rounded-lg mx-1 mb-1 transition-colors"
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
                data-menu="user"
                class="flex items-center text-white/80 hover:text-white hover:bg-white/10 px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200 backdrop-blur-sm border border-transparent hover:border-white/20"
              >
                <svg class="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
                </svg>
                {{ authStore.user?.username || '用户' }}
              </button>
              <div v-if="showUserMenu" data-dropdown="user" class="absolute right-0 mt-2 w-48 bg-white/10 backdrop-blur-lg border border-white/20 rounded-lg shadow-2xl z-20">
                <div class="py-1">
                  <div class="px-4 py-2 text-sm text-white/70 border-b border-white/20 mx-1 mt-1">
                    <div class="font-medium text-white">{{ authStore.user?.username }}</div>
                    <div class="text-xs text-white/60">{{ authStore.user?.role === 'admin' ? '管理员' : '普通用户' }}</div>
                  </div>
                  <button
                    @click="handleLogout"
                    class="flex items-center w-full text-left px-4 py-2 text-sm text-white hover:bg-white/20 rounded-lg mx-1 mb-1 mt-2 transition-colors"
                  >
                    <svg class="w-4 h-4 mr-2 text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" />
                    </svg>
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
  const target = event.target as Element
  
  // 检查点击是否在用户菜单区域内
  const userMenuButton = document.querySelector('[data-menu="user"]')
  const userMenuDropdown = document.querySelector('[data-dropdown="user"]')
  
  if (showUserMenu.value && userMenuButton && userMenuDropdown) {
    if (!userMenuButton.contains(target) && !userMenuDropdown.contains(target)) {
      showUserMenu.value = false
    }
  }
  
  // 检查点击是否在管理员菜单区域内
  const adminMenuButton = document.querySelector('[data-menu="admin"]')
  const adminMenuDropdown = document.querySelector('[data-dropdown="admin"]')
  
  if (showAdminMenu.value && adminMenuButton && adminMenuDropdown) {
    if (!adminMenuButton.contains(target) && !adminMenuDropdown.contains(target)) {
      showAdminMenu.value = false
    }
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
/* 玻璃形态导航栏样式 */
.nav-glass {
  background: linear-gradient(135deg, 
    rgba(15, 23, 42, 0.95) 0%, 
    rgba(30, 41, 59, 0.9) 50%, 
    rgba(15, 23, 42, 0.95) 100%
  );
  border-bottom: 1px solid rgba(255, 255, 255, 0.2);
  box-shadow: 
    0 8px 32px rgba(0, 0, 0, 0.3),
    inset 0 1px 0 rgba(255, 255, 255, 0.1);
}

/* 活跃链接样式 */
.router-link-active {
  @apply bg-blue-500/20 text-blue-200 border-blue-400/30;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.1);
}

/* 菜单下拉动画 */
.nav-glass div[class*="absolute"] {
  animation: slideDown 0.2s ease-out;
}

@keyframes slideDown {
  from {
    opacity: 0;
    transform: translateY(-10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* 响应式调整 */
@media (max-width: 768px) {
  .nav-glass {
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
  }
}
</style>