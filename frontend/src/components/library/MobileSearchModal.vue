<template>
  <!-- 全屏模态框 - 仅移动端 -->
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="show" class="fixed inset-0 z-50 bg-white dark:bg-gray-900 md:hidden">
        <!-- 顶部搜索栏 -->
        <div class="sticky top-0 bg-white dark:bg-gray-900 border-b border-gray-200 dark:border-gray-700 px-4 py-3">
          <div class="flex items-center space-x-3">
            <!-- 返回按钮 -->
            <button @click="handleClose"
              class="p-2 -ml-2 rounded-lg text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
              aria-label="返回">
              <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
              </svg>
            </button>

            <!-- 大搜索框 -->
            <div class="flex-1 relative">
              <input ref="searchInput" v-model="searchQuery" @input="handleInput" @keyup.enter="handleSearchSubmit"
                type="text" placeholder="搜索书籍、作者、标签..."
                class="w-full px-4 py-3 text-lg bg-gray-100 dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent text-gray-900 dark:text-white placeholder-gray-500 dark:placeholder-gray-400" />

              <!-- 清除按钮 -->
              <button v-if="searchQuery" @click="clearSearch"
                class="absolute right-3 top-1/2 transform -translate-y-1/2 p-1 rounded-full text-gray-400 hover:text-gray-600 dark:hover:text-gray-300">
                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>
          </div>
        </div>

        <!-- 内容区域 -->
        <div class="overflow-y-auto h-[calc(100vh-73px)]">
          <!-- 标签自动完成建议 -->
          <div v-if="searchQuery && suggestions.length > 0" class="px-4 py-3">
            <h3 class="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-2">
              建议
            </h3>
            <div class="space-y-1">
              <button v-for="(suggestion, index) in suggestions" :key="index" @click="selectSuggestion(suggestion)"
                class="flex items-center w-full px-4 py-3 text-left text-gray-900 dark:text-white bg-gray-50 dark:bg-gray-800 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors">
                <svg class="w-5 h-5 mr-3 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                    d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                </svg>
                <span>{{ suggestion }}</span>
              </button>
            </div>
          </div>

          <!-- 搜索历史 -->
          <div v-if="!searchQuery && searchHistory.length > 0" class="px-4 py-3">
            <div class="flex items-center justify-between mb-2">
              <h3 class="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                最近搜索
              </h3>
              <button @click="clearHistory"
                class="text-xs text-blue-600 dark:text-blue-400 hover:text-blue-700 dark:hover:text-blue-300 font-medium">
                清除
              </button>
            </div>
            <div class="space-y-1">
              <button v-for="(item, index) in recentHistory" :key="index" @click="selectHistory(item)"
                class="flex items-center justify-between w-full px-4 py-3 text-left text-gray-900 dark:text-white bg-gray-50 dark:bg-gray-800 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors group">
                <div class="flex items-center flex-1 min-w-0">
                  <svg class="w-5 h-5 mr-3 text-gray-400 flex-shrink-0" fill="none" stroke="currentColor"
                    viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                      d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                  <span class="truncate">{{ item }}</span>
                </div>
                <button @click.stop="removeHistoryItem(index)"
                  class="ml-2 p-1 opacity-0 group-hover:opacity-100 transition-opacity">
                  <svg class="w-4 h-4 text-gray-400 hover:text-red-500" fill="none" stroke="currentColor"
                    viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </button>
            </div>
          </div>

          <!-- 空状态 -->
          <div v-if="!searchQuery && searchHistory.length === 0"
            class="flex flex-col items-center justify-center h-full text-center px-4 py-12">
            <svg class="w-16 h-16 text-gray-300 dark:text-gray-600 mb-4" fill="none" stroke="currentColor"
              viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
            </svg>
            <p class="text-gray-500 dark:text-gray-400 text-lg">搜索书籍、作者或标签</p>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, watch, nextTick, computed, onMounted, onUnmounted } from 'vue'

interface Props {
  show: boolean
  initialQuery?: string
}

const props = withDefaults(defineProps<Props>(), {
  show: false,
  initialQuery: ''
})

const emit = defineEmits<{
  'close': []
  'search': [query: string]
}>()

const searchInput = ref<HTMLInputElement | null>(null)
const searchQuery = ref('')
const searchHistory = ref<string[]>([])
const suggestions = ref<string[]>([])

// 最近 5 条历史记录
const recentHistory = computed(() => {
  return searchHistory.value.slice(0, 5)
})

// 加载搜索历史
const loadSearchHistory = () => {
  try {
    const stored = localStorage.getItem('search-history')
    if (stored) {
      searchHistory.value = JSON.parse(stored)
    }
  } catch (error) {
    console.error('Failed to load search history:', error)
    searchHistory.value = []
  }
}

// 保存搜索历史
const saveSearchHistory = () => {
  try {
    localStorage.setItem('search-history', JSON.stringify(searchHistory.value))
  } catch (error) {
    console.error('Failed to save search history:', error)
  }
}

// 添加到搜索历史
const addToHistory = (query: string) => {
  if (!query.trim()) return

  // 去重：移除已存在的相同项
  const filtered = searchHistory.value.filter(item => item !== query)

  // 添加到开头
  searchHistory.value = [query, ...filtered]

  // 最多保存 10 条
  if (searchHistory.value.length > 10) {
    searchHistory.value = searchHistory.value.slice(0, 10)
  }

  saveSearchHistory()
}

// 清除搜索框
const clearSearch = () => {
  searchQuery.value = ''
  suggestions.value = []
}

// 清除历史记录
const clearHistory = () => {
  searchHistory.value = []
  saveSearchHistory()
}

// 移除单条历史记录
const removeHistoryItem = (index: number) => {
  searchHistory.value.splice(index, 1)
  saveSearchHistory()
}

// 选择历史记录
const selectHistory = (item: string) => {
  searchQuery.value = item
  handleSearchSubmit()
}

// 选择建议
const selectSuggestion = (suggestion: string) => {
  searchQuery.value = suggestion
  handleSearchSubmit()
}

// 处理输入
const handleInput = () => {
  // 这里可以实现标签自动完成逻辑
  // 暂时使用简单的模拟数据
  if (searchQuery.value.trim()) {
    // 模拟建议（实际应该从 API 获取）
    suggestions.value = [
      `${searchQuery.value} - 书籍`,
      `${searchQuery.value} - 作者`,
      `${searchQuery.value} - 标签`
    ].slice(0, 3)
  } else {
    suggestions.value = []
  }
}

// 提交搜索
const handleSearchSubmit = () => {
  const query = searchQuery.value.trim()
  if (query) {
    addToHistory(query)
    emit('search', query)
    handleClose()
  }
}

// 关闭模态框
const handleClose = () => {
  emit('close')
}

// ESC 键关闭
const handleKeydown = (event: KeyboardEvent) => {
  if (event.key === 'Escape' && props.show) {
    handleClose()
  }
}

// 监听 show 变化，自动聚焦
watch(() => props.show, async (newVal) => {
  if (newVal) {
    // 加载初始查询
    searchQuery.value = props.initialQuery || ''

    // 自动聚焦
    await nextTick()
    searchInput.value?.focus()

    // 如果有初始查询，触发建议
    if (searchQuery.value) {
      handleInput()
    }
  } else {
    // 关闭时清空
    searchQuery.value = ''
    suggestions.value = []
  }
})

// 监听初始查询变化
watch(() => props.initialQuery, (newVal) => {
  if (props.show && newVal) {
    searchQuery.value = newVal
    handleInput()
  }
})

// 组件挂载时加载历史
onMounted(() => {
  loadSearchHistory()
  document.addEventListener('keydown', handleKeydown)
})

// 组件卸载时移除监听
onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown)
})
</script>

<style scoped>
/* 模态框动画 */
.modal-enter-active,
.modal-leave-active {
  transition: all 0.3s ease;
}

.modal-enter-from {
  opacity: 0;
  transform: translateY(100%);
}

.modal-leave-to {
  opacity: 0;
  transform: translateY(-100%);
}

.modal-enter-to,
.modal-leave-from {
  opacity: 1;
  transform: translateY(0);
}

/* 防止背景滚动 */
body:has(.modal-enter-active),
body:has(.modal-leave-active) {
  overflow: hidden;
}
</style>
