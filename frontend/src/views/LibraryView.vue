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
      :view-mode="libraryViewMode"
      @toggle-mobile-search="libraryStore.toggleMobileSearch()"
      @search="handleTopBarSearch"
      @toggle-advanced-search="showAdvancedSearch = !showAdvancedSearch"
      @select-category="handleSelectCategory"
      @edit-category="handleEditCategory"
      @create-category="openCreateCategoryModal"
      @set-view-mode="setLibraryViewMode"
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
          v-if="libraryViewMode === 'single'"
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

        <section v-if="libraryViewMode === 'collections'" class="px-3 pb-6">
          <div class="flex items-center justify-between gap-3 px-1 py-3">
            <div>
              <div class="flex items-center gap-2">
                <h2 class="text-sm font-medium text-[var(--text-primary)]">合集</h2>
                <button
                  v-if="collectionReviews.length"
                  class="inline-flex items-center gap-1 text-xs text-amber-400 hover:text-amber-300"
                  :title="`处理 ${collectionReviews.length} 条待确认合集成员`"
                  :aria-label="`处理 ${collectionReviews.length} 条待确认合集成员`"
                  @click="showCollectionReviews = true"
                >
                  <ExclamationTriangleIcon class="h-3.5 w-3.5" />
                  <span>{{ collectionReviews.length }}</span>
                </button>
              </div>
              <p class="mt-0.5 text-xs text-[var(--text-tertiary)]">{{ collections.length }} 个合集</p>
            </div>
            <div class="flex items-center gap-2">
              <button
                class="flex items-center gap-1.5 px-2.5 py-1.5 rounded text-xs border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] disabled:opacity-60"
                :disabled="collectionsRebuilding"
                @click="handleRebuildCollections"
              >
                <svg class="w-3.5 h-3.5" :class="collectionsRebuilding ? 'animate-spin' : ''" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2M15 20h-4" /></svg>
                {{ collectionsRebuilding ? '识别中...' : '重新识别合集' }}
              </button>
            </div>
          </div>
          <div v-if="collectionsLoading" class="flex items-center justify-center py-20 text-sm text-[var(--text-secondary)]">加载合集...</div>
          <div v-else-if="collections.length === 0" class="py-20 text-center text-sm text-[var(--text-secondary)]">
            <p>尚未发现可展示的合集。</p>
            <button class="mt-3 text-[var(--accent)] hover:underline" :disabled="collectionsRebuilding" @click="handleRebuildCollections">开始本地识别</button>
          </div>
          <div v-else class="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-5 lg:grid-cols-6 xl:grid-cols-7 2xl:grid-cols-8 gap-2">
            <CollectionCard v-for="collection in collections" :key="collection.id" :collection="collection" @open="openCollection" @contextmenu="handleCollectionContextMenu" />
          </div>
        </section>

        <section v-else-if="libraryViewMode === 'versions'" class="px-3 pb-6">
          <div class="flex items-center justify-between gap-3 px-1 py-3">
            <div>
              <h2 class="text-sm font-medium text-[var(--text-primary)]">多版本</h2>
              <p class="mt-0.5 text-xs text-[var(--text-tertiary)]">{{ versionGroups.length }} 组可比较内容</p>
            </div>
            <div class="flex rounded border border-[var(--border)] overflow-hidden text-xs">
              <button class="px-2.5 py-1.5" :class="versionStatus === 'active' ? 'bg-[var(--bg-tertiary)] text-[var(--text-primary)]' : 'text-[var(--text-secondary)]'" @click="versionStatus = 'active'">待处理</button>
              <button class="px-2.5 py-1.5" :class="versionStatus === 'keep_all' ? 'bg-[var(--bg-tertiary)] text-[var(--text-primary)]' : 'text-[var(--text-secondary)]'" @click="versionStatus = 'keep_all'">全部保留</button>
            </div>
          </div>
          <div v-if="selectedVersionGroupIds.size" class="mb-3 flex items-center justify-between gap-3 border-y border-[var(--border)] py-2">
            <span class="text-xs text-[var(--text-secondary)]">已选择 {{ selectedVersionGroupIds.size }} 组</span>
            <button class="px-2.5 py-1.5 rounded text-xs border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)]" :disabled="versionBatchBusy" @click="handleBatchVersionGroups">{{ versionBatchBusy ? '处理中...' : versionStatus === 'keep_all' ? '批量恢复待处理' : '批量全部保留' }}</button>
          </div>
          <div v-if="versionGroupsLoading" class="flex items-center justify-center py-20 text-sm text-[var(--text-secondary)]">加载多版本...</div>
          <div v-else-if="versionGroups.length === 0" class="py-20 text-center text-sm text-[var(--text-secondary)]">没有发现需要比较的多版本。</div>
          <div v-else class="mx-auto max-w-4xl space-y-2">
            <VersionGroupCard v-for="group in versionGroups" :key="group.id" :group="group" :selected="selectedVersionGroupIds.has(group.id)" @open="openVersionGroup" @toggle="toggleVersionGroupSelection" />
          </div>
        </section>

        <div v-else>
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
      @retry-title-translation="handleRetryTitleTranslationFromContext"
      @show-collections="handleShowCollectionsFromContext"
      @add-tag="handleAddTagFromContext"
      @add-to-category="handleAddToCategoryFromContext"
      @remove-from-category="handleRemoveFromCategoryFromContext"
      @delete-archive="handleDeleteArchiveFromContext"
    />

    <CollectionContextMenu
      :show="showCollectionContextMenu"
      :collection="contextMenuCollection"
      :position="collectionContextMenuPosition"
      :can-manage="authStore.isAdmin"
      @close="closeCollectionContextMenu"
      @open="handleOpenCollectionFromContext"
      @continue-reading="handleContinueCollectionFromContext"
      @edit="handleEditCollectionFromContext"
      @rebuild="handleRebuildCollectionFromContext"
      @delete-all="handleDeleteAllCollections"
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

    <CollectionDetailPanel
      :show="selectedCollectionId !== null"
      :detail="selectedCollectionDetail"
      :is-loading="collectionDetailLoading"
      :reviews="collectionReviews"
      @close="selectedCollectionId = null"
      @open-reader="openReader"
      @remove-member="handleRemoveCollectionMember"
      @reviews-changed="handleCollectionReviewChanged"
    />
    <CollectionEditModal :show="showCollectionEditModal" :collection="editingCollection" @close="showCollectionEditModal = false" @save="handleSaveCollection" />
    <VersionGroupPanel
      :show="selectedVersionGroup !== null"
      :group="selectedVersionGroup"
      :can-manage="authStore.isAdmin"
      @close="selectedVersionGroup = null"
      @open-reader="openReader"
      @open-comparison="openVersionComparison"
      @cleanup="handleVersionCleanup"
      @keep-all="handleKeepAllVersions"
      @restore="handleRestoreVersionGroup"
    />
    <CollectionReviewModal
      :show="showCollectionReviews"
      :reviews="collectionReviews"
      @close="showCollectionReviews = false"
      @changed="handleCollectionReviewChanged"
      @open-reader="openReader"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, onActivated, onDeactivated, nextTick } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useQuery, useQueryClient } from '@tanstack/vue-query'
import { ExclamationTriangleIcon } from '@heroicons/vue/24/outline'
import { useAuthStore } from '@/stores/auth'
import { useLibraryStore } from '@/stores/library'
import LibraryTopBar from '@/components/library/LibraryTopBar.vue'
import CategoryBottomBar from '@/components/library/CategoryBottomBar.vue'
import RandomCarousel from '@/components/library/RandomCarousel.vue'
import MobileSearchModal from '@/components/library/MobileSearchModal.vue'
import AdvancedSearchPanel from '@/components/library/AdvancedSearchPanel.vue'
import ArchiveThumbnailCard from '@/components/ArchiveThumbnailCard.vue'
import CollectionCard from '@/components/CollectionCard.vue'
import CollectionDetailPanel from '@/components/CollectionDetailPanel.vue'
import CollectionReviewModal from '@/components/CollectionReviewModal.vue'
import CollectionContextMenu from '@/components/CollectionContextMenu.vue'
import CollectionEditModal from '@/components/CollectionEditModal.vue'
import VersionGroupCard from '@/components/VersionGroupCard.vue'
import VersionGroupPanel from '@/components/VersionGroupPanel.vue'
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
  retryArchiveTitleTranslation,
  getCollections,
  getCollection,
  getCollectionReviews,
  rebuildCollections,
  removeCollectionMember,
  updateCollection,
  getVersionGroups,
  cleanupVersions,
  keepAllVersions,
  restoreVersionGroup,
  deleteAllCollections,
  deleteArchive,
} from '@/utils/api'
import type {
  Archive,
  SearchParams,
  Category,
  DynamicCategory,
  PaginatedResponse,
  ReadingProgress,
  CollectionSummary,
  CollectionDetail,
  CollectionReviewItem,
  VersionGroup,
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
const libraryViewMode = ref<'single' | 'collections' | 'versions'>('single')
const selectedCollectionId = ref<string | null>(null)
const showCollectionReviews = ref(false)
const collectionsRebuilding = ref(false)
const versionStatus = ref('active')
const selectedVersionGroup = ref<VersionGroup | null>(null)
const selectedVersionGroupIds = ref(new Set<string>())
const versionBatchBusy = ref(false)

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
const showCollectionContextMenu = ref(false)
const contextMenuCollection = ref<CollectionSummary | null>(null)
const collectionContextMenuPosition = ref({ x: 0, y: 0 })
const showCollectionEditModal = ref(false)
const editingCollection = ref<CollectionSummary | null>(null)

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

const collectionQueryKey = computed(() => ['collections', searchQuery.value])
const { data: collectionsData, isLoading: collectionsLoading, refetch: refetchCollections } = useQuery({
  queryKey: collectionQueryKey,
  queryFn: () => getCollections(searchQuery.value),
  enabled: computed(() => libraryViewMode.value === 'collections'),
  retry: 1,
})
const collections = computed<CollectionSummary[]>(() => collectionsData.value || [])

const versionGroupQueryKey = computed(() => ['versionGroups', searchQuery.value, versionStatus.value])
const { data: versionGroupsData, isLoading: versionGroupsLoading, refetch: refetchVersionGroups } = useQuery({
  queryKey: versionGroupQueryKey,
  queryFn: () => getVersionGroups(searchQuery.value, versionStatus.value),
  enabled: computed(() => libraryViewMode.value === 'versions'),
  retry: 1,
})
const versionGroups = computed<VersionGroup[]>(() => versionGroupsData.value || [])

watch(versionStatus, () => {
  selectedVersionGroupIds.value = new Set()
})

const { data: collectionReviewsData, refetch: refetchCollectionReviews } = useQuery({
  queryKey: ['collectionReviews'],
  queryFn: getCollectionReviews,
  staleTime: 30_000,
  retry: 1,
})
const collectionReviews = computed<CollectionReviewItem[]>(() => collectionReviewsData.value || [])

const { data: selectedCollectionData, isLoading: collectionDetailLoading, refetch: refetchSelectedCollection } = useQuery({
  queryKey: computed(() => ['collection', selectedCollectionId.value]),
  queryFn: () => getCollection(selectedCollectionId.value!),
  enabled: computed(() => selectedCollectionId.value !== null),
  retry: 1,
})
const selectedCollectionDetail = computed<CollectionDetail | null>(() => selectedCollectionData.value || null)

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

const setLibraryViewMode = (mode: 'single' | 'collections' | 'versions') => {
  libraryViewMode.value = mode
  if (mode === 'collections') void refetchCollections()
  if (mode === 'versions') void refetchVersionGroups()
}

const openCollection = (collection: CollectionSummary) => {
  selectedCollectionId.value = collection.id
}

const handleCollectionContextMenu = (event: MouseEvent, collection: CollectionSummary) => {
  contextMenuCollection.value = collection
  collectionContextMenuPosition.value = { x: event.clientX, y: event.clientY }
  showCollectionContextMenu.value = true
}

const closeCollectionContextMenu = () => {
  showCollectionContextMenu.value = false
  contextMenuCollection.value = null
}

const handleOpenCollectionFromContext = () => {
  const collection = contextMenuCollection.value
  closeCollectionContextMenu()
  if (collection) openCollection(collection)
}

const handleContinueCollectionFromContext = async () => {
  const collection = contextMenuCollection.value
  closeCollectionContextMenu()
  if (!collection) return
  const detail = await getCollection(collection.id).catch(() => null)
  const archiveId = detail?.members.find(member => member.confidence >= 0.75)?.archive.id || detail?.members[0]?.archive.id
  if (archiveId) openReader(archiveId)
}

const handleEditCollectionFromContext = () => {
  editingCollection.value = contextMenuCollection.value
  showCollectionEditModal.value = !!editingCollection.value
  closeCollectionContextMenu()
}

const handleSaveCollection = async (title: string, subtitle: string) => {
  const collection = editingCollection.value
  if (!collection) return
  try {
    await updateCollection(collection.id, { displayTitle: title, subtitle })
    showCollectionEditModal.value = false
    await Promise.all([refetchCollections(), refetchSelectedCollection()])
  } catch (error) {
    console.error('更新合集信息失败:', error)
    await showInfoDialog('无法保存合集信息，请稍后重试。', '操作失败')
  }
}

const handleRebuildCollectionFromContext = async () => {
  closeCollectionContextMenu()
  await handleRebuildCollections()
}

const handleDeleteAllCollections = async () => {
  closeCollectionContextMenu()
  const confirmed = await openDialog({
    title: '删除全部合集',
    message: `将删除 ${collections.value.length} 个合集的成员关系、待确认项和排除记录。漫画文件与多版本数据不会删除。`,
    type: 'danger',
    confirmText: '删除全部合集',
  })
  if (!confirmed) return
  try {
    const result = await deleteAllCollections()
    selectedCollectionId.value = null
    await Promise.all([refetchCollections(), refetchCollectionReviews(), refetchSelectedCollection()])
    await showInfoDialog(`已删除 ${result.deleted} 个合集；漫画文件和多版本数据未受影响。`, '合集已删除')
  } catch (error) {
    console.error('删除全部合集失败:', error)
    await showInfoDialog('无法删除全部合集，请稍后重试。', '操作失败')
  }
}

const openVersionGroup = (group: VersionGroup) => {
  selectedVersionGroup.value = group
}

const openVersionComparison = (groupId: string, archiveIds: string[], memberIds: string[]) => {
  if (archiveIds.length < 2) return
  saveViewSnapshot()
  selectedVersionGroup.value = null
  router.push({ name: 'version-compare', query: { ids: archiveIds.slice(0, 4).join(','), members: memberIds.join(','), group: groupId } })
}

const toggleVersionGroupSelection = (id: string) => {
  const next = new Set(selectedVersionGroupIds.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  selectedVersionGroupIds.value = next
}

const handleBatchVersionGroups = async () => {
  const ids = [...selectedVersionGroupIds.value]
  if (!ids.length) return
  versionBatchBusy.value = true
  try {
    if (versionStatus.value === 'keep_all') {
      await Promise.all(ids.map(id => restoreVersionGroup(id)))
    } else {
      await Promise.all(ids.map(id => keepAllVersions(id)))
    }
    selectedVersionGroupIds.value = new Set()
    await refetchVersionGroups()
  } catch (error) {
    console.error('批量处理多版本失败:', error)
    await showInfoDialog(versionStatus.value === 'keep_all' ? '部分版本组未能恢复待处理，请刷新后重试。' : '部分版本组未能标记为全部保留，请刷新后重试。', '操作失败')
  } finally {
    versionBatchBusy.value = false
  }
}

const handleKeepAllVersions = async (id: string) => {
  try {
    await keepAllVersions(id)
    selectedVersionGroup.value = null
    await refetchVersionGroups()
  } catch (error) {
    console.error('保留多版本失败:', error)
    await showInfoDialog('无法记录此版本组，请稍后重试。', '操作失败')
  }
}

const handleRestoreVersionGroup = async (id: string) => {
  try {
    await restoreVersionGroup(id)
    selectedVersionGroup.value = null
    await refetchVersionGroups()
  } catch (error) {
    console.error('恢复多版本待处理失败:', error)
    await showInfoDialog('无法恢复此版本组，请稍后重试。', '操作失败')
  }
}

const handleVersionCleanup = async (group: VersionGroup, keepArchiveId: string) => {
  const deleteArchiveIds = group.members.filter(member => member.archive.id !== keepArchiveId).map(member => member.archive.id)
  const confirmed = await openDialog({
    title: '确认清理多版本',
    message: `将保留选中的版本，并永久删除另外 ${deleteArchiveIds.length} 个文件。标签、静态分类和阅读进度会迁移到保留版本。`,
    type: 'danger',
    confirmText: '删除其他版本',
  })
  if (!confirmed) return
  try {
    const result = await cleanupVersions(group.id, keepArchiveId, deleteArchiveIds)
    selectedVersionGroup.value = null
    await Promise.all([refetchVersionGroups(), refetchCollections(), refetchSelectedCollection(), refetch()])
    const message = result.failedArchiveIds.length
      ? `已删除 ${result.deleted} 个版本；${result.failedArchiveIds.length} 个文件未能删除。`
      : `已删除 ${result.deleted} 个版本，保留版本及关联信息已更新。`
    await showInfoDialog(message, '多版本清理')
  } catch (error) {
    console.error('清理多版本失败:', error)
    await showInfoDialog('无法完成多版本清理，请稍后重试。', '操作失败')
  }
}

const handleRebuildCollections = async () => {
  if (collectionsRebuilding.value) return
  collectionsRebuilding.value = true
  try {
    const result = await rebuildCollections()
    await Promise.all([refetchCollections(), refetchCollectionReviews(), refetchSelectedCollection(), refetchVersionGroups()])
    await showInfoDialog(
      `已分析 ${result.parsedArchives} 本漫画，创建或更新 ${result.createdCollections} 个合集，加入 ${result.groupedArchives} 本成员。${result.pendingReviews ? `另有 ${result.pendingReviews} 条待确认。` : ''}`,
      '合集识别完成',
    )
  } catch (error) {
    console.error('合集识别失败:', error)
    await showInfoDialog('无法完成合集识别，请稍后重试。', '操作失败')
  } finally {
    collectionsRebuilding.value = false
  }
}

const handleCollectionReviewChanged = async () => {
  await Promise.all([refetchCollections(), refetchCollectionReviews(), refetchSelectedCollection()])
}

const handleRemoveCollectionMember = async (archiveId: string) => {
  const confirmed = await openDialog({
    title: '移出合集',
    message: '移出后会记录为“不是同一合集”，后续自动识别不会再次加入。',
    type: 'warning',
    confirmText: '移出',
  })
  if (!confirmed) return
  try {
    await removeCollectionMember(archiveId)
    await Promise.all([refetchCollections(), refetchCollectionReviews(), refetchSelectedCollection()])
  } catch (error) {
    console.error('移出合集失败:', error)
    await showInfoDialog('无法移出该漫画，请稍后重试。', '操作失败')
  }
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

const openReader = (archiveId: string, collectionId?: string) => {
  saveViewSnapshot()
  router.push({
    name: 'reader',
    params: { id: archiveId },
    query: collectionId ? { collection: collectionId } : undefined,
  })
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

const handleRetryTitleTranslationFromContext = async () => {
  if (!contextMenuArchive.value) return
  const archive = contextMenuArchive.value
  closeContextMenu()

  try {
    const result = await retryArchiveTitleTranslation(archive.id)
    await showInfoDialog(
      result.queued
        ? `《${archive.title}》已重新加入标题翻译队列。旧译文会保留到新译文完成。`
        : `《${archive.title}》已经在标题翻译队列中。`,
      '标题翻译',
    )
  } catch (error) {
    console.error('重新翻译标题失败:', error)
    await showInfoDialog('无法创建标题翻译任务，请检查 AI 设置后重试。', '操作失败')
  }
}

const handleShowCollectionsFromContext = () => {
  const title = contextMenuArchive.value?.title || ''
  closeContextMenu()
  searchQuery.value = title
  setLibraryViewMode('collections')
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
