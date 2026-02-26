<template>
  <div class="library-view min-h-screen bg-[var(--bg-secondary)]">
    <!-- 顶部栏 -->
    <LibraryTopBar
      :search-query="searchQuery"
      :user-name="userName"
      @toggle-mobile-search="libraryStore.toggleMobileSearch()"
      @search="handleTopBarSearch"
    />

    <!-- 桌面端分类下拉（自带按钮和下拉面板） -->
    <div class="hidden md:flex fixed top-4 right-48 z-50">
      <CategoryDropdown
        :selected-category-id="libraryStore.selectedCategoryId"
        :total-archives="allArchivesCount"
        @select-category="handleSelectCategory"
      />
    </div>

    <!-- 主内容区 -->
    <main class="pt-14 md:pt-16 pb-16 md:pb-0">
      <!-- 随机精选 -->
      <RandomCarousel v-if="libraryStore.showCarousel" />

      <!-- 信息栏：当前分类 + 漫画数量 -->
      <div class="flex items-center justify-between px-4 py-3">
        <h2 class="text-base font-semibold text-[var(--text-primary)]">
          {{ currentCategoryName }}
        </h2>
        <span class="text-sm text-[var(--text-secondary)]">
          {{ totalArchives }} 部
        </span>
      </div>

      <!-- 错误信息 -->
      <div v-if="error" class="mx-4 mb-4 p-3 rounded-lg bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800">
        <div class="flex items-center text-red-600 dark:text-red-400 text-sm">
          <svg class="w-4 h-4 mr-2 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          加载失败: {{ error.message }}
        </div>
      </div>

      <!-- 加载中 -->
      <div v-if="isLoading" class="flex items-center justify-center py-20">
        <div class="w-6 h-6 border-2 border-gray-300 dark:border-gray-600 border-t-blue-500 rounded-full animate-spin" />
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
      <div v-else class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6 gap-4 px-4 pb-4">
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
      <div v-if="totalPages > 1" class="flex items-center justify-center space-x-2 px-4 py-6">
        <button :disabled="currentPage === 1" class="px-3 py-1.5 text-sm rounded-lg border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] disabled:opacity-40 disabled:cursor-not-allowed transition-colors" @click="goToPage(currentPage - 1)">
          上一页
        </button>
        <button v-if="showFirstPage" :class="pageButtonClass(1)" @click="goToPage(1)">1</button>
        <span v-if="showLeftEllipsis" class="px-2 text-[var(--text-tertiary)]">...</span>
        <button v-for="page in visiblePages" :key="page" :class="pageButtonClass(page)" @click="goToPage(page)">{{ page }}</button>
        <span v-if="showRightEllipsis" class="px-2 text-[var(--text-tertiary)]">...</span>
        <button v-if="showLastPage" :class="pageButtonClass(totalPages)" @click="goToPage(totalPages)">{{ totalPages }}</button>
        <button :disabled="currentPage === totalPages" class="px-3 py-1.5 text-sm rounded-lg border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] disabled:opacity-40 disabled:cursor-not-allowed transition-colors" @click="goToPage(currentPage + 1)">
          下一页
        </button>
      </div>
    </main>

    <!-- 底部分类栏（仅移动端） -->
    <CategoryBottomBar
      :selected-category-id="libraryStore.selectedCategoryId"
      @select-category="handleSelectCategory"
      @create-category="showCreateCategoryModal = true"
    />

    <!-- 移动端搜索模态框 -->
    <MobileSearchModal
      :show="libraryStore.showMobileSearch"
      :initial-query="searchQuery"
      @close="libraryStore.showMobileSearch = false"
      @search="handleMobileSearch"
    />

    <!-- 分类模态框 -->
    <CategoryModal
      v-if="showCreateCategoryModal"
      @close="showCreateCategoryModal = false"
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
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useQuery, useQueryClient } from '@tanstack/vue-query'
import { useAuthStore } from '@/stores/auth'
import { useLibraryStore } from '@/stores/library'
import LibraryTopBar from '@/components/library/LibraryTopBar.vue'
import CategoryBottomBar from '@/components/library/CategoryBottomBar.vue'
import CategoryDropdown from '@/components/library/CategoryDropdown.vue'
import RandomCarousel from '@/components/library/RandomCarousel.vue'
import MobileSearchModal from '@/components/library/MobileSearchModal.vue'
import ArchiveThumbnailCard from '@/components/ArchiveThumbnailCard.vue'
import CategoryModal from '@/components/CategoryModal.vue'
import ArchiveContextMenu from '@/components/ArchiveContextMenu.vue'
import TagModal from '@/components/common/TagModal.vue'
import {
  searchArchives,
  getCategoryArchives,
  getBatchProgress,
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
  ReadingProgress,
} from '@/types/api'

const router = useRouter()
const route = useRoute()
const queryClient = useQueryClient()
const authStore = useAuthStore()
const libraryStore = useLibraryStore()

// 基础状态
const searchQuery = ref('')
const currentPage = ref(1)
const pageSize = ref(20)
const showCreateCategoryModal = ref(false)
const showEditCategoryModal = ref(false)
const selectedCategory = ref<Category | DynamicCategory | null>(null)

// 右键菜单
const showContextMenu = ref(false)
const contextMenuArchive = ref<Archive | null>(null)
const contextMenuPosition = ref({ x: 0, y: 0 })

// 标签模态框
const showTagModal = ref(false)
const tagModalArchive = ref<Archive | null>(null)

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
  sortBy: 'createdAt',
  sortOrder: 'asc',
}))

// Query key
const queryKey = computed(() => [
  'archives',
  libraryStore.selectedCategoryId,
  currentPage.value,
  pageSize.value,
  searchQuery.value,
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
  'w-8 h-8 text-sm rounded-lg transition-colors',
  currentPage.value === page
    ? 'bg-blue-500 text-white'
    : 'border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)]',
]

// 事件处理
const handleTopBarSearch = (query: string) => {
  searchQuery.value = query
  currentPage.value = 1
}

const handleMobileSearch = (query: string) => {
  searchQuery.value = query
  currentPage.value = 1
  libraryStore.showMobileSearch = false
}

const handleSelectCategory = async (categoryId: string | null) => {
  libraryStore.selectCategory(categoryId)
  searchQuery.value = ''
  currentPage.value = 1
  await refetch()
}

const openReader = (archiveId: string) => {
  router.push(`/reader/${archiveId}`)
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
  showCreateCategoryModal.value = false
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
    alert('添加标签失败，请稍后重试')
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
    alert('添加到分类失败，请稍后重试')
  }
}

const handleRemoveArchiveFromCategory = async (archiveId: string, categoryId: string) => {
  try {
    await removeArchivesFromCategory(categoryId, { archiveIds: [archiveId] })
    queryClient.invalidateQueries({ queryKey: ['categories'] })
    refetch()
  } catch (err) {
    console.error('Failed to remove from category:', err)
    alert('移出分类失败，请稍后重试')
  }
}

// 删除档案
const handleDeleteArchiveFromContext = () => {
  if (!contextMenuArchive.value) return
  const archive = contextMenuArchive.value
  if (confirm(`确定要删除漫画《${archive.title}》吗？此操作不可撤销。`)) {
    handleDeleteArchive(archive.id)
  }
  closeContextMenu()
}

const handleDeleteArchive = async (archiveId: string) => {
  try {
    await deleteArchive(archiveId)
    queryClient.invalidateQueries({ queryKey: ['categories'] })
    queryClient.invalidateQueries({ queryKey: ['allArchivesCount'] })
    refetch()
  } catch (err) {
    console.error('Failed to delete archive:', err)
    alert('删除失败，请稍后重试')
  }
}

// 从阅读器返回时刷新进度
watch(route, (newRoute, oldRoute) => {
  if (newRoute.name === 'library' && oldRoute?.name === 'reader') {
    setTimeout(() => {
      queryClient.invalidateQueries({ queryKey: ['batchProgress'] })
    }, 100)
  }
})

onMounted(() => {
  document.addEventListener('click', closeContextMenu)
  if (route.name === 'library') {
    queryClient.invalidateQueries({ queryKey: ['batchProgress'] })
  }
})

onUnmounted(() => {
  document.removeEventListener('click', closeContextMenu)
})
</script>

<style scoped>
.library-view {
  /* Library 页面根容器 */
}
</style>
