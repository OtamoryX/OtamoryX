<template>
  <header
    class="fixed top-0 left-0 right-0 z-50 bg-[#1b1b2f] border-b border-[#2d2d44]">
    <!-- 移动端布局 (< 768px) -->
    <div class="md:hidden h-[calc(env(safe-area-inset-top,0px)+3.5rem)] flex items-end justify-between px-3 pb-2">
      <!-- Logo/标题 -->
      <div class="flex items-center">
        <svg class="w-5 h-5 mr-1.5 text-[#7b68ee]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
            d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.746 0 3.332.477 4.5 1.253v13C20.832 18.477 19.246 18 17.5 18c-1.746 0-3.332.477-4.5 1.253z" />
        </svg>
        <span class="hidden min-[390px]:inline text-base font-semibold text-[#e0e0e0]">OtamoryX</span>
      </div>

      <!-- 搜索和用户按钮 -->
      <div class="flex items-center space-x-1">
        <div class="flex items-center rounded border border-[#3d3d5c] bg-[#2d2d44] p-0.5">
          <button
            class="min-h-9 px-2.5 py-1 text-xs rounded transition-colors"
            :class="viewMode !== 'collections' ? 'bg-[#4b4b70] text-white' : 'text-[#a0a0c0] hover:text-white'"
            @click="emit('set-view-mode', 'single')"
          >单本</button>
          <button
            class="min-h-9 px-2.5 py-1 text-xs rounded transition-colors"
            :class="viewMode === 'collections' ? 'bg-[#4b4b70] text-white' : 'text-[#a0a0c0] hover:text-white'"
            @click="emit('set-view-mode', 'collections')"
          >合集</button>
        </div>
        <!-- 移动端搜索（含筛选数量 badge） -->
        <button @click="emit('toggle-mobile-search')"
          class="relative flex h-10 w-10 items-center justify-center rounded text-[#a0a0a0] hover:text-white hover:bg-white/10 transition-colors">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
          </svg>
          <span v-if="activeFilterCount > 0 || searchQuery"
            class="absolute top-1 right-1 w-2.5 h-2.5 bg-[#7b68ee] rounded-full border border-[#1b1b2f]" />
        </button>

        <RouterLink
          to="/reading-history"
          class="flex h-10 w-10 items-center justify-center rounded text-[#a0a0a0] hover:bg-white/10 hover:text-white transition-colors"
          aria-label="阅读记录"
          title="阅读记录"
        >
          <ClockIcon class="h-5 w-5" />
        </RouterLink>

        <RouterLink
          to="/tags"
          class="flex h-10 w-10 items-center justify-center rounded text-[#a0a0a0] hover:bg-white/10 hover:text-white transition-colors"
          aria-label="标签"
          title="标签"
        >
          <TagIcon class="h-5 w-5" />
        </RouterLink>

        <!-- 移动端用户菜单 -->
        <div ref="mobileUserMenuRef" class="relative">
          <button
            type="button"
            :aria-expanded="showUserMenu"
            aria-haspopup="menu"
            @click.stop="toggleUserMenu"
            class="flex h-10 min-w-10 items-center justify-center px-2 rounded text-[#a0a0a0] hover:text-white hover:bg-white/10 transition-colors text-sm">
            {{ userInitial }}
          </button>
          <Transition name="dropdown">
            <div v-if="showUserMenu"
              class="absolute right-0 z-50 mt-1 w-40 bg-[#1b1b2f] border border-[#2d2d44] rounded shadow-lg overflow-hidden"
              role="menu">
              <div class="px-3 py-2 text-xs text-[#808080] border-b border-[#2d2d44]">{{ userName || '用户' }}</div>
              <RouterLink
                to="/settings"
                @click.stop="showUserMenu = false"
                class="flex touch-manipulation items-center w-full px-3 py-2 text-sm text-[#c0c0d0] hover:bg-[#2d2d44] transition-colors">
                <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                    d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                </svg>
                个人设置
              </RouterLink>
              <RouterLink v-if="authStore.isAdmin" :to="{ name: 'admin-settings', query: { tab: 'system' } }" @click.stop="showUserMenu = false"
                class="flex touch-manipulation items-center w-full px-3 py-2 text-sm text-[#c0c0d0] hover:bg-[#2d2d44] transition-colors">
                <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                    d="M4 6h16M4 12h16M4 18h16" />
                </svg>
                管理设置
              </RouterLink>
              <button type="button" @click.stop.prevent="handleLogout"
                class="flex touch-manipulation items-center w-full px-3 py-2 text-sm text-red-400 hover:bg-[#2d2d44] transition-colors">
                <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" />
                </svg>
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

      <!-- 视图切换 -->
      <div class="flex items-center p-0.5 rounded border border-[#3d3d5c] bg-[#2d2d44] flex-shrink-0">
        <button
          class="px-2.5 py-1 text-xs rounded transition-colors"
          :class="viewMode === 'single' ? 'bg-[#4b4b70] text-white' : 'text-[#a0a0c0] hover:text-white'"
          @click="emit('set-view-mode', 'single')"
        >单本</button>
        <button
          class="px-2.5 py-1 text-xs rounded transition-colors"
          :class="viewMode === 'collections' ? 'bg-[#4b4b70] text-white' : 'text-[#a0a0c0] hover:text-white'"
          @click="emit('set-view-mode', 'collections')"
        >合集</button>
        <button class="px-2.5 py-1 text-xs rounded transition-colors" :class="viewMode === 'versions' ? 'bg-[#4b4b70] text-white' : 'text-[#a0a0c0] hover:text-white'" @click="emit('set-view-mode', 'versions')">多版本</button>
      </div>

      <!-- 搜索框 -->
      <div class="flex-1 min-w-0 max-w-lg">
        <div class="flex items-center gap-2">
          <div class="relative flex-1 min-w-0">
            <input v-model="localSearchQuery" @input="handleSearch" type="text" placeholder="搜索漫画、标签..."
              class="w-full px-3 py-1.5 pl-9 text-sm bg-[#2d2d44] border border-[#3d3d5c] rounded text-[#e0e0e0] placeholder-[#707090] focus:outline-none focus:border-[#7b68ee] focus:bg-[#35355c] transition-all" />
            <svg class="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-[#707090]" fill="none"
              stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
            </svg>
          </div>
          <!-- 高级筛选按钮 -->
          <button
            @click="emit('toggle-advanced-search')"
            :class="[
              'relative flex-shrink-0 p-1.5 rounded border transition-colors',
              showAdvancedSearch
                ? 'bg-[#7b68ee]/20 border-[#7b68ee] text-[#7b68ee]'
                : 'bg-[#2d2d44] border-[#3d3d5c] text-[#a0a0c0] hover:text-white hover:border-[#7b68ee]'
            ]"
            title="高级筛选"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2a1 1 0 01-.293.707L13 13.414V19a1 1 0 01-.553.894l-4 2A1 1 0 017 21v-7.586L3.293 6.707A1 1 0 013 6V4z" />
            </svg>
            <span v-if="activeFilterCount > 0"
              class="absolute -top-1.5 -right-1.5 w-4 h-4 flex items-center justify-center text-[9px] bg-[#7b68ee] text-white rounded-full font-bold">
              {{ activeFilterCount }}
            </span>
          </button>
        </div>
      </div>

      <!-- 右侧按钮组 -->
      <div class="flex items-center space-x-2 flex-shrink-0 min-w-fit">
        <!-- 分类下拉 -->
        <CategoryDropdown
          :selected-category-id="selectedCategoryId"
          :total-archives="totalArchives"
          @select-category="emit('select-category', $event)"
          @edit-category="emit('edit-category', $event)"
          @create-category="emit('create-category')"
        />

        <RouterLink
          to="/reading-history"
          class="inline-flex h-9 items-center justify-center gap-1.5 rounded px-2 text-sm text-[#a0a0a0] hover:bg-white/10 hover:text-white transition-colors"
          aria-label="阅读记录"
          title="阅读记录"
        >
          <ClockIcon class="h-4 w-4" />
          <span class="hidden lg:inline">阅读记录</span>
        </RouterLink>

        <RouterLink
          to="/tags"
          class="inline-flex h-9 items-center justify-center gap-1.5 rounded px-2 text-sm text-[#a0a0a0] hover:bg-white/10 hover:text-white transition-colors"
          aria-label="标签"
          title="标签"
        >
          <TagIcon class="h-4 w-4" />
          <span class="hidden lg:inline">标签</span>
        </RouterLink>

        <!-- 设置按钮 -->
        <button @click="navigateToSettings" title="个人设置" aria-label="个人设置"
          class="p-2 rounded text-[#a0a0a0] hover:text-white hover:bg-white/10 transition-colors">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
          </svg>
        </button>

        <!-- 用户菜单 -->
        <div ref="desktopUserMenuRef" class="relative">
          <button
            type="button"
            :aria-expanded="showUserMenu"
            aria-haspopup="menu"
            @click.stop="toggleUserMenu"
            class="flex items-center space-x-1.5 px-3 py-1.5 rounded text-[#c0c0c0] hover:text-white hover:bg-white/10 transition-colors text-sm">
            <div
              class="w-6 h-6 rounded-full bg-[#7b68ee] flex items-center justify-center text-white text-xs font-semibold">
              {{ userInitial }}
            </div>
            <span class="hidden lg:inline max-w-32 truncate">{{ userName || '用户' }}</span>
          </button>

          <Transition name="dropdown">
            <div v-if="showUserMenu"
              class="absolute right-0 z-50 mt-1 w-40 bg-[#1b1b2f] border border-[#2d2d44] rounded shadow-lg overflow-hidden"
              role="menu">
              <div class="px-3 py-2 text-xs text-[#808080] border-b border-[#2d2d44]">{{ userName || '用户' }}</div>
              <RouterLink to="/settings" @click.stop="showUserMenu = false"
                class="flex touch-manipulation items-center w-full px-3 py-2 text-sm text-[#c0c0d0] hover:bg-[#2d2d44] transition-colors">
                个人设置
              </RouterLink>
              <RouterLink v-if="authStore.isAdmin" :to="{ name: 'admin-settings', query: { tab: 'system' } }" @click.stop="showUserMenu = false"
                class="flex touch-manipulation items-center w-full px-3 py-2 text-sm text-[#c0c0d0] hover:bg-[#2d2d44] transition-colors">
                管理设置
              </RouterLink>
              <button type="button" @click.stop.prevent="handleLogout"
                class="flex touch-manipulation items-center w-full px-3 py-2 text-sm text-red-400 hover:bg-[#2d2d44] transition-colors">
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
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { RouterLink, useRouter } from 'vue-router'
import { ClockIcon, TagIcon } from '@heroicons/vue/24/outline'
import { useAuthStore } from '@/stores/auth'
import CategoryDropdown from '@/components/library/CategoryDropdown.vue'
import type { Category } from '@/types/api'

interface Props {
  searchQuery?: string
  userName?: string
  showAdvancedSearch?: boolean
  activeFilterCount?: number
  selectedCategoryId?: string | null
  totalArchives?: number
  viewMode?: 'single' | 'collections' | 'versions'
}

const props = withDefaults(defineProps<Props>(), {
  searchQuery: '',
  userName: '',
  showAdvancedSearch: false,
  activeFilterCount: 0,
  selectedCategoryId: null,
  totalArchives: 0,
  viewMode: 'single',
})

const emit = defineEmits<{
  'toggle-mobile-search': []
  'search': [query: string]
  'toggle-advanced-search': []
  'select-category': [categoryId: string | null]
  'edit-category': [category: Category]
  'create-category': []
  'set-view-mode': [mode: 'single' | 'collections' | 'versions']
}>()

const router = useRouter()
const authStore = useAuthStore()
const localSearchQuery = ref(props.searchQuery)
const showUserMenu = ref(false)
const mobileUserMenuRef = ref<HTMLElement | null>(null)
const desktopUserMenuRef = ref<HTMLElement | null>(null)

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
  showUserMenu.value = false
  router.push('/settings')
}

const toggleUserMenu = () => {
  showUserMenu.value = !showUserMenu.value
}

const handleLogout = () => {
  authStore.logout()
  showUserMenu.value = false
  router.push('/login')
}

const handleClickOutside = (event: MouseEvent) => {
  const target = event.target as Node | null
  if (!target) return

  const isInsideUserMenu = [mobileUserMenuRef.value, desktopUserMenuRef.value]
    .some((menu) => menu?.contains(target))

  if (showUserMenu.value && !isInsideUserMenu) {
    showUserMenu.value = false
  }
}

onMounted(() => {
  document.addEventListener('click', handleClickOutside)
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
})
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
