<template>
  <div class="library-view flex h-full w-full">
    <!-- 分类侧边栏 -->
    <CategorySidebar
      :selected-category-id="selectedCategoryId"
      :total-archives="totalArchives"
      :collapsed="sidebarCollapsed"
      @select-category="handleSelectCategory"
      @create-category="showCreateCategoryModal = true"
      @edit-category="handleEditCategory"
      @toggle-collapse="handleToggleCollapse"
    />

    <!-- 主内容区域 -->
    <div class="flex-1 p-6">
      <div class="mb-6">
        <div class="flex items-center justify-between mb-4">
          <h1 class="text-2xl font-bold text-gray-900">
            {{ currentCategoryName }}
          </h1>
          <div class="text-sm text-gray-500">
            共 {{ archives.length }} 部漫画
          </div>
        </div>
        
        <!-- 搜索栏 -->
        <div class="max-w-2xl space-y-4">
          <div class="flex gap-3">
            <div class="flex-1 relative">
              <input
                v-model="searchQuery"
                type="text"
                placeholder="搜索漫画..."
                class="w-full px-4 py-2 pr-12 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                @keyup.enter="handleSearch"
              />
              <button
                @click="handleSearch"
                :disabled="isSearching"
                class="absolute right-2 top-1/2 transform -translate-y-1/2 p-2 text-gray-400 hover:text-blue-600 focus:outline-none disabled:cursor-not-allowed"
              >
                <svg v-if="!isSearching" class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                </svg>
                <svg v-else class="w-5 h-5 animate-spin" fill="none" viewBox="0 0 24 24">
                  <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                  <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
              </button>
            </div>
            <button
              @click="showAdvancedSearch = !showAdvancedSearch"
              class="px-4 py-2 border border-gray-300 rounded-lg text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 100 4m0-4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 100 4m0-4v2m0-6V4" />
              </svg>
            </button>
          </div>

          <!-- 高级搜索面板 -->
          <div v-if="showAdvancedSearch" class="bg-gray-50 rounded-lg p-4 space-y-4">
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
              <!-- 添加时间筛选 -->
              <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">添加时间</label>
                <div class="space-y-2">
                  <div>
                    <label class="block text-xs text-gray-600 mb-1">从</label>
                    <input
                      v-model="advancedSearch.createdAfter"
                      type="date"
                      class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                    />
                  </div>
                  <div>
                    <label class="block text-xs text-gray-600 mb-1">到</label>
                    <input
                      v-model="advancedSearch.createdBefore"
                      type="date"
                      class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                    />
                  </div>
                </div>
              </div>

              <!-- 最后阅读时间筛选 -->
              <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">最后阅读时间</label>
                <div class="space-y-2">
                  <div>
                    <label class="block text-xs text-gray-600 mb-1">从</label>
                    <input
                      v-model="advancedSearch.lastReadAfter"
                      type="date"
                      class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                    />
                  </div>
                  <div>
                    <label class="block text-xs text-gray-600 mb-1">到</label>
                    <input
                      v-model="advancedSearch.lastReadBefore"
                      type="date"
                      class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                    />
                  </div>
                </div>
              </div>

              <!-- 页数筛选 -->
              <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">页数范围</label>
                <div class="grid grid-cols-2 gap-2">
                  <div>
                    <label class="block text-xs text-gray-600 mb-1">最少</label>
                    <input
                      v-model.number="advancedSearch.minPages"
                      type="number"
                      min="1"
                      class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                      placeholder="0"
                    />
                  </div>
                  <div>
                    <label class="block text-xs text-gray-600 mb-1">最多</label>
                    <input
                      v-model.number="advancedSearch.maxPages"
                      type="number"
                      min="1"
                      class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                      placeholder="999"
                    />
                  </div>
                </div>
              </div>

              <!-- 排序 -->
              <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">排序方式</label>
                <div class="space-y-2">
                  <select
                    v-model="advancedSearch.sortBy"
                    class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                  >
                    <option value="">默认</option>
                    <option value="title">标题</option>
                    <option value="createdAt">添加时间</option>
                    <option value="lastReadAt">最后阅读时间</option>
                    <option value="pageCount">页数</option>
                    <option value="fileSize">文件大小</option>
                  </select>
                  <select
                    v-model="advancedSearch.sortOrder"
                    class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                  >
                    <option value="desc">降序</option>
                    <option value="asc">升序</option>
                  </select>
                </div>
              </div>
            </div>

            <!-- 操作按钮 -->
            <div class="flex justify-end space-x-3 pt-4 border-t border-gray-200">
              <button
                @click="clearAdvancedSearch"
                class="px-4 py-2 text-gray-600 hover:text-gray-800"
              >
                清空
              </button>
              <button
                @click="handleAdvancedSearch"
                class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                搜索
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- 错误信息 -->
      <div v-if="error" class="bg-red-50 border border-red-200 rounded-lg p-4 mb-4">
        <div class="text-red-800">加载失败: {{ error.message }}</div>
      </div>

      <!-- 漫画网格 -->
      <div v-if="isLoading" class="text-center py-8">
        <div class="text-gray-500">加载中...</div>
      </div>

      <div v-else-if="archives.length === 0" class="text-center py-8">
        <div class="text-gray-500">
          {{ selectedCategoryId ? '该分类下没有漫画' : '没有找到漫画' }}
        </div>
      </div>

      <div v-else class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6 gap-4">
        <ArchiveCard
          v-for="archive in archives"
          :key="archive.id"
          :archive="archive"
          :progress-percentage="progressData.get(archive.id)?.progressPercentage"
          @click="openReader(archive.id)"
        />
      </div>
    </div>

    <!-- 创建分类模态框 -->
    <CreateCategoryModal
      v-if="showCreateCategoryModal"
      @close="showCreateCategoryModal = false"
      @created="handleCategoryCreated"
    />

    <!-- 编辑分类模态框 -->
    <EditCategoryModal
      v-if="showEditCategoryModal && selectedCategory"
      :category="selectedCategory"
      @close="showEditCategoryModal = false"
      @updated="handleCategoryUpdated"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useQuery, useQueryClient } from '@tanstack/vue-query'
import ArchiveCard from '@/components/ArchiveCard.vue'
import CategorySidebar from '@/components/CategorySidebar.vue'
import CreateCategoryModal from '@/components/CreateCategoryModal.vue'
import EditCategoryModal from '@/components/EditCategoryModal.vue'
import { getArchives, searchArchives, getCategoryArchives, getBatchProgress } from '@/utils/api'
import type { Archive, SearchParams, Category, DynamicCategory, ReadingProgress } from '@/types/api'

const router = useRouter()
const route = useRoute()
const queryClient = useQueryClient()
const searchQuery = ref('')
const isSearching = ref(false)
const selectedCategoryId = ref<string | null>(null)
const showCreateCategoryModal = ref(false)
const showEditCategoryModal = ref(false)
const selectedCategory = ref<Category | DynamicCategory | null>(null)
const showAdvancedSearch = ref(false)

// 侧边栏折叠状态管理
const sidebarCollapsed = ref(false)
const COLLAPSE_BREAKPOINT = 1024 // 定义断点宽度 (1024px = lg)

// 进度数据管理
const progressData = ref<Map<string, ReadingProgress>>(new Map())

// 高级搜索参数
const advancedSearch = ref({
  createdAfter: '',
  createdBefore: '',
  lastReadAfter: '',
  lastReadBefore: '',
  minPages: undefined as number | undefined,
  maxPages: undefined as number | undefined,
  sortBy: '',
  sortOrder: 'desc'
})

// 当前分类名称
const currentCategoryName = computed(() => {
  if (!selectedCategoryId.value) return '全部漫画'
  return selectedCategory.value?.name || '分类漫画'
})

// 搜索参数
const searchParams = computed<SearchParams>(() => {
  const params: SearchParams = {
    query: searchQuery.value || undefined,
    page: 1,
    limit: 20
  }

  // 如果有高级搜索参数，则添加到搜索参数中
  if (advancedSearch.value.createdAfter) {
    params.createdAfter = advancedSearch.value.createdAfter
  }
  if (advancedSearch.value.createdBefore) {
    params.createdBefore = advancedSearch.value.createdBefore
  }
  if (advancedSearch.value.lastReadAfter) {
    params.lastReadAfter = advancedSearch.value.lastReadAfter
  }
  if (advancedSearch.value.lastReadBefore) {
    params.lastReadBefore = advancedSearch.value.lastReadBefore
  }
  if (advancedSearch.value.minPages) {
    params.minPages = advancedSearch.value.minPages
  }
  if (advancedSearch.value.maxPages) {
    params.maxPages = advancedSearch.value.maxPages
  }
  if (advancedSearch.value.sortBy) {
    params.sortBy = advancedSearch.value.sortBy
    params.sortOrder = advancedSearch.value.sortOrder
  }

  return params
})

// 根据选择的分类和搜索查询决定使用哪个 API
const { data, isLoading, refetch, error } = useQuery({
  queryKey: ['archives', selectedCategoryId, searchParams],
  queryFn: async () => {
    try {
      if (selectedCategoryId.value) {
        // 获取分类下的漫画
        console.log('Getting category archives:', selectedCategoryId.value)
        const result = await getCategoryArchives(selectedCategoryId.value, searchParams.value)
        console.log('Category archives result:', result)
        return result
      } else if (searchQuery.value.trim()) {
        // 搜索所有漫画
        console.log('Searching with params:', searchParams.value)
        const result = await searchArchives(searchParams.value)
        console.log('Search result:', result)
        return result
      } else {
        // 获取所有漫画
        console.log('Getting all archives')
        const result = await getArchives()
        console.log('Archives result:', result)
        return result
      }
    } catch (err) {
      console.error('API Error:', err)
      throw err
    }
  },
  retry: 1,
})

// 使用计算属性从 Vue Query 获取数据
const archives = computed<Archive[]>(() => {
  return data.value?.data || []
})

// 总漫画数量
const totalArchives = computed(() => {
  return data.value?.total || 0
})

// 获取当前页面所有漫画的进度数据
const { data: batchProgressData } = useQuery({
  queryKey: ['batchProgress', archives],
  queryFn: async () => {
    const archiveIds = archives.value.map(archive => archive.id)
    if (archiveIds.length === 0) return []
    
    console.log(`Loading progress for ${archiveIds.length} archives`)
    const result = await getBatchProgress(archiveIds)
    console.log(`Loaded progress for ${result.length} archives`)
    return result
  },
  enabled: computed(() => archives.value.length > 0),
  retry: false,
  staleTime: 5 * 60 * 1000, // 5分钟内认为数据是新鲜的
  cacheTime: 10 * 60 * 1000, // 10分钟后清除缓存
})

// 将进度数据转换为Map格式便于查找
watch(batchProgressData, (newProgressData) => {
  if (newProgressData) {
    const progressMap = new Map<string, ReadingProgress>()
    newProgressData.forEach(progress => {
      progressMap.set(progress.archiveId, progress)
    })
    progressData.value = progressMap
  }
}, { immediate: true })

// 调试用：监视数据变化
watch(data, (newData) => {
  console.log('Archives data updated:', newData)
}, { immediate: true })

const handleSearch = async () => {
  console.log('Searching for:', searchQuery.value)
  isSearching.value = true
  try {
    await refetch()
  } finally {
    isSearching.value = false
  }
}

const handleSelectCategory = (categoryId: string | null) => {
  console.log('Selected category:', categoryId)
  selectedCategoryId.value = categoryId
  // 清空搜索查询
  searchQuery.value = ''
}

const handleEditCategory = (category: Category | DynamicCategory) => {
  selectedCategory.value = category
  showEditCategoryModal.value = true
}

const handleCategoryCreated = () => {
  showCreateCategoryModal.value = false
  // 重新获取分类数据
  refetch()
}

const handleCategoryUpdated = () => {
  showEditCategoryModal.value = false
  selectedCategory.value = null
  // 重新获取分类数据
  refetch()
}

const openReader = (archiveId: string) => {
  router.push(`/reader/${archiveId}`)
}

const handleAdvancedSearch = async () => {
  console.log('Advanced search with params:', advancedSearch.value)
  isSearching.value = true
  try {
    await refetch()
  } finally {
    isSearching.value = false
  }
}

const clearAdvancedSearch = () => {
  advancedSearch.value = {
    createdAfter: '',
    createdBefore: '',
    lastReadAfter: '',
    lastReadBefore: '',
    minPages: undefined,
    maxPages: undefined,
    sortBy: '',
    sortOrder: 'desc'
  }
}

// 当从阅读器返回时刷新进度数据
const refreshProgressData = () => {
  console.log('Refreshing progress data')
  // 刷新进度数据查询
  queryClient.invalidateQueries({ queryKey: ['batchProgress'] })
}

// 监听路由变化，当从 reader 返回到 library 时刷新进度
watch(route, (newRoute, oldRoute) => {
  console.log('Route changed:', { from: oldRoute?.name, to: newRoute.name })
  if (newRoute.name === 'library' && oldRoute?.name === 'reader') {
    console.log('Returned from reader to library, refreshing progress')
    // 延迟一点刷新，确保进度已经保存
    setTimeout(refreshProgressData, 100)
  }
})

// 响应式侧边栏逻辑
const checkScreenWidth = () => {
  const width = window.innerWidth
  const shouldCollapse = width < COLLAPSE_BREAKPOINT
  if (sidebarCollapsed.value !== shouldCollapse) {
    console.log(`Screen width: ${width}px, setting sidebar collapsed: ${shouldCollapse}`)
    sidebarCollapsed.value = shouldCollapse
  }
}

// 防抖处理 resize 事件
let resizeTimeout: number | null = null
const handleWindowResize = () => {
  if (resizeTimeout) {
    clearTimeout(resizeTimeout)
  }
  resizeTimeout = setTimeout(() => {
    checkScreenWidth()
    resizeTimeout = null
  }, 150)
}

// 处理侧边栏手动切换
const handleToggleCollapse = (collapsed: boolean) => {
  console.log('Sidebar manually toggled:', collapsed)
  sidebarCollapsed.value = collapsed
}

onMounted(() => {
  console.log('LibraryView mounted')
  // 初始检查屏幕宽度
  checkScreenWidth()
  // 添加响应式事件监听器
  window.addEventListener('resize', handleWindowResize)
  // 页面加载时刷新一次进度数据（仅在库页面）
  if (route.name === 'library') {
    refreshProgressData()
  }
})

onUnmounted(() => {
  window.removeEventListener('resize', handleWindowResize)
})

// 移除 onActivated，因为它只在 KeepAlive 下工作
// onActivated(() => {
//   console.log('LibraryView activated, refreshing progress data')
//   // 刷新进度数据查询
//   queryClient.invalidateQueries({ queryKey: ['batchProgress'] })
// })
</script>