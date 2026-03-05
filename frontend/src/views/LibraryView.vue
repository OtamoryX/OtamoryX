<template>
  <div class="library-view min-h-screen bg-[var(--bg-secondary)]">
    <!-- 顶部栏 -->
    <LibraryTopBar
      :search-query="searchQuery"
      :user-name="userName"
      :show-advanced-search="showAdvancedSearch"
      :active-filter-count="activeFilterCount"
      :selected-category-id="libraryStore.selectedCategoryId"
      :total-archives="allArchivesCount"
      @toggle-mobile-search="libraryStore.toggleMobileSearch()"
      @search="handleTopBarSearch"
      @toggle-advanced-search="showAdvancedSearch = !showAdvancedSearch"
      @select-category="handleSelectCategory"
      @edit-category="handleEditCategory"
      @create-category="openCreateCategoryModal"
    />

    <!-- 高级筛选面板（桌面端） -->
    <div class="hidden md:block">
      <AdvancedSearchPanel
        :show="showAdvancedSearch"
        :current-filters="advancedFilters"
        :can-save-dynamic-category="canSaveCurrentSearchAsDynamicCategory"
        @apply-filters="handleAdvancedFilters"
        @reset-filters="handleResetFilters"
        @save-dynamic-category="handleSaveCurrentSearchAsDynamicCategory"
      />
    </div>

    <!-- 主内容区 -->
    <main :class="['pt-14 md:pt-14 pb-16 md:pb-4 transition-all', showAdvancedSearch ? 'md:pt-44' : '']">
      <div class="mx-auto w-full max-w-[1440px]">
        <!-- 随机精选（始终渲染，内部控制折叠）-->
        <RandomCarousel
          :category-id="libraryStore.selectedCategoryId || ''"
          :search-query="searchQuery"
          :tags="advancedFilters.tags"
          :min-pages="advancedFilters.minPages"
          :max-pages="advancedFilters.maxPages"
          :created-after="advancedFilters.createdAfter"
          :created-before="advancedFilters.createdBefore"
          @open-archive="openReader"
          @archive-contextmenu="handleArchiveContextMenu"
        />

        <!-- 移动端：活跃筛选 chips 条 -->
        <div v-if="(searchQuery || activeFilterCount > 0)" class="md:hidden px-3 py-2 flex items-center gap-2 overflow-x-auto border-b border-[var(--border)] bg-[var(--bg-primary)]" style="scrollbar-width: none;">
          <!-- 搜索词 chip -->
          <span v-if="searchQuery"
            class="inline-flex items-center gap-1 flex-shrink-0 px-2.5 py-1 rounded-full text-xs bg-[var(--accent)]/20 text-[var(--accent)] border border-[var(--accent)]/30">
            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
            </svg>
            {{ searchQuery }}
            <button @click="handleClearSearch">×</button>
          </span>
          <!-- 标签 chips -->
          <span v-for="tag in (advancedFilters.tags || [])" :key="tag"
            class="inline-flex items-center gap-1 flex-shrink-0 px-2.5 py-1 rounded-full text-xs bg-[var(--bg-tertiary)] text-[var(--text-secondary)] border border-[var(--border)]">
            {{ tag }}
            <button @click="removeActiveTag(tag)">×</button>
          </span>
          <!-- 页数范围 chip -->
          <span v-if="advancedFilters.minPages != null || advancedFilters.maxPages != null"
            class="inline-flex items-center gap-1 flex-shrink-0 px-2.5 py-1 rounded-full text-xs bg-[var(--bg-tertiary)] text-[var(--text-secondary)] border border-[var(--border)]">
            页数 {{ advancedFilters.minPages ?? '?' }}~{{ advancedFilters.maxPages ?? '?' }}
            <button @click="removePageFilter">×</button>
          </span>
          <!-- 日期范围 chip -->
          <span v-if="advancedFilters.createdAfter || advancedFilters.createdBefore"
            class="inline-flex items-center gap-1 flex-shrink-0 px-2.5 py-1 rounded-full text-xs bg-[var(--bg-tertiary)] text-[var(--text-secondary)] border border-[var(--border)]">
            时间筛选
            <button @click="removeDateFilter">×</button>
          </span>
          <!-- 重置全部 -->
          <button v-if="activeFilterCount > 0 || searchQuery"
            class="flex-shrink-0 ml-auto px-3 py-1 text-xs text-red-400 hover:text-red-300 transition-colors"
            @click="handleMobileClearAll">
            清除全部
          </button>
        </div>

        <!-- 信息栏：当前分类 + 漫画数量 -->
        <div class="flex items-center justify-between px-4 py-2">
          <h2 class="text-sm font-medium text-[var(--text-primary)]">
            {{ currentCategoryName }}
          </h2>
          <span class="text-xs text-[var(--text-secondary)]">
            {{ totalArchives }} 部
          </span>
        </div>

        <!-- 错误信息 -->
        <div v-if="error" class="mx-4 mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30">
          <div class="flex items-center text-red-400 text-sm">
            <svg class="w-4 h-4 mr-2 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            加载失败: {{ error.message }}
          </div>
        </div>

        <!-- 加载中 -->
        <div v-if="isLoading" class="flex items-center justify-center py-20">
          <div class="w-6 h-6 border-2 border-[var(--border)] border-t-[var(--accent)] rounded-full animate-spin" />
          <span class="ml-3 text-sm text-[var(--text-secondary)]">加载中...</span>
        </div>

        <!-- 空状态 -->
        <div v-else-if="archives.length === 0" class="flex flex-col items-center justify-center py-20 text-[var(--text-secondary)]">
          <svg class="w-16 h-16 mb-4 opacity-40" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
          </svg>
          <p>{{ libraryStore.selectedCategoryId ? '该分类下没有漫画' : '没有找到漫画' }}</p>
        </div>

        <!-- 漫画网格 -->
        <div v-else class="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-5 lg:grid-cols-6 xl:grid-cols-7 2xl:grid-cols-8 gap-2 px-3 pb-4">
          <ArchiveThumbnailCard
            v-for="archive in archives"
            :key="archive.id"
            :archive="archive"
            :progress-percentage="progressData.get(archive.id)?.progressPercentage"
            @click="openReader(archive.id)"
            @contextmenu="handleArchiveContextMenu"
          />
        </div>

        <!-- 分页 -->
        <div v-if="totalPages > 1" class="flex items-center justify-center space-x-1.5 px-4 py-4">
          <button :disabled="currentPage === 1" class="px-3 py-1 text-xs rounded border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] disabled:opacity-40 disabled:cursor-not-allowed transition-colors" @click="goToPage(currentPage - 1)">
            上一页
          </button>
          <button v-if="showFirstPage" :class="pageButtonClass(1)" @click="goToPage(1)">1</button>
          <span v-if="showLeftEllipsis" class="px-1 text-[var(--text-tertiary)] text-xs">...</span>
          <button v-for="page in visiblePages" :key="page" :class="pageButtonClass(page)" @click="goToPage(page)">{{ page }}</button>
          <span v-if="showRightEllipsis" class="px-1 text-[var(--text-tertiary)] text-xs">...</span>
          <button v-if="showLastPage" :class="pageButtonClass(totalPages)" @click="goToPage(totalPages)">{{ totalPages }}</button>
          <button :disabled="currentPage === totalPages" class="px-3 py-1 text-xs rounded border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] disabled:opacity-40 disabled:cursor-not-allowed transition-colors" @click="goToPage(currentPage + 1)">
            下一页
          </button>
        </div>
      </div>
    </main>

    <!-- 底部分类栏（仅移动端） -->
    <CategoryBottomBar
      :selected-category-id="libraryStore.selectedCategoryId"
      @select-category="handleSelectCategory"
      @create-category="openCreateCategoryModal"
    />

    <!-- 移动端搜索模态框 -->
    <MobileSearchModal
      :show="libraryStore.showMobileSearch"
      :initial-query="searchQuery"
      :current-filters="advancedFilters"
      @close="libraryStore.showMobileSearch = false"
      @apply="handleMobileApply"
    />

    <!-- 分类模态框 -->
    <CategoryModal
      v-if="showCreateCategoryModal"
      :initial-category-type="createCategoryInitialType"
      :initial-search-params="createCategoryInitialSearchParams"
      @close="closeCreateCategoryModal"
      @created="handleCategoryCreated"
    />
    <CategoryModal
      v-if="showEditCategoryModal && selectedCategory"
      :category="selectedCategory"
      @close="showEditCategoryModal = false"
      @updated="handleCategoryUpdated"
    />

    <!-- 右键菜单 -->
    <ArchiveContextMenu
      :show="showContextMenu"
      :archive="contextMenuArchive"
      :position="contextMenuPosition"
      @close="closeContextMenu"
      @open-reader-new-tab="handleOpenReaderInNewTabFromContext"
      @edit-metadata="handleEditMetadataFromContext"
      @add-tag="handleAddTagFromContext"
      @add-to-category="handleAddToCategoryFromContext"
      @remove-from-category="handleRemoveFromCategoryFromContext"
      @delete-archive="handleDeleteArchiveFromContext"
    />

    <!-- 标签添加模态框 -->
    <TagModal
      v-if="showTagModal"
      :archive="tagModalArchive!"
      @close="closeTagModal"
      @submit="handleTagModalSubmit"
    />

    <ConfirmModal
      :show="dialog.show"
      :title="dialog.title"
      :message="dialog.message"
      :type="dialog.type"
      :confirm-text="dialog.confirmText"
      :cancel-text="dialog.cancelText"
      :show-cancel="dialog.showCancel"
      @close="handleDialogClose"
      @confirm="handleDialogConfirm"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, onActivated, onDeactivated, nextTick } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useQuery, useQueryClient } from '@tanstack/vue-query'
import { useAuthStore } from '@/stores/auth'
import { useLibraryStore } from '@/stores/library'
import LibraryTopBar from '@/components/library/LibraryTopBar.vue'
import CategoryBottomBar from '@/components/library/CategoryBottomBar.vue'
import RandomCarousel from '@/components/library/RandomCarousel.vue'
import MobileSearchModal from '@/components/library/MobileSearchModal.vue'
import AdvancedSearchPanel from '@/components/library/AdvancedSearchPanel.vue'
import ArchiveThumbnailCard from '@/components/ArchiveThumbnailCard.vue'
import CategoryModal from '@/components/CategoryModal.vue'
import ArchiveContextMenu from '@/components/ArchiveContextMenu.vue'
import TagModal from '@/components/common/TagModal.vue'
import ConfirmModal from '@/components/common/ConfirmModal.vue'
import {
  searchArchives,
  getCategoryArchives,
  getBatchProgress,
  getArchive,
  getProgress,
  createTag,
  addTagToArchive,
  addArchivesToCategory,
  removeArchivesFromCategory,
  deleteArchive,
} from '@/utils/api'
import type {
  Archive,
  SearchParams,
  Category,
  DynamicCategory,
  PaginatedResponse,
  ReadingProgress,
} from '@/types/api'

const router = useRouter()
const route = useRoute()
const queryClient = useQueryClient()
const authStore = useAuthStore()
const libraryStore = useLibraryStore()

const LIBRARY_VIEW_SNAPSHOT_KEY = 'library-view-snapshot-v1'
const LIBRARY_RETURN_ARCHIVE_KEY = 'library-return-archive-id'

interface LibraryViewSnapshot {
  searchQuery: string
  currentPage: number
  advancedFilters: Partial<SearchParams>
  showAdvancedSearch: boolean
  selectedCategoryId: string | null
  scrollTop: number
}

const loadLibraryViewSnapshot = (): LibraryViewSnapshot | null => {
  try {
    const raw = sessionStorage.getItem(LIBRARY_VIEW_SNAPSHOT_KEY)
    if (!raw) return null
    const parsed = JSON.parse(raw)
    if (!parsed || typeof parsed !== 'object') return null

    return {
      searchQuery: typeof parsed.searchQuery === 'string' ? parsed.searchQuery : '',
      currentPage: Number.isInteger(parsed.currentPage) && parsed.currentPage > 0 ? parsed.currentPage : 1,
      advancedFilters: parsed.advancedFilters && typeof parsed.advancedFilters === 'object' ? parsed.advancedFilters : {},
      showAdvancedSearch: Boolean(parsed.showAdvancedSearch),
      selectedCategoryId:
        typeof parsed.selectedCategoryId === 'string' || parsed.selectedCategoryId === null
          ? parsed.selectedCategoryId
          : null,
      scrollTop: typeof parsed.scrollTop === 'number' && parsed.scrollTop >= 0 ? parsed.scrollTop : 0,
    }
  } catch (error) {
    console.error('Failed to parse library snapshot:', error)
    return null
  }
}

const initialSnapshot = loadLibraryViewSnapshot()
if (initialSnapshot && initialSnapshot.selectedCategoryId !== libraryStore.selectedCategoryId) {
  libraryStore.selectCategory(initialSnapshot.selectedCategoryId)
}

// 基础状态
const searchQuery = ref(initialSnapshot?.searchQuery ?? '')
const currentPage = ref(initialSnapshot?.currentPage ?? 1)

// 动态 pageSize：列数 × 每页行数
function getColumnsCount(): number {
  const w = window.innerWidth
  if (w >= 1536) return 8
  if (w >= 1280) return 7
  if (w >= 1024) return 6
  if (w >= 768) return 5
  if (w >= 640) return 4
  return 3
}
const columns = ref(getColumnsCount())
const pageSize = computed(() => columns.value * libraryStore.rowsPerPage)

let resizeTimer: ReturnType<typeof setTimeout>
function onResize() {
  clearTimeout(resizeTimer)
  resizeTimer = setTimeout(() => {
    const newCols = getColumnsCount()
    if (newCols !== columns.value) {
      columns.value = newCols
      currentPage.value = 1
    }
  }, 200)
}

const showCreateCategoryModal = ref(false)
const showEditCategoryModal = ref(false)
const selectedCategory = ref<Category | DynamicCategory | null>(null)
const createCategoryInitialType = ref<'static' | 'dynamic'>('static')
const createCategoryInitialSearchParams = ref<Partial<SearchParams>>({})

// 高级搜索面板
const showAdvancedSearch = ref(initialSnapshot?.showAdvancedSearch ?? false)
const advancedFilters = ref<Partial<SearchParams>>(initialSnapshot?.advancedFilters ?? {})
const restoredScrollTop = ref<number | null>(initialSnapshot?.scrollTop ?? null)

const saveViewSnapshot = (scrollTop = window.scrollY) => {
  try {
    restoredScrollTop.value = scrollTop
    const snapshot: LibraryViewSnapshot = {
      searchQuery: searchQuery.value,
      currentPage: currentPage.value,
      advancedFilters: advancedFilters.value,
      showAdvancedSearch: showAdvancedSearch.value,
      selectedCategoryId: libraryStore.selectedCategoryId,
      scrollTop,
    }
    sessionStorage.setItem(LIBRARY_VIEW_SNAPSHOT_KEY, JSON.stringify(snapshot))
  } catch (error) {
    console.error('Failed to save library snapshot:', error)
  }
}

const restoreViewScroll = () => {
  if (restoredScrollTop.value == null) return
  const top = restoredScrollTop.value
  nextTick(() => {
    requestAnimationFrame(() => {
      window.scrollTo({ top, behavior: 'auto' })
    })
  })
}

const activeFilterCount = computed(() => {
  let count = 0
  if (advancedFilters.value.tags && advancedFilters.value.tags.length > 0) count++
  if (advancedFilters.value.minPages != null || advancedFilters.value.maxPages != null) count++
  if (advancedFilters.value.createdAfter || advancedFilters.value.createdBefore) count++
  if (advancedFilters.value.sortBy && advancedFilters.value.sortBy !== 'createdAt') count++
  if (advancedFilters.value.sortOrder && advancedFilters.value.sortOrder !== 'asc') count++
  return count
})

const currentSearchSnapshot = computed<Partial<SearchParams>>(() => ({
  query: searchQuery.value.trim() || undefined,
  tags: advancedFilters.value.tags,
  minPages: advancedFilters.value.minPages,
  maxPages: advancedFilters.value.maxPages,
  minFileSize: advancedFilters.value.minFileSize,
  maxFileSize: advancedFilters.value.maxFileSize,
  createdAfter: advancedFilters.value.createdAfter,
  createdBefore: advancedFilters.value.createdBefore,
  lastReadAfter: advancedFilters.value.lastReadAfter,
  lastReadBefore: advancedFilters.value.lastReadBefore,
  sortBy: advancedFilters.value.sortBy || 'createdAt',
  sortOrder: advancedFilters.value.sortOrder || 'asc',
}))

const canSaveCurrentSearchAsDynamicCategory = computed(() => {
  if (searchQuery.value.trim()) return true
  if (advancedFilters.value.tags?.length) return true
  if (advancedFilters.value.minPages != null || advancedFilters.value.maxPages != null) return true
  if (advancedFilters.value.minFileSize != null || advancedFilters.value.maxFileSize != null) return true
  if (advancedFilters.value.createdAfter || advancedFilters.value.createdBefore) return true
  if (advancedFilters.value.lastReadAfter || advancedFilters.value.lastReadBefore) return true
  if (advancedFilters.value.sortBy && advancedFilters.value.sortBy !== 'createdAt') return true
  if (advancedFilters.value.sortOrder && advancedFilters.value.sortOrder !== 'asc') return true
  return false
})

// 右键菜单
const showContextMenu = ref(false)
const contextMenuArchive = ref<Archive | null>(null)
const contextMenuPosition = ref({ x: 0, y: 0 })

// 标签模态框
const showTagModal = ref(false)
const tagModalArchive = ref<Archive | null>(null)

type DialogType = 'default' | 'danger' | 'warning' | 'info'

interface DialogOptions {
  title?: string
  message: string
  type?: DialogType
  confirmText?: string
  cancelText?: string
  showCancel?: boolean
}

const dialog = ref({
  show: false,
  title: '提示',
  message: '',
  type: 'default' as DialogType,
  confirmText: '确认',
  cancelText: '取消',
  showCancel: true,
})

let dialogResolver: ((result: boolean) => void) | null = null

const openDialog = (options: DialogOptions): Promise<boolean> => {
  if (dialogResolver) {
    dialogResolver(false)
    dialogResolver = null
  }

  dialog.value = {
    show: true,
    title: options.title ?? '提示',
    message: options.message,
    type: options.type ?? 'default',
    confirmText: options.confirmText ?? '确认',
    cancelText: options.cancelText ?? '取消',
    showCancel: options.showCancel ?? true,
  }

  return new Promise((resolve) => {
    dialogResolver = resolve
  })
}

const resolveDialog = (result: boolean) => {
  dialog.value.show = false
  if (dialogResolver) {
    dialogResolver(result)
    dialogResolver = null
  }
}

const handleDialogClose = () => resolveDialog(false)
const handleDialogConfirm = () => resolveDialog(true)

const showInfoDialog = async (message: string, title = '提示') => {
  await openDialog({
    title,
    message,
    type: 'info',
    confirmText: '知道了',
    showCancel: false,
  })
}

// 进度数据
const progressData = ref<Map<string, ReadingProgress>>(new Map())

// 用户名
const userName = computed(() => authStore.user?.username || '')

// 当前分类名称
const currentCategoryName = computed(() => {
  if (!libraryStore.selectedCategoryId) return '全部漫画'
  return '分类漫画'
})

// 搜索参数
const searchParams = computed<SearchParams>(() => ({
  query: searchQuery.value || undefined,
  pageNumb: currentPage.value,
  pageSize: pageSize.value,
  sortBy: advancedFilters.value.sortBy || 'createdAt',
  sortOrder: advancedFilters.value.sortOrder || 'asc',
  tags: advancedFilters.value.tags,
  minPages: advancedFilters.value.minPages,
  maxPages: advancedFilters.value.maxPages,
  createdAfter: advancedFilters.value.createdAfter,
  createdBefore: advancedFilters.value.createdBefore,
}))

// Query key
const queryKey = computed(() => [
  'archives',
  libraryStore.selectedCategoryId,
  currentPage.value,
  pageSize.value,
  searchQuery.value,
  advancedFilters.value,
])

// 主查询
const { data, isLoading, refetch, error } = useQuery({
  queryKey,
  queryFn: async () => {
    if (libraryStore.selectedCategoryId) {
      return await getCategoryArchives(libraryStore.selectedCategoryId, searchParams.value)
    }
    return await searchArchives(searchParams.value)
  },
  retry: 1,
})

const archives = computed<Archive[]>(() => data.value?.data || [])
const totalArchives = computed(() => data.value?.total || 0)
const totalPages = computed(() => Math.ceil(totalArchives.value / pageSize.value))

// 全部漫画总数（用于分类下拉显示）
const { data: allArchivesData } = useQuery({
  queryKey: ['allArchivesCount'],
  queryFn: () => searchArchives({ pageNumb: 1, pageSize: 1 }),
  staleTime: 5 * 60 * 1000,
})
const allArchivesCount = computed(() => allArchivesData.value?.total || 0)

// 批量进度查询
const { data: batchProgressData } = useQuery({
  queryKey: ['batchProgress', archives],
  queryFn: async () => {
    const ids = archives.value.map(a => a.id)
    if (ids.length === 0) return []
    return await getBatchProgress(ids)
  },
  enabled: computed(() => archives.value.length > 0),
  retry: false,
  staleTime: 5 * 60 * 1000,
})

watch(batchProgressData, (newData) => {
  if (newData) {
    const map = new Map<string, ReadingProgress>()
    newData.forEach(p => map.set(p.archiveId, p))
    progressData.value = map
  }
}, { immediate: true })

watch(
  [searchQuery, currentPage, advancedFilters, showAdvancedSearch, () => libraryStore.selectedCategoryId],
  () => {
    saveViewSnapshot()
  },
  { deep: true },
)

// 分页逻辑
const visiblePages = computed(() => {
  const pages: number[] = []
  const maxVisible = 5
  let start = Math.max(1, currentPage.value - Math.floor(maxVisible / 2))
  let end = Math.min(totalPages.value, start + maxVisible - 1)
  if (end - start + 1 < maxVisible) start = Math.max(1, end - maxVisible + 1)
  for (let i = start; i <= end; i++) {
    if (i !== 1 && i !== totalPages.value) pages.push(i)
  }
  return pages
})
const showFirstPage = computed(() => !visiblePages.value.includes(1) && totalPages.value > 1)
const showLastPage = computed(() => !visiblePages.value.includes(totalPages.value) && totalPages.value > 1)
const showLeftEllipsis = computed(() => visiblePages.value.length > 0 && visiblePages.value[0]! > 2)
const showRightEllipsis = computed(() => visiblePages.value.length > 0 && visiblePages.value[visiblePages.value.length - 1]! < totalPages.value - 1)

const pageButtonClass = (page: number) => [
  'w-7 h-7 text-xs rounded transition-colors',
  currentPage.value === page
    ? 'bg-[var(--accent)] text-white'
    : 'border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)]',
]

// 事件处理
const handleTopBarSearch = (query: string) => {
  searchQuery.value = query
  currentPage.value = 1
}

const handleAdvancedFilters = (filters: Partial<SearchParams>) => {
  advancedFilters.value = filters
  currentPage.value = 1
}

const handleResetFilters = () => {
  advancedFilters.value = {}
  currentPage.value = 1
}

const handleMobileApply = (payload: { query: string; filters: Partial<SearchParams> }) => {
  searchQuery.value = payload.query
  advancedFilters.value = payload.filters
  currentPage.value = 1
  libraryStore.showMobileSearch = false
}

const handleClearSearch = () => {
  searchQuery.value = ''
  currentPage.value = 1
}

const removeActiveTag = (tag: string) => {
  const tags = (advancedFilters.value.tags || []).filter(t => t !== tag)
  advancedFilters.value = { ...advancedFilters.value, tags: tags.length > 0 ? tags : undefined }
  currentPage.value = 1
}

const removePageFilter = () => {
  advancedFilters.value = { ...advancedFilters.value, minPages: undefined, maxPages: undefined }
  currentPage.value = 1
}

const removeDateFilter = () => {
  advancedFilters.value = { ...advancedFilters.value, createdAfter: undefined, createdBefore: undefined }
  currentPage.value = 1
}

const handleMobileClearAll = () => {
  searchQuery.value = ''
  advancedFilters.value = {}
  currentPage.value = 1
}

const openCreateCategoryModal = () => {
  createCategoryInitialType.value = 'static'
  createCategoryInitialSearchParams.value = {}
  showCreateCategoryModal.value = true
}

const handleSaveCurrentSearchAsDynamicCategory = () => {
  createCategoryInitialType.value = 'dynamic'
  createCategoryInitialSearchParams.value = { ...currentSearchSnapshot.value }
  showCreateCategoryModal.value = true
}

const closeCreateCategoryModal = () => {
  showCreateCategoryModal.value = false
  createCategoryInitialType.value = 'static'
  createCategoryInitialSearchParams.value = {}
}

const handleSelectCategory = async (categoryId: string | null) => {
  libraryStore.selectCategory(categoryId)
  searchQuery.value = ''
  currentPage.value = 1
  await refetch()
}

const handleEditCategory = (category: Category) => {
  selectedCategory.value = category
  showEditCategoryModal.value = true
}

const openReader = (archiveId: string) => {
  saveViewSnapshot()
  router.push(`/reader/${archiveId}`)
}

const openReaderInNewTab = (archiveId: string) => {
  const routeLocation = router.resolve({
    name: 'reader',
    params: { id: archiveId },
  })
  window.open(routeLocation.href, '_blank', 'noopener,noreferrer')
}

const goToPage = async (page: number) => {
  if (page >= 1 && page <= totalPages.value) {
    currentPage.value = page
    await refetch()
    window.scrollTo({ top: 0, behavior: 'smooth' })
  }
}

// 分类模态框
const handleCategoryCreated = () => {
  closeCreateCategoryModal()
  queryClient.invalidateQueries({ queryKey: ['categories'] })
  queryClient.invalidateQueries({ queryKey: ['allArchivesCount'] })
  refetch()
}

const handleCategoryUpdated = () => {
  showEditCategoryModal.value = false
  selectedCategory.value = null
  queryClient.invalidateQueries({ queryKey: ['categories'] })
  queryClient.invalidateQueries({ queryKey: ['allArchivesCount'] })
  refetch()
}

// 右键菜单
const handleArchiveContextMenu = (event: MouseEvent, archive: Archive) => {
  contextMenuArchive.value = archive
  contextMenuPosition.value = { x: event.clientX, y: event.clientY }
  showContextMenu.value = true
}

const closeContextMenu = () => {
  showContextMenu.value = false
  contextMenuArchive.value = null
}

const handleOpenReaderInNewTabFromContext = () => {
  if (!contextMenuArchive.value) return
  const archiveId = contextMenuArchive.value.id
  closeContextMenu()
  openReaderInNewTab(archiveId)
}

const handleEditMetadataFromContext = () => {
  if (!contextMenuArchive.value) return
  const archiveId = contextMenuArchive.value.id
  closeContextMenu()
  saveViewSnapshot()
  router.push({
    name: 'reader',
    params: { id: archiveId },
    query: { panel: 'info' },
  })
}

// 标签操作
const handleAddTagFromContext = () => {
  if (!contextMenuArchive.value) return
  tagModalArchive.value = contextMenuArchive.value
  showTagModal.value = true
  closeContextMenu()
}

const handleAddTagToArchive = async (archiveId: string, tagName: string, namespace: string) => {
  try {
    const tag = await createTag(tagName, namespace)
    await addTagToArchive(archiveId, tag.id)
    refetch()
  } catch (err) {
    console.error('Failed to add tag:', err)
    await showInfoDialog('添加标签失败，请稍后重试', '操作失败')
  }
}

const handleTagModalSubmit = async (tagName: string, namespace: string) => {
  if (!tagModalArchive.value) return
  await handleAddTagToArchive(tagModalArchive.value.id, tagName, namespace)
  showTagModal.value = false
  tagModalArchive.value = null
}

const closeTagModal = () => {
  showTagModal.value = false
  tagModalArchive.value = null
}

// 分类操作
const handleAddToCategoryFromContext = (categoryId: string) => {
  if (!contextMenuArchive.value) return
  handleAddArchiveToCategory(contextMenuArchive.value.id, categoryId)
  closeContextMenu()
}

const handleRemoveFromCategoryFromContext = (categoryId: string) => {
  if (!contextMenuArchive.value) return
  handleRemoveArchiveFromCategory(contextMenuArchive.value.id, categoryId)
  closeContextMenu()
}

const handleAddArchiveToCategory = async (archiveId: string, categoryId: string) => {
  try {
    await addArchivesToCategory(categoryId, { archiveIds: [archiveId] })
    queryClient.invalidateQueries({ queryKey: ['categories'] })
    refetch()
  } catch (err) {
    console.error('Failed to add to category:', err)
    await showInfoDialog('添加到分类失败，请稍后重试', '操作失败')
  }
}

const handleRemoveArchiveFromCategory = async (archiveId: string, categoryId: string) => {
  try {
    await removeArchivesFromCategory(categoryId, { archiveIds: [archiveId] })
    queryClient.invalidateQueries({ queryKey: ['categories'] })
    refetch()
  } catch (err) {
    console.error('Failed to remove from category:', err)
    await showInfoDialog('移出分类失败，请稍后重试', '操作失败')
  }
}

// 删除档案
const handleDeleteArchiveFromContext = async () => {
  if (!contextMenuArchive.value) return
  const archive = contextMenuArchive.value
  closeContextMenu()

  const confirmed = await openDialog({
    title: '确认删除漫画',
    message: `确定要删除漫画《${archive.title}》吗？此操作不可撤销。`,
    type: 'danger',
    confirmText: '删除',
  })

  if (!confirmed) return
  await handleDeleteArchive(archive.id)
}

const handleDeleteArchive = async (archiveId: string) => {
  try {
    await deleteArchive(archiveId)
    removeArchiveFromRandomCache(archiveId)
    queryClient.invalidateQueries({ queryKey: ['categories'] })
    queryClient.invalidateQueries({ queryKey: ['allArchivesCount'] })
    refetch()
  } catch (err) {
    console.error('Failed to delete archive:', err)
    await showInfoDialog('删除失败，请稍后重试', '操作失败')
  }
}

const removeArchiveFromRandomCache = (archiveId: string) => {
  queryClient.setQueriesData<Archive[]>(
    { queryKey: ['randomArchives'] },
    (cachedArchives) => {
      if (!cachedArchives) return cachedArchives
      return cachedArchives.filter(archive => archive.id !== archiveId)
    },
  )
}

const updateArchiveInCurrentPage = (updatedArchive: Archive) => {
  queryClient.setQueryData<PaginatedResponse<Archive> | undefined>(queryKey.value, (cachedData) => {
    if (!cachedData) return cachedData
    const targetIndex = cachedData.data.findIndex(archive => archive.id === updatedArchive.id)
    if (targetIndex < 0) return cachedData

    const nextArchives = [...cachedData.data]
    nextArchives[targetIndex] = updatedArchive

    return {
      ...cachedData,
      data: nextArchives,
    }
  })
}

const refreshSingleArchiveFromReader = async (archiveId: string) => {
  try {
    const [latestArchive, latestProgress] = await Promise.all([
      getArchive(archiveId),
      getProgress(archiveId).catch(() => null),
    ])

    updateArchiveInCurrentPage(latestArchive)

    const nextProgressMap = new Map(progressData.value)
    if (latestProgress) {
      nextProgressMap.set(archiveId, latestProgress)
    } else {
      nextProgressMap.delete(archiveId)
    }
    progressData.value = nextProgressMap

    if (contextMenuArchive.value?.id === archiveId) {
      contextMenuArchive.value = latestArchive
    }
  } catch (error) {
    console.error('Failed to refresh returning archive:', error)
  }
}

const consumeReturningArchiveId = (): string | null => {
  try {
    const archiveId = sessionStorage.getItem(LIBRARY_RETURN_ARCHIVE_KEY)
    if (!archiveId) return null
    sessionStorage.removeItem(LIBRARY_RETURN_ARCHIVE_KEY)
    return archiveId
  } catch (error) {
    console.error('Failed to consume returning archive id:', error)
    return null
  }
}

let listenersBound = false
const bindGlobalListeners = () => {
  if (listenersBound) return
  document.addEventListener('click', closeContextMenu)
  window.addEventListener('resize', onResize)
  listenersBound = true
}

const unbindGlobalListeners = () => {
  if (!listenersBound) return
  document.removeEventListener('click', closeContextMenu)
  window.removeEventListener('resize', onResize)
  listenersBound = false
}

onMounted(() => {
  bindGlobalListeners()
  if (route.name === 'library') {
    queryClient.invalidateQueries({ queryKey: ['batchProgress'] })
  }
  const returningArchiveId = consumeReturningArchiveId()
  if (returningArchiveId) {
    void refreshSingleArchiveFromReader(returningArchiveId)
  }
  restoreViewScroll()
})

onActivated(() => {
  bindGlobalListeners()
  const returningArchiveId = consumeReturningArchiveId()
  if (returningArchiveId) {
    void refreshSingleArchiveFromReader(returningArchiveId)
  }
  restoreViewScroll()
})

onDeactivated(() => {
  saveViewSnapshot(window.scrollY)
  unbindGlobalListeners()
})

onUnmounted(() => {
  saveViewSnapshot(window.scrollY)
  unbindGlobalListeners()
  clearTimeout(resizeTimer)
})
</script>

<style scoped>
.library-view {
  /* Library 页面根容器 */
}
</style>
