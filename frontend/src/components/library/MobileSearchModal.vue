<template>
  <!-- 全屏模态框 - 仅移动端 -->
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="show" class="fixed inset-0 z-50 bg-[var(--bg-primary)] md:hidden flex flex-col">

        <!-- 顶部搜索栏 -->
        <div class="flex-shrink-0 bg-[var(--bg-primary)] border-b border-[var(--border)] px-4 py-3">
          <div class="flex items-center space-x-3">
            <button @click="handleClose"
              class="p-2 -ml-2 rounded text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] transition-colors"
              aria-label="返回">
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
              </svg>
            </button>
            <div class="flex-1 relative">
              <input ref="searchInput" v-model="query" @keyup.enter="handleApply"
                type="text" placeholder="搜索漫画、标签..."
                class="w-full px-4 py-2.5 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded text-[var(--text-primary)] placeholder-[var(--text-tertiary)] focus:outline-none focus:border-[var(--accent)] focus:ring-1 focus:ring-[var(--accent)]/30 transition-colors" />
              <button v-if="query" @click="query = ''"
                class="absolute right-3 top-1/2 -translate-y-1/2 p-1 rounded text-[var(--text-tertiary)] hover:text-[var(--text-primary)]">
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>
            <!-- 筛选按钮 -->
            <button
              @click="showFilters = !showFilters"
              :class="[
                'relative flex-shrink-0 p-2 rounded border transition-colors',
                showFilters || activeFilterCount > 0
                  ? 'bg-[var(--accent)]/20 border-[var(--accent)] text-[var(--accent)]'
                  : 'bg-[var(--bg-tertiary)] border-[var(--border)] text-[var(--text-secondary)]'
              ]"
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2a1 1 0 01-.293.707L13 13.414V19a1 1 0 01-.553.894l-4 2A1 1 0 017 21v-7.586L3.293 6.707A1 1 0 013 6V4z" />
              </svg>
              <span v-if="activeFilterCount > 0"
                class="absolute -top-1.5 -right-1.5 w-4 h-4 flex items-center justify-center text-[9px] bg-[var(--accent)] text-white rounded-full font-bold">
                {{ activeFilterCount }}
              </span>
            </button>
          </div>
        </div>

        <!-- 高级筛选面板（展开时） -->
        <Transition
          enter-active-class="transition-all duration-200 ease-out"
          enter-from-class="opacity-0 max-h-0"
          enter-to-class="opacity-100 max-h-[500px]"
          leave-active-class="transition-all duration-150 ease-in"
          leave-from-class="opacity-100 max-h-[500px]"
          leave-to-class="opacity-0 max-h-0"
        >
          <div v-if="showFilters" class="flex-shrink-0 overflow-hidden border-b border-[var(--border)] bg-[var(--bg-secondary)] px-4 py-3 space-y-3">

            <!-- 标签筛选 -->
            <div>
              <label class="block text-xs text-[var(--text-tertiary)] mb-1.5">标签筛选</label>
              <div class="relative">
                <input
                  v-model="tagInput"
                  type="text"
                  placeholder="输入标签名..."
                  class="w-full px-3 py-2 text-sm bg-[var(--bg-tertiary)] border border-[var(--border)] rounded text-[var(--text-primary)] placeholder-[var(--text-tertiary)] focus:outline-none focus:border-[var(--accent)] transition-colors"
                  @focus="showTagSuggest = true"
                  @blur="handleTagBlur"
                />
                <div v-if="showTagSuggest && tagSuggestions.length > 0"
                  class="absolute left-0 right-0 z-10 mt-1 max-h-[150px] overflow-y-auto bg-[var(--bg-card)] border border-[var(--border)] rounded shadow-lg">
                  <button v-for="tag in tagSuggestions" :key="`${tag.namespace}:${tag.name}`"
                    class="flex items-center w-full px-3 py-2 text-sm hover:bg-[var(--bg-tertiary)] transition-colors"
                    @mousedown.prevent="addTag(tag)">
                    <span class="text-[var(--accent)] mr-1">{{ tag.namespace }}:</span>
                    <span class="text-[var(--text-primary)]">{{ tag.name }}</span>
                  </button>
                </div>
              </div>
              <div v-if="selectedTags.length > 0" class="flex flex-wrap gap-1.5 mt-2">
                <span v-for="tag in selectedTags" :key="tag"
                  class="inline-flex items-center px-2 py-1 rounded text-xs bg-[var(--accent)]/20 text-[var(--accent)] border border-[var(--accent)]/30">
                  {{ tag }}
                  <button class="ml-1" @click="removeTag(tag)">×</button>
                </span>
              </div>
            </div>

            <!-- 页数范围 -->
            <div>
              <label class="block text-xs text-[var(--text-tertiary)] mb-1.5">页数范围</label>
              <div class="flex items-center gap-2">
                <input v-model.number="localFilters.minPages" type="number" min="0" placeholder="最少页"
                  class="flex-1 px-3 py-2 text-sm bg-[var(--bg-tertiary)] border border-[var(--border)] rounded text-[var(--text-primary)] placeholder-[var(--text-tertiary)] focus:outline-none focus:border-[var(--accent)] transition-colors" />
                <span class="text-[var(--text-tertiary)] text-xs">~</span>
                <input v-model.number="localFilters.maxPages" type="number" min="0" placeholder="最多页"
                  class="flex-1 px-3 py-2 text-sm bg-[var(--bg-tertiary)] border border-[var(--border)] rounded text-[var(--text-primary)] placeholder-[var(--text-tertiary)] focus:outline-none focus:border-[var(--accent)] transition-colors" />
              </div>
            </div>

            <!-- 添加时间 -->
            <div>
              <label class="block text-xs text-[var(--text-tertiary)] mb-1.5">添加时间</label>
              <div class="flex items-center gap-2">
                <input v-model="localFilters.createdAfter" type="date"
                  class="flex-1 px-2 py-2 text-sm bg-[var(--bg-tertiary)] border border-[var(--border)] rounded text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] transition-colors" />
                <span class="text-[var(--text-tertiary)] text-xs">~</span>
                <input v-model="localFilters.createdBefore" type="date"
                  class="flex-1 px-2 py-2 text-sm bg-[var(--bg-tertiary)] border border-[var(--border)] rounded text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] transition-colors" />
              </div>
            </div>

            <!-- 排序 -->
            <div>
              <label class="block text-xs text-[var(--text-tertiary)] mb-1.5">排序方式</label>
              <div class="flex items-center gap-2">
                <select v-model="localFilters.sortBy"
                  class="flex-1 px-3 py-2 text-sm bg-[var(--bg-tertiary)] border border-[var(--border)] rounded text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] transition-colors">
                  <option value="createdAt">添加时间</option>
                  <option value="title">标题</option>
                  <option value="fileSize">文件大小</option>
                  <option value="pageCount">页数</option>
                  <option value="updatedAt">更新时间</option>
                </select>
                <button
                  class="flex-shrink-0 p-2 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded text-[var(--text-secondary)] hover:text-[var(--text-primary)] transition-colors"
                  @click="toggleSortOrder">
                  <svg v-if="localFilters.sortOrder === 'asc'" class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 4h13M3 8h9m-9 4h6m4 0l4-4m0 0l4 4m-4-4v12" />
                  </svg>
                  <svg v-else class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 4h13M3 8h9m-9 4h9m5-4v12m0 0l-4-4m4 4l4-4" />
                  </svg>
                </button>
              </div>
            </div>

          </div>
        </Transition>

        <!-- 可滚动内容区（搜索历史） -->
        <div class="flex-1 overflow-y-auto">
          <div v-if="searchHistory.length > 0 && !query" class="px-4 py-3">
            <div class="flex items-center justify-between mb-2">
              <h3 class="text-xs font-medium text-[var(--text-tertiary)] uppercase tracking-wider">最近搜索</h3>
              <button @click="clearHistory" class="text-xs text-[var(--accent)] hover:underline">清除</button>
            </div>
            <div class="space-y-1">
              <button v-for="(item, index) in recentHistory" :key="index" @click="selectHistory(item)"
                class="flex items-center justify-between w-full px-3 py-2.5 text-left text-[var(--text-primary)] bg-[var(--bg-tertiary)] hover:bg-[var(--border)] rounded transition-colors group text-sm">
                <div class="flex items-center flex-1 min-w-0">
                  <svg class="w-4 h-4 mr-3 text-[var(--text-tertiary)] shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                  <span class="truncate">{{ item }}</span>
                </div>
                <button @click.stop="removeHistoryItem(index)" class="ml-2 p-1 opacity-0 group-hover:opacity-100 transition-opacity">
                  <svg class="w-4 h-4 text-[var(--text-tertiary)] hover:text-red-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </button>
            </div>
          </div>

          <div v-if="!query && searchHistory.length === 0 && !showFilters"
            class="flex flex-col items-center justify-center py-16 text-center px-4">
            <svg class="w-12 h-12 text-[var(--text-tertiary)] mb-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
            </svg>
            <p class="text-[var(--text-secondary)]">搜索漫画，或使用高级筛选</p>
          </div>
        </div>

        <!-- 底部操作栏 -->
        <div class="flex-shrink-0 border-t border-[var(--border)] bg-[var(--bg-primary)] px-4 py-3 flex items-center gap-3">
          <button
            class="px-4 py-2 text-sm text-[var(--text-secondary)] hover:text-[var(--text-primary)] transition-colors"
            @click="handleReset"
          >
            重置全部
          </button>
          <button
            class="flex-1 py-2 text-sm bg-[var(--accent)] text-white rounded hover:bg-[var(--accent)]/80 transition-colors font-medium"
            @click="handleApply"
          >
            搜索
          </button>
        </div>

      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from 'vue'
import { useQuery } from '@tanstack/vue-query'
import { getTags } from '@/utils/api'
import type { SearchParams, Tag } from '@/types/api'

interface Props {
  show: boolean
  initialQuery?: string
  currentFilters?: Partial<SearchParams>
}

const props = withDefaults(defineProps<Props>(), {
  show: false,
  initialQuery: '',
})

const emit = defineEmits<{
  'close': []
  'apply': [payload: { query: string; filters: Partial<SearchParams> }]
}>()

const searchInput = ref<HTMLInputElement | null>(null)
const query = ref('')
const searchHistory = ref<string[]>([])

// 筛选面板
const showFilters = ref(false)
const tagInput = ref('')
const showTagSuggest = ref(false)
const selectedTags = ref<string[]>([])

interface LocalFilters {
  minPages?: number
  maxPages?: number
  createdAfter?: string
  createdBefore?: string
  sortBy: string
  sortOrder: string
}

const localFilters = ref<LocalFilters>({ sortBy: 'createdAt', sortOrder: 'asc' })

// 标签数据
const { data: allTags } = useQuery({
  queryKey: ['tags'],
  queryFn: getTags,
  staleTime: 5 * 60 * 1000,
})

const tagSuggestions = computed<Tag[]>(() => {
  const input = tagInput.value.trim().toLowerCase()
  if (!input || !allTags.value) return []
  return allTags.value
    .filter(tag => {
      const full = `${tag.namespace}:${tag.name}`.toLowerCase()
      const already = selectedTags.value.includes(`${tag.namespace}:${tag.name}`)
      return !already && (full.includes(input) || tag.name.toLowerCase().includes(input))
    })
    .slice(0, 6)
})

const activeFilterCount = computed(() => {
  let count = 0
  if (selectedTags.value.length > 0) count++
  if (localFilters.value.minPages != null || localFilters.value.maxPages != null) count++
  if (localFilters.value.createdAfter || localFilters.value.createdBefore) count++
  if (localFilters.value.sortBy !== 'createdAt' || localFilters.value.sortOrder !== 'asc') count++
  return count
})

const recentHistory = computed(() => searchHistory.value.slice(0, 5))

// 标签操作
const addTag = (tag: Tag) => {
  const tagStr = `${tag.namespace}:${tag.name}`
  if (!selectedTags.value.includes(tagStr)) {
    selectedTags.value.push(tagStr)
  }
  tagInput.value = ''
  showTagSuggest.value = false
}

const removeTag = (tag: string) => {
  selectedTags.value = selectedTags.value.filter(t => t !== tag)
}

const handleTagBlur = () => {
  setTimeout(() => { showTagSuggest.value = false }, 150)
}

const toggleSortOrder = () => {
  localFilters.value.sortOrder = localFilters.value.sortOrder === 'asc' ? 'desc' : 'asc'
}

// 历史记录
const loadSearchHistory = () => {
  try {
    const stored = localStorage.getItem('search-history')
    if (stored) searchHistory.value = JSON.parse(stored)
  } catch { searchHistory.value = [] }
}

const saveSearchHistory = () => {
  try {
    localStorage.setItem('search-history', JSON.stringify(searchHistory.value))
  } catch { /* ignore */ }
}

const addToHistory = (q: string) => {
  if (!q.trim()) return
  const filtered = searchHistory.value.filter(item => item !== q)
  searchHistory.value = [q, ...filtered].slice(0, 10)
  saveSearchHistory()
}

const clearHistory = () => {
  searchHistory.value = []
  saveSearchHistory()
}

const removeHistoryItem = (index: number) => {
  searchHistory.value.splice(index, 1)
  saveSearchHistory()
}

const selectHistory = (item: string) => {
  query.value = item
  handleApply()
}

// 应用搜索
const handleApply = () => {
  if (query.value.trim()) addToHistory(query.value.trim())

  const filters: Partial<SearchParams> = {
    sortBy: localFilters.value.sortBy,
    sortOrder: localFilters.value.sortOrder,
  }
  if (selectedTags.value.length > 0) filters.tags = selectedTags.value
  if (localFilters.value.minPages != null) filters.minPages = localFilters.value.minPages
  if (localFilters.value.maxPages != null) filters.maxPages = localFilters.value.maxPages
  if (localFilters.value.createdAfter) filters.createdAfter = localFilters.value.createdAfter
  if (localFilters.value.createdBefore) filters.createdBefore = localFilters.value.createdBefore

  emit('apply', { query: query.value.trim(), filters })
  emit('close')
}

// 重置全部（含关闭）
const handleReset = () => {
  query.value = ''
  selectedTags.value = []
  tagInput.value = ''
  localFilters.value = { sortBy: 'createdAt', sortOrder: 'asc' }
  emit('apply', { query: '', filters: {} })
  emit('close')
}

const handleClose = () => {
  emit('close')
}

// 同步外部传入的当前筛选
const syncFilters = () => {
  const f = props.currentFilters
  if (!f) return
  if (f.tags) selectedTags.value = [...f.tags]
  if (f.minPages != null) localFilters.value.minPages = f.minPages
  if (f.maxPages != null) localFilters.value.maxPages = f.maxPages
  if (f.createdAfter) localFilters.value.createdAfter = f.createdAfter
  if (f.createdBefore) localFilters.value.createdBefore = f.createdBefore
  if (f.sortBy) localFilters.value.sortBy = f.sortBy
  if (f.sortOrder) localFilters.value.sortOrder = f.sortOrder
}

const handleKeydown = (event: KeyboardEvent) => {
  if (event.key === 'Escape' && props.show) handleClose()
}

watch(() => props.show, async (newVal) => {
  if (newVal) {
    query.value = props.initialQuery || ''
    syncFilters()
    await nextTick()
    searchInput.value?.focus()
  } else {
    showFilters.value = false
  }
})

onMounted(() => {
  loadSearchHistory()
  document.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown)
})
</script>

<style scoped>
.modal-enter-active,
.modal-leave-active {
  transition: all 0.25s ease;
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
</style>
