<template>
  <header
    class="fixed top-0 left-0 right-0 z-50 bg-[#1b1b2f] border-b border-[#2d2d44]">
    <!-- 移动端布局 (< 768px) -->
    <div class="md:hidden h-14 flex items-center justify-between px-3">
      <!-- Logo/标题 -->
      <div class="flex items-center">
        <svg class="w-5 h-5 mr-1.5 text-[#7b68ee]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
            d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.746 0 3.332.477 4.5 1.253v13C20.832 18.477 19.246 18 17.5 18c-1.746 0-3.332.477-4.5 1.253z" />
        </svg>
        <span class="text-base font-semibold text-[#e0e0e0]">OtamoryX</span>
      </div>

      <!-- 搜索和用户按钮 -->
      <div class="flex items-center space-x-1">
        <button @click="emit('toggle-mobile-search')"
          class="p-2 rounded text-[#a0a0a0] hover:text-white hover:bg-white/10 transition-colors">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
          </svg>
        </button>

        <!-- 移动端用户菜单 -->
        <div class="relative">
          <button @click="toggleUserMenu"
            class="flex items-center px-2 py-1.5 rounded text-[#a0a0a0] hover:text-white hover:bg-white/10 transition-colors text-sm">
            {{ userInitial }}
          </button>
          <Transition name="dropdown">
            <div v-if="showUserMenu"
              class="absolute right-0 mt-1 w-40 bg-[#1b1b2f] border border-[#2d2d44] rounded shadow-lg overflow-hidden">
              <div class="px-3 py-2 text-xs text-[#808080] border-b border-[#2d2d44]">{{ userName || '用户' }}</div>
              <button @click="handleLogout"
                class="flex items-center w-full px-3 py-2 text-sm text-red-400 hover:bg-[#2d2d44] transition-colors">
                退出登录
              </button>
            </div>
          </Transition>
        </div>
      </div>
    </div>

    <!-- 桌面端布局 (>= 768px) -->
    <div class="hidden md:flex h-14 items-center justify-between px-4 gap-4">
      <!-- Logo/标题 -->
      <div class="flex items-center flex-shrink-0">
        <svg class="w-5 h-5 mr-1.5 text-[#7b68ee]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
            d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.746 0 3.332.477 4.5 1.253v13C20.832 18.477 19.246 18 17.5 18c-1.746 0-3.332.477-4.5 1.253z" />
        </svg>
        <span class="text-base font-semibold text-[#e0e0e0] whitespace-nowrap">OtamoryX</span>
      </div>

      <!-- 搜索框 -->
      <div class="flex-1 max-w-lg">
        <div class="relative">
          <input v-model="localSearchQuery" @input="handleSearch" type="text" placeholder="搜索漫画、标签..."
            class="w-full px-3 py-1.5 pl-9 text-sm bg-[#2d2d44] border border-[#3d3d5c] rounded text-[#e0e0e0] placeholder-[#707090] focus:outline-none focus:border-[#7b68ee] focus:bg-[#35355c] transition-all" />
          <svg class="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-[#707090]" fill="none"
            stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
          </svg>
        </div>
      </div>

      <!-- 右侧按钮组 -->
      <div class="flex items-center space-x-2 flex-shrink-0">
        <!-- 设置按钮 -->
        <button @click="navigateToSettings"
          class="p-2 rounded text-[#a0a0a0] hover:text-white hover:bg-white/10 transition-colors">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
          </svg>
        </button>

        <!-- 用户菜单 -->
        <div class="relative">
          <button @click="toggleUserMenu"
            class="flex items-center space-x-1.5 px-3 py-1.5 rounded text-[#c0c0c0] hover:text-white hover:bg-white/10 transition-colors text-sm">
            <div
              class="w-6 h-6 rounded-full bg-[#7b68ee] flex items-center justify-center text-white text-xs font-semibold">
              {{ userInitial }}
            </div>
            <span>{{ userName || '用户' }}</span>
          </button>

          <Transition name="dropdown">
            <div v-if="showUserMenu"
              class="absolute right-0 mt-1 w-40 bg-[#1b1b2f] border border-[#2d2d44] rounded shadow-lg overflow-hidden">
              <div class="px-3 py-2 text-xs text-[#808080] border-b border-[#2d2d44]">{{ userName || '用户' }}</div>
              <button @click="handleLogout"
                class="flex items-center w-full px-3 py-2 text-sm text-red-400 hover:bg-[#2d2d44] transition-colors">
                退出登录
              </button>
            </div>
          </Transition>
        </div>
      </div>
    </div>
  </header>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useRouter } from 'vue-router'

interface Props {
  searchQuery?: string
  userName?: string
}

const props = withDefaults(defineProps<Props>(), {
  searchQuery: '',
  userName: ''
})

const emit = defineEmits<{
  'toggle-mobile-search': []
  'search': [query: string]
}>()

const router = useRouter()
const localSearchQuery = ref(props.searchQuery)
const showUserMenu = ref(false)

const userInitial = computed(() => {
  return props.userName ? props.userName.charAt(0).toUpperCase() : 'U'
})

watch(() => props.searchQuery, (newVal) => {
  localSearchQuery.value = newVal
})

const handleSearch = () => {
  emit('search', localSearchQuery.value)
}

const navigateToSettings = () => {
  router.push('/settings')
}

const toggleUserMenu = () => {
  showUserMenu.value = !showUserMenu.value
}

const handleLogout = () => {
  showUserMenu.value = false
  router.push('/login')
}

const handleClickOutside = (event: MouseEvent) => {
  const target = event.target as HTMLElement
  if (showUserMenu.value && !target.closest('.relative')) {
    showUserMenu.value = false
  }
}

if (typeof window !== 'undefined') {
  document.addEventListener('click', handleClickOutside)
}
</script>

<style scoped>
.dropdown-enter-active,
.dropdown-leave-active {
  transition: all 0.15s ease;
}

.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}

.dropdown-enter-to,
.dropdown-leave-from {
  opacity: 1;
  transform: translateY(0);
}
</style>
