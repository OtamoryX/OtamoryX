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
        :current-filters="filtersForCurrentView"
        :view-mode="libraryViewMode"
        :can-save-dynamic-category="canSaveCurrentSearchAsDynamicCategory"
        @apply-filters="handleAdvancedFilters"
        @reset-filters="handleResetFilters"
        @save-dynamic-category="handleSaveCurrentSearchAsDynamicCategory"
      />
    </div>

    <!-- 主内容区 -->
    <main
      :class="[
        'pt-[calc(env(safe-area-inset-top,0px)+3.5rem)] md:pt-14 pb-[calc(env(safe-area-inset-bottom,0px)+4rem)] md:pb-4 transition-all',
        showAdvancedSearch ? 'md:pt-44' : '',
      ]"
    >
      <div class="mx-auto w-full max-w-[1440px]">
        <!-- 随机精选（始终渲染，内部控制折叠）-->
        <RandomCarousel
          v-if="libraryViewMode === 'single'"
          :category-id="libraryStore.selectedCategoryId || ''"
          :search-query="searchQuery"
          :tags="advancedFilters.tags"
          :min-pages="advancedFilters.minPages"
          :max-pages="advancedFilters.maxPages"
          :min-file-size="advancedFilters.minFileSize"
          :max-file-size="advancedFilters.maxFileSize"
          :created-after="advancedFilters.createdAfter"
          :created-before="advancedFilters.createdBefore"
          @open-archive="(archiveId, recommendationSessionId, recommendationPosition) => openReader(archiveId, undefined, recommendationSessionId, recommendationPosition)"
          @archive-contextmenu="handleArchiveContextMenu"
        />

        <!-- 移动端：活跃筛选 chips 条 -->
        <div
          v-if="searchQuery || activeFilterCount > 0"
          class="md:hidden px-3 py-2 flex items-center gap-2 overflow-x-auto border-b border-[var(--border)] bg-[var(--bg-primary)]"
          style="scrollbar-width: none"
        >
          <!-- 搜索词 chip -->
          <span
            v-if="searchQuery"
            class="inline-flex items-center gap-1 flex-shrink-0 px-2.5 py-1 rounded-full text-xs bg-[var(--accent)]/20 text-[var(--accent)] border border-[var(--accent)]/30"
          >
            <svg
              class="w-3 h-3"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
              />
            </svg>
            {{ searchQuery }}
            <button @click="handleClearSearch">×</button>
          </span>
          <!-- 标签 chips -->
          <span
            v-for="tag in advancedFilters.tags || []"
            :key="tag"
            class="inline-flex items-center gap-1 flex-shrink-0 px-2.5 py-1 rounded-full text-xs bg-[var(--bg-tertiary)] text-[var(--text-secondary)] border border-[var(--border)]"
          >
            {{ tag }}
            <button @click="removeActiveTag(tag)">×</button>
          </span>
          <!-- 页数范围 chip -->
          <span
            v-if="
              advancedFilters.minPages != null ||
              advancedFilters.maxPages != null
            "
            class="inline-flex items-center gap-1 flex-shrink-0 px-2.5 py-1 rounded-full text-xs bg-[var(--bg-tertiary)] text-[var(--text-secondary)] border border-[var(--border)]"
          >
            页数 {{ advancedFilters.minPages ?? "?" }}~{{
              advancedFilters.maxPages ?? "?"
            }}
            <button @click="removePageFilter">×</button>
          </span>
          <span
            v-if="
              advancedFilters.minFileSize != null ||
              advancedFilters.maxFileSize != null
            "
            class="inline-flex items-center gap-1 flex-shrink-0 px-2.5 py-1 rounded-full text-xs bg-[var(--bg-tertiary)] text-[var(--text-secondary)] border border-[var(--border)]"
            >文件大小<button @click="removeFileSizeFilter">×</button></span
          >
          <!-- 日期范围 chip -->
          <span
            v-if="advancedFilters.createdAfter || advancedFilters.createdBefore"
            class="inline-flex items-center gap-1 flex-shrink-0 px-2.5 py-1 rounded-full text-xs bg-[var(--bg-tertiary)] text-[var(--text-secondary)] border border-[var(--border)]"
          >
            时间筛选
            <button @click="removeDateFilter">×</button>
          </span>
          <!-- 重置全部 -->
          <button
            v-if="activeFilterCount > 0 || searchQuery"
            class="flex-shrink-0 ml-auto px-3 py-1 text-xs text-red-400 hover:text-red-300 transition-colors"
            @click="handleMobileClearAll"
          >
            清除全部
          </button>
        </div>

        <section v-if="libraryViewMode === 'collections'" class="px-3 pb-6">
          <div class="flex items-center justify-between gap-3 px-1 py-3">
            <div>
              <div class="flex items-center gap-2">
                <h2 class="text-sm font-medium text-[var(--text-primary)]">
                  合集
                </h2>
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
              <p class="mt-0.5 text-xs text-[var(--text-tertiary)]">
                {{ collections.length }} 个合集<span v-if="hasMemberFilter"
                  >，按合集内命中文件筛选</span
                >
              </p>
            </div>
            <div v-if="authStore.isAdmin" class="flex items-center gap-2">
              <button
                class="flex items-center gap-1.5 px-2.5 py-1.5 rounded text-xs border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] disabled:opacity-60"
                :disabled="collectionsPreviewLoading"
                @click="handlePreviewCollections"
              >
                <svg
                  class="w-3.5 h-3.5"
                  :class="collectionsPreviewLoading ? 'animate-spin' : ''"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2M15 20h-4"
                  />
                </svg>
                {{ collectionsPreviewLoading ? "分析中..." : "预览识别结果" }}
              </button>
            </div>
          </div>
          <div
            v-if="collectionsLoading"
            class="flex items-center justify-center py-20 text-sm text-[var(--text-secondary)]"
          >
            加载合集...
          </div>
          <div
            v-else-if="collections.length === 0"
            class="py-20 text-center text-sm text-[var(--text-secondary)]"
          >
            <p>尚未发现可展示的合集。</p>
            <button
              v-if="authStore.isAdmin"
              class="mt-3 text-[var(--accent)] hover:underline"
              :disabled="collectionsPreviewLoading"
              @click="handlePreviewCollections"
            >
              预览本地识别结果
            </button>
          </div>
          <div
            v-else
            class="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-5 lg:grid-cols-6 xl:grid-cols-7 2xl:grid-cols-8 gap-2"
          >
            <CollectionCard
              v-for="collection in pagedCollections"
              :key="collection.id"
              :collection="collection"
              @open="openCollection"
              @contextmenu="handleCollectionContextMenu"
            />
          </div>
          <div
            v-if="collectionTotalPages > 1"
            class="flex items-center justify-center space-x-1.5 px-4 py-4"
          >
            <button
              :disabled="collectionPage === 1"
              class="min-h-10 px-3 py-1 text-xs rounded border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
              @click="goToCollectionPage(collectionPage - 1)"
            >
              上一页
            </button>
            <button
              v-if="collectionShowFirstPage"
              :class="collectionPageButtonClass(1)"
              @click="goToCollectionPage(1)"
            >
              1
            </button>
            <span
              v-if="collectionShowLeftEllipsis"
              class="px-1 text-[var(--text-tertiary)] text-xs"
              >...</span
            >
            <button
              v-for="page in collectionVisiblePages"
              :key="page"
              :class="collectionPageButtonClass(page)"
              @click="goToCollectionPage(page)"
            >
              {{ page }}
            </button>
            <span
              v-if="collectionShowRightEllipsis"
              class="px-1 text-[var(--text-tertiary)] text-xs"
              >...</span
            >
            <button
              v-if="collectionShowLastPage"
              :class="collectionPageButtonClass(collectionTotalPages)"
              @click="goToCollectionPage(collectionTotalPages)"
            >
              {{ collectionTotalPages }}
            </button>
            <button
              :disabled="collectionPage === collectionTotalPages"
              class="min-h-10 px-3 py-1 text-xs rounded border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
              @click="goToCollectionPage(collectionPage + 1)"
            >
              下一页
            </button>
          </div>
        </section>

        <section v-else-if="libraryViewMode === 'versions'" class="px-3 pb-6">
          <div class="flex items-center justify-between gap-3 px-1 py-3">
            <div>
              <h2 class="text-sm font-medium text-[var(--text-primary)]">
                多版本
              </h2>
              <p class="mt-0.5 text-xs text-[var(--text-tertiary)]">
                {{ versionGroups.length }} 组可比较内容
              </p>
            </div>
            <div
              class="flex rounded border border-[var(--border)] overflow-hidden text-xs"
            >
              <button
                class="px-2.5 py-1.5"
                :class="
                  versionStatus === 'active'
                    ? 'bg-[var(--bg-tertiary)] text-[var(--text-primary)]'
                    : 'text-[var(--text-secondary)]'
                "
                @click="versionStatus = 'active'"
              >
                待处理
              </button>
              <button
                class="px-2.5 py-1.5"
                :class="
                  versionStatus === 'keep_all'
                    ? 'bg-[var(--bg-tertiary)] text-[var(--text-primary)]'
                    : 'text-[var(--text-secondary)]'
                "
                @click="versionStatus = 'keep_all'"
              >
                无需处理
              </button>
            </div>
          </div>
          <div
            v-if="selectedVersionGroupIds.size"
            class="mb-3 flex flex-wrap items-center justify-between gap-3 border-y border-[var(--border)] py-2"
          >
            <span class="text-xs text-[var(--text-secondary)]"
              >已选择 {{ selectedVersionGroupIds.size }} 组<span
                v-if="
                  versionStatus === 'active' &&
                  selectedRecommendedVersionGroups.length
                "
                class="text-emerald-400"
                >，其中
                {{
                  selectedRecommendedVersionGroups.length
                }}
                组可按推荐处理</span
              ></span
            >
            <div class="flex flex-wrap items-center gap-2">
              <button
                v-if="versionStatus === 'active'"
                class="inline-flex h-8 items-center border border-red-500/60 px-2.5 text-xs text-red-400 transition-colors hover:bg-red-500/10 disabled:cursor-not-allowed disabled:opacity-50"
                :disabled="
                  versionBatchBusy ||
                  !selectedRecommendedVersionGroups.length ||
                  !authStore.isAdmin
                "
                title="保留每组推荐版本，并永久删除同组其余文件"
                @click="handleBatchVersionGroups('recommended-cleanup')"
              >
                {{ versionBatchBusy ? "处理中..." : "批量按推荐清理" }}
              </button>
              <button
                class="inline-flex h-8 items-center border border-[var(--border)] px-2.5 text-xs text-[var(--text-secondary)] transition-colors hover:bg-[var(--bg-tertiary)] disabled:cursor-not-allowed disabled:opacity-50"
                :disabled="versionBatchBusy"
                @click="
                  handleBatchVersionGroups(
                    versionStatus === 'keep_all' ? 'restore' : 'keep-all',
                  )
                "
              >
                {{
                  versionBatchBusy
                    ? "处理中..."
                    : versionStatus === "keep_all"
                      ? "批量恢复待处理"
                      : "批量标记无需处理"
                }}
              </button>
            </div>
          </div>
          <div
            v-if="versionGroupsLoading"
            class="flex items-center justify-center py-20 text-sm text-[var(--text-secondary)]"
          >
            加载多版本...
          </div>
          <div
            v-else-if="versionGroups.length === 0"
            class="py-20 text-center text-sm text-[var(--text-secondary)]"
          >
            没有发现需要比较的多版本。
          </div>
          <div
            v-else
            class="mx-auto grid max-w-7xl grid-cols-3 gap-2 px-3 pb-4 sm:grid-cols-4 md:grid-cols-5 lg:grid-cols-6"
          >
            <VersionGroupCard
              v-for="group in versionGroups"
              :key="group.id"
              :group="group"
              :selected="selectedVersionGroupIds.has(group.id)"
              @open="openVersionGroup"
              @toggle="toggleVersionGroupSelection"
            />
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
          <div
            v-if="error"
            class="mx-4 mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30"
          >
            <div class="flex items-center text-red-400 text-sm">
              <svg
                class="w-4 h-4 mr-2 flex-shrink-0"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
              加载失败: {{ error.message }}
            </div>
          </div>

          <!-- 加载中 -->
          <div v-if="isLoading" class="flex items-center justify-center py-20">
            <div
              class="w-6 h-6 border-2 border-[var(--border)] border-t-[var(--accent)] rounded-full animate-spin"
            />
            <span class="ml-3 text-sm text-[var(--text-secondary)]"
              >加载中...</span
            >
          </div>

          <!-- 空状态 -->
          <div
            v-else-if="archives.length === 0"
            class="flex flex-col items-center justify-center py-20 text-[var(--text-secondary)]"
          >
            <svg
              class="w-16 h-16 mb-4 opacity-40"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="1"
                d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
              />
            </svg>
            <p>
              {{
                libraryStore.selectedCategoryId
                  ? "该分类下没有漫画"
                  : "没有找到漫画"
              }}
            </p>
          </div>

          <!-- 漫画网格 -->
          <div
            v-else
            class="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-5 lg:grid-cols-6 xl:grid-cols-7 2xl:grid-cols-8 gap-2 px-3 pb-4"
          >
            <ArchiveThumbnailCard
              v-for="archive in archives"
              :key="archive.id"
              :archive="archive"
              :progress-percentage="
                progressData.get(archive.id)?.progressPercentage
              "
              @click="openReader(archive.id)"
              @contextmenu="handleArchiveContextMenu"
            />
          </div>

          <!-- 分页 -->
          <div
            v-if="totalPages > 1"
            class="flex items-center justify-center space-x-1.5 px-4 py-4"
          >
            <button
              :disabled="currentPage === 1"
              class="min-h-10 px-3 py-1 text-xs rounded border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
              @click="goToPage(currentPage - 1)"
            >
              上一页
            </button>
            <button
              class="min-h-10 min-w-18 px-3 py-1 text-xs rounded border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] md:hidden"
              title="跳转页码"
              @click="openMobilePageJump"
            >
              {{ currentPage }} / {{ totalPages }}
            </button>
            <span class="hidden md:contents">
              <button
                v-if="showFirstPage"
                :class="pageButtonClass(1)"
                @click="goToPage(1)"
              >
                1
              </button>
              <span
                v-if="showLeftEllipsis"
                class="px-1 text-[var(--text-tertiary)] text-xs"
                >...</span
              >
              <button
                v-for="page in visiblePages"
                :key="page"
                :class="pageButtonClass(page)"
                @click="goToPage(page)"
              >
                {{ page }}
              </button>
              <span
                v-if="showRightEllipsis"
                class="px-1 text-[var(--text-tertiary)] text-xs"
                >...</span
              >
              <button
                v-if="showLastPage"
                :class="pageButtonClass(totalPages)"
                @click="goToPage(totalPages)"
              >
                {{ totalPages }}
              </button>
            </span>
            <button
              :disabled="currentPage === totalPages"
              class="min-h-10 px-3 py-1 text-xs rounded border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
              @click="goToPage(currentPage + 1)"
            >
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
      :current-filters="filtersForCurrentView"
      :view-mode="libraryViewMode"
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
      @delete-with-members="handleDeleteCollectionWithMembers"
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

    <BaseModal
      :show="showMobilePageJump"
      title="跳转页码"
      width="sm"
      @close="showMobilePageJump = false"
    >
      <form class="space-y-4" @submit.prevent="applyMobilePageJump">
        <input
          v-model.number="mobilePageJump"
          class="w-full rounded border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-3 text-center text-base text-[var(--text-primary)] outline-none focus:border-[var(--accent)]"
          type="number"
          min="1"
          :max="totalPages"
          inputmode="numeric"
        />
        <button
          class="h-11 w-full rounded bg-[var(--accent)] text-sm font-medium text-white"
          type="submit"
        >
          跳转
        </button>
      </form>
    </BaseModal>

    <CollectionDetailPanel
      :show="selectedCollectionId !== null"
      :detail="selectedCollectionDetail"
      :is-loading="collectionDetailLoading"
      :has-member-filter="hasMemberFilter"
      @close="selectedCollectionId = null"
      @open-reader="openReader"
      @remove-member="handleRemoveCollectionMember"
      @reviews-changed="handleCollectionReviewChanged"
    />
    <CollectionEditModal
      :show="showCollectionEditModal"
      :collection="editingCollection"
      @close="showCollectionEditModal = false"
      @save="handleSaveCollection"
    />
    <VersionGroupPanel
      :show="selectedVersionGroup !== null"
      :group="selectedVersionGroup"
      :can-manage="authStore.isAdmin"
      @close="selectedVersionGroup = null"
      @open-reader="openReader"
      @open-comparison="openVersionComparison"
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
    <BaseModal
      :show="authStore.isAdmin && showCollectionPreview"
      title="合集识别预览"
      width="xl"
      max-height="full"
      @close="showCollectionPreview = false"
    >
      <div v-if="collectionPreview" class="space-y-5 text-sm">
        <p class="text-[var(--text-secondary)]">
          已分析
          {{
            collectionPreview.parsedArchives
          }}
          本漫画。本次预览不会改动书库；确认应用后才会更新合集。
        </p>
        <section>
          <div class="flex items-center justify-between gap-3">
            <h3 class="font-medium text-[var(--text-primary)]">将识别为合集</h3>
            <span class="text-xs text-[var(--text-tertiary)]"
              >{{ collectionPreview.collectionCandidates.length }} 组</span
            >
          </div>
          <div
            v-if="collectionPreview.collectionCandidates.length"
            class="mt-2 divide-y divide-[var(--border)] border-y border-[var(--border)]"
          >
            <div
              v-for="item in collectionPreview.collectionCandidates"
              :key="`${item.displayTitle}-${item.reason}`"
              class="py-2.5"
            >
              <div class="flex items-center justify-between gap-3">
                <span
                  class="min-w-0 truncate text-xs font-medium text-[var(--text-primary)]"
                  >{{ item.displayTitle }}</span
                ><span
                  class="shrink-0 text-[10px]"
                  :class="
                    item.status === 'needs_review'
                      ? 'text-amber-400'
                      : 'text-emerald-400'
                  "
                  >{{ item.memberCount }} 本 ·
                  {{
                    item.status === "needs_review" ? "含待确认" : "可自动加入"
                  }}</span
                >
              </div>
              <p class="mt-1 text-[10px] text-[var(--text-tertiary)]">
                {{ item.reason }}
              </p>
            </div>
          </div>
          <p v-else class="mt-2 text-xs text-[var(--text-tertiary)]">
            没有新的合集候选。
          </p>
        </section>
        <section>
          <div class="flex items-center justify-between gap-3">
            <h3 class="font-medium text-[var(--text-primary)]">疑似多版本</h3>
            <span class="text-xs text-[var(--text-tertiary)]"
              >{{ collectionPreview.versionCandidates.length }} 组</span
            >
          </div>
          <p class="mt-1 text-xs text-[var(--text-tertiary)]">
            这些文件属于同一内容单元，不会被当成合集成员。
          </p>
          <div
            v-if="collectionPreview.versionCandidates.length"
            class="mt-2 divide-y divide-[var(--border)] border-y border-[var(--border)]"
          >
            <div
              v-for="item in collectionPreview.versionCandidates"
              :key="`${item.displayTitle}-${item.memberCount}`"
              class="py-2.5"
            >
              <div class="flex items-center justify-between gap-3">
                <span
                  class="min-w-0 truncate text-xs font-medium text-[var(--text-primary)]"
                  >{{ item.displayTitle }}</span
                ><span class="shrink-0 text-[10px] text-[var(--text-tertiary)]"
                  >{{ item.memberCount }} 个版本</span
                >
              </div>
              <p class="mt-1 text-[10px] text-[var(--text-tertiary)]">
                {{ item.reason }}
              </p>
            </div>
          </div>
        </section>
      </div>
      <template #footer>
        <div class="flex items-center justify-end gap-2">
          <button
            class="h-9 border border-[var(--border)] px-3 text-xs text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)]"
            @click="showCollectionPreview = false"
          >
            取消</button
          ><button
            class="h-9 bg-[var(--accent)] px-3 text-xs text-white hover:opacity-90 disabled:opacity-50"
            :disabled="collectionsRebuilding"
            @click="handleRebuildCollections"
          >
            {{
              collectionsRebuilding
                ? "应用中..."
                : `应用识别结果${collectionPreview?.pendingReviewCount ? `（${collectionPreview.pendingReviewCount} 项待确认）` : ""}`
            }}
          </button>
        </div>
      </template>
    </BaseModal>
  </div>
</template>

<script setup lang="ts">
import {
  ref,
  computed,
  watch,
  onMounted,
  onUnmounted,
  onActivated,
  onDeactivated,
  nextTick,
} from "vue";
import { useRouter, useRoute } from "vue-router";
import { useQuery, useQueryClient } from "@tanstack/vue-query";
import { ExclamationTriangleIcon } from "@heroicons/vue/24/outline";
import { useAuthStore } from "@/stores/auth";
import { useLibraryStore } from "@/stores/library";
import LibraryTopBar from "@/components/library/LibraryTopBar.vue";
import CategoryBottomBar from "@/components/library/CategoryBottomBar.vue";
import RandomCarousel from "@/components/library/RandomCarousel.vue";
import MobileSearchModal from "@/components/library/MobileSearchModal.vue";
import AdvancedSearchPanel from "@/components/library/AdvancedSearchPanel.vue";
import ArchiveThumbnailCard from "@/components/ArchiveThumbnailCard.vue";
import CollectionCard from "@/components/CollectionCard.vue";
import CollectionDetailPanel from "@/components/CollectionDetailPanel.vue";
import CollectionReviewModal from "@/components/CollectionReviewModal.vue";
import CollectionContextMenu from "@/components/CollectionContextMenu.vue";
import CollectionEditModal from "@/components/CollectionEditModal.vue";
import VersionGroupCard from "@/components/VersionGroupCard.vue";
import VersionGroupPanel from "@/components/VersionGroupPanel.vue";
import CategoryModal from "@/components/CategoryModal.vue";
import ArchiveContextMenu from "@/components/ArchiveContextMenu.vue";
import TagModal from "@/components/common/TagModal.vue";
import ConfirmModal from "@/components/common/ConfirmModal.vue";
import BaseModal from "@/components/base/BaseModal.vue";
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
  previewCollectionRebuild,
  rebuildCollections,
  removeCollectionMember,
  updateCollection,
  getVersionGroups,
  cleanupVersions,
  keepAllVersions,
  restoreVersionGroup,
  deleteCollectionWithMembers,
  deleteArchive,
} from "@/utils/api";
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
  CollectionRebuildPreview,
  VersionGroup,
} from "@/types/api";

const router = useRouter();
const route = useRoute();
const queryClient = useQueryClient();
const authStore = useAuthStore();
const libraryStore = useLibraryStore();

const LIBRARY_VIEW_SNAPSHOT_KEY = "library-view-snapshot-v1";
const LIBRARY_RETURN_ARCHIVE_KEY = "library-return-archive-id";

interface LibraryViewSnapshot {
  searchQuery: string;
  currentPage: number;
  advancedFilters: Partial<SearchParams>;
  showAdvancedSearch: boolean;
  selectedCategoryId: string | null;
  scrollTop: number;
}

const loadLibraryViewSnapshot = (): LibraryViewSnapshot | null => {
  try {
    const raw = sessionStorage.getItem(LIBRARY_VIEW_SNAPSHOT_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object") return null;

    return {
      searchQuery:
        typeof parsed.searchQuery === "string" ? parsed.searchQuery : "",
      currentPage:
        Number.isInteger(parsed.currentPage) && parsed.currentPage > 0
          ? parsed.currentPage
          : 1,
      advancedFilters:
        parsed.advancedFilters && typeof parsed.advancedFilters === "object"
          ? parsed.advancedFilters
          : {},
      showAdvancedSearch: Boolean(parsed.showAdvancedSearch),
      selectedCategoryId:
        typeof parsed.selectedCategoryId === "string" ||
        parsed.selectedCategoryId === null
          ? parsed.selectedCategoryId
          : null,
      scrollTop:
        typeof parsed.scrollTop === "number" && parsed.scrollTop >= 0
          ? parsed.scrollTop
          : 0,
    };
  } catch (error) {
    console.error("Failed to parse library snapshot:", error);
    return null;
  }
};

const initialSnapshot = loadLibraryViewSnapshot();
if (
  initialSnapshot &&
  initialSnapshot.selectedCategoryId !== libraryStore.selectedCategoryId
) {
  libraryStore.selectCategory(initialSnapshot.selectedCategoryId);
}

// 基础状态
const searchQuery = ref(initialSnapshot?.searchQuery ?? "");
const currentPage = ref(initialSnapshot?.currentPage ?? 1);
const collectionPage = ref(1);
const showMobilePageJump = ref(false);
const mobilePageJump = ref(currentPage.value);
const libraryViewMode = ref<"single" | "collections" | "versions">("single");
const selectedCollectionId = ref<string | null>(null);
const showCollectionReviews = ref(false);
const collectionsRebuilding = ref(false);
const collectionsPreviewLoading = ref(false);
const showCollectionPreview = ref(false);
const collectionPreview = ref<CollectionRebuildPreview | null>(null);
const versionStatus = ref("active");
const selectedVersionGroup = ref<VersionGroup | null>(null);
const selectedVersionGroupIds = ref(new Set<string>());
const versionBatchBusy = ref(false);
const viewSorts = ref<
  Record<
    "single" | "collections" | "versions",
    { sortBy: string; sortOrder: string }
  >
>({
  single: { sortBy: "createdAt", sortOrder: "asc" },
  collections: { sortBy: "recognitionPriority", sortOrder: "asc" },
  versions: { sortBy: "recognitionPriority", sortOrder: "asc" },
});

// 动态 pageSize：列数 × 每页行数
function getColumnsCount(): number {
  const w = window.innerWidth;
  if (w >= 1536) return 8;
  if (w >= 1280) return 7;
  if (w >= 1024) return 6;
  if (w >= 768) return 5;
  if (w >= 640) return 4;
  return 3;
}
const columns = ref(getColumnsCount());
const pageSize = computed(() => columns.value * libraryStore.rowsPerPage);

let resizeTimer: ReturnType<typeof setTimeout>;
function onResize() {
  clearTimeout(resizeTimer);
  resizeTimer = setTimeout(() => {
    if (window.innerWidth < 768 && libraryViewMode.value === "versions")
      libraryViewMode.value = "single";
    const newCols = getColumnsCount();
    if (newCols !== columns.value) {
      columns.value = newCols;
      currentPage.value = 1;
    }
  }, 200);
}

const showCreateCategoryModal = ref(false);
const showEditCategoryModal = ref(false);
const selectedCategory = ref<Category | DynamicCategory | null>(null);
const createCategoryInitialType = ref<"static" | "dynamic">("static");
const createCategoryInitialSearchParams = ref<Partial<SearchParams>>({});

// 高级搜索面板
const showAdvancedSearch = ref(initialSnapshot?.showAdvancedSearch ?? false);
const advancedFilters = ref<Partial<SearchParams>>(
  initialSnapshot?.advancedFilters ?? {},
);
const restoredScrollTop = ref<number | null>(
  initialSnapshot?.scrollTop ?? null,
);

const saveViewSnapshot = (scrollTop = window.scrollY) => {
  try {
    restoredScrollTop.value = scrollTop;
    const snapshot: LibraryViewSnapshot = {
      searchQuery: searchQuery.value,
      currentPage: currentPage.value,
      advancedFilters: advancedFilters.value,
      showAdvancedSearch: showAdvancedSearch.value,
      selectedCategoryId: libraryStore.selectedCategoryId,
      scrollTop,
    };
    sessionStorage.setItem(LIBRARY_VIEW_SNAPSHOT_KEY, JSON.stringify(snapshot));
  } catch (error) {
    console.error("Failed to save library snapshot:", error);
  }
};

const restoreViewScroll = () => {
  if (restoredScrollTop.value == null) return;
  const top = restoredScrollTop.value;
  nextTick(() => {
    requestAnimationFrame(() => {
      window.scrollTo({ top, behavior: "auto" });
    });
  });
};

const activeFilterCount = computed(() => {
  let count = 0;
  if (advancedFilters.value.tags && advancedFilters.value.tags.length > 0)
    count++;
  if (
    advancedFilters.value.minPages != null ||
    advancedFilters.value.maxPages != null
  )
    count++;
  if (
    advancedFilters.value.minFileSize != null ||
    advancedFilters.value.maxFileSize != null
  )
    count++;
  if (advancedFilters.value.createdAfter || advancedFilters.value.createdBefore)
    count++;
  return count;
});

const currentViewSort = computed(() => viewSorts.value[libraryViewMode.value]);
const filtersForCurrentView = computed<Partial<SearchParams>>(() => ({
  ...advancedFilters.value,
  ...currentViewSort.value,
}));
const hasMemberFilter = computed(() =>
  Boolean(
    libraryStore.selectedCategoryId ||
    searchQuery.value.trim() ||
    advancedFilters.value.tags?.length ||
    advancedFilters.value.minPages != null ||
    advancedFilters.value.maxPages != null ||
    advancedFilters.value.minFileSize != null ||
    advancedFilters.value.maxFileSize != null ||
    advancedFilters.value.createdAfter ||
    advancedFilters.value.createdBefore ||
    advancedFilters.value.lastReadAfter ||
    advancedFilters.value.lastReadBefore,
  ),
);
const memberSearchParams = computed<SearchParams & { categoryId?: string }>(
  () => ({
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
    categoryId: libraryStore.selectedCategoryId || undefined,
  }),
);

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
  sortBy: currentViewSort.value.sortBy,
  sortOrder: currentViewSort.value.sortOrder,
}));

const canSaveCurrentSearchAsDynamicCategory = computed(() => {
  if (searchQuery.value.trim()) return true;
  if (advancedFilters.value.tags?.length) return true;
  if (
    advancedFilters.value.minPages != null ||
    advancedFilters.value.maxPages != null
  )
    return true;
  if (
    advancedFilters.value.minFileSize != null ||
    advancedFilters.value.maxFileSize != null
  )
    return true;
  if (advancedFilters.value.createdAfter || advancedFilters.value.createdBefore)
    return true;
  if (
    advancedFilters.value.lastReadAfter ||
    advancedFilters.value.lastReadBefore
  )
    return true;
  if (
    advancedFilters.value.sortBy &&
    advancedFilters.value.sortBy !== "createdAt"
  )
    return true;
  if (
    advancedFilters.value.sortOrder &&
    advancedFilters.value.sortOrder !== "asc"
  )
    return true;
  return false;
});

// 右键菜单
const showContextMenu = ref(false);
const contextMenuArchive = ref<Archive | null>(null);
const contextMenuPosition = ref({ x: 0, y: 0 });
const showCollectionContextMenu = ref(false);
const contextMenuCollection = ref<CollectionSummary | null>(null);
const collectionContextMenuPosition = ref({ x: 0, y: 0 });
const showCollectionEditModal = ref(false);
const editingCollection = ref<CollectionSummary | null>(null);

// 标签模态框
const showTagModal = ref(false);
const tagModalArchive = ref<Archive | null>(null);

type DialogType = "default" | "danger" | "warning" | "info";

interface DialogOptions {
  title?: string;
  message: string;
  type?: DialogType;
  confirmText?: string;
  cancelText?: string;
  showCancel?: boolean;
}

const dialog = ref({
  show: false,
  title: "提示",
  message: "",
  type: "default" as DialogType,
  confirmText: "确认",
  cancelText: "取消",
  showCancel: true,
});

let dialogResolver: ((result: boolean) => void) | null = null;

const openDialog = (options: DialogOptions): Promise<boolean> => {
  if (dialogResolver) {
    dialogResolver(false);
    dialogResolver = null;
  }

  dialog.value = {
    show: true,
    title: options.title ?? "提示",
    message: options.message,
    type: options.type ?? "default",
    confirmText: options.confirmText ?? "确认",
    cancelText: options.cancelText ?? "取消",
    showCancel: options.showCancel ?? true,
  };

  return new Promise((resolve) => {
    dialogResolver = resolve;
  });
};

const resolveDialog = (result: boolean) => {
  dialog.value.show = false;
  if (dialogResolver) {
    dialogResolver(result);
    dialogResolver = null;
  }
};

const handleDialogClose = () => resolveDialog(false);
const handleDialogConfirm = () => resolveDialog(true);

const showInfoDialog = async (message: string, title = "提示") => {
  await openDialog({
    title,
    message,
    type: "info",
    confirmText: "知道了",
    showCancel: false,
  });
};

// 进度数据
const progressData = ref<Map<string, ReadingProgress>>(new Map());

// 用户名
const userName = computed(() => authStore.user?.username || "");

// 当前分类名称
const currentCategoryName = computed(() => {
  if (!libraryStore.selectedCategoryId) return "全部漫画";
  return "分类漫画";
});

// 搜索参数
const searchParams = computed<SearchParams>(() => ({
  query: searchQuery.value || undefined,
  pageNumb: currentPage.value,
  pageSize: pageSize.value,
  sortBy: advancedFilters.value.sortBy || "createdAt",
  sortOrder: advancedFilters.value.sortOrder || "asc",
  tags: advancedFilters.value.tags,
  minPages: advancedFilters.value.minPages,
  maxPages: advancedFilters.value.maxPages,
  minFileSize: advancedFilters.value.minFileSize,
  maxFileSize: advancedFilters.value.maxFileSize,
  createdAfter: advancedFilters.value.createdAfter,
  createdBefore: advancedFilters.value.createdBefore,
}));

// Query key
const queryKey = computed(() => [
  "archives",
  libraryStore.selectedCategoryId,
  currentPage.value,
  pageSize.value,
  searchQuery.value,
  advancedFilters.value,
]);

// 主查询
const { data, isLoading, refetch, error } = useQuery({
  queryKey,
  queryFn: async () => {
    if (libraryStore.selectedCategoryId) {
      return await getCategoryArchives(
        libraryStore.selectedCategoryId,
        searchParams.value,
      );
    }
    return await searchArchives(searchParams.value);
  },
  retry: 1,
});

const collectionQueryKey = computed(() => [
  "collections",
  memberSearchParams.value,
  currentViewSort.value,
]);
const {
  data: collectionsData,
  isLoading: collectionsLoading,
  refetch: refetchCollections,
} = useQuery({
  queryKey: collectionQueryKey,
  queryFn: () =>
    getCollections({ ...memberSearchParams.value, ...currentViewSort.value }),
  enabled: computed(() => libraryViewMode.value === "collections"),
  retry: 1,
});
const collections = computed<CollectionSummary[]>(
  () => collectionsData.value || [],
);
const collectionTotalPages = computed(() =>
  Math.max(1, Math.ceil(collections.value.length / pageSize.value)),
);
const pagedCollections = computed(() => {
  const start = (collectionPage.value - 1) * pageSize.value;
  return collections.value.slice(start, start + pageSize.value);
});
watch([collections, pageSize], () => {
  if (collectionPage.value > collectionTotalPages.value)
    collectionPage.value = collectionTotalPages.value;
});

const versionGroupQueryKey = computed(() => [
  "versionGroups",
  memberSearchParams.value,
  currentViewSort.value,
  versionStatus.value,
]);
const {
  data: versionGroupsData,
  isLoading: versionGroupsLoading,
  refetch: refetchVersionGroups,
} = useQuery({
  queryKey: versionGroupQueryKey,
  queryFn: () =>
    getVersionGroups({
      ...memberSearchParams.value,
      ...currentViewSort.value,
      status: versionStatus.value,
    }),
  enabled: computed(() => libraryViewMode.value === "versions"),
  retry: 1,
});
const versionGroups = computed<VersionGroup[]>(
  () => versionGroupsData.value || [],
);
const selectedRecommendedVersionGroups = computed(() =>
  versionGroups.value.filter(
    (group) =>
      selectedVersionGroupIds.value.has(group.id) && group.recommendedArchiveId,
  ),
);

watch(versionStatus, () => {
  selectedVersionGroupIds.value = new Set();
});

const { data: collectionReviewsData, refetch: refetchCollectionReviews } =
  useQuery({
    queryKey: ["collectionReviews"],
    queryFn: getCollectionReviews,
    staleTime: 30_000,
    retry: 1,
  });
const collectionReviews = computed<CollectionReviewItem[]>(
  () => collectionReviewsData.value || [],
);

const {
  data: selectedCollectionData,
  isLoading: collectionDetailLoading,
  refetch: refetchSelectedCollection,
} = useQuery({
  queryKey: computed(() => [
    "collection",
    selectedCollectionId.value,
    memberSearchParams.value,
  ]),
  queryFn: () =>
    getCollection(selectedCollectionId.value!, memberSearchParams.value),
  enabled: computed(() => selectedCollectionId.value !== null),
  retry: 1,
});
const selectedCollectionDetail = computed<CollectionDetail | null>(
  () => selectedCollectionData.value || null,
);

const archives = computed<Archive[]>(() => data.value?.data || []);
const totalArchives = computed(() => data.value?.total || 0);
const totalPages = computed(() =>
  Math.ceil(totalArchives.value / pageSize.value),
);

// 全部漫画总数（用于分类下拉显示）
const { data: allArchivesData } = useQuery({
  queryKey: ["allArchivesCount"],
  queryFn: () => searchArchives({ pageNumb: 1, pageSize: 1 }),
  staleTime: 5 * 60 * 1000,
});
const allArchivesCount = computed(() => allArchivesData.value?.total || 0);

// 批量进度查询
const { data: batchProgressData } = useQuery({
  queryKey: ["batchProgress", archives],
  queryFn: async () => {
    const ids = archives.value.map((a) => a.id);
    if (ids.length === 0) return [];
    return await getBatchProgress(ids);
  },
  enabled: computed(() => archives.value.length > 0),
  retry: false,
  staleTime: 5 * 60 * 1000,
});

watch(
  batchProgressData,
  (newData) => {
    if (newData) {
      const map = new Map<string, ReadingProgress>();
      newData.forEach((p) => map.set(p.archiveId, p));
      progressData.value = map;
    }
  },
  { immediate: true },
);

watch(
  [
    searchQuery,
    currentPage,
    advancedFilters,
    showAdvancedSearch,
    () => libraryStore.selectedCategoryId,
  ],
  () => {
    saveViewSnapshot();
  },
  { deep: true },
);

// 分页逻辑
const visiblePages = computed(() => {
  const pages: number[] = [];
  const maxVisible = 5;
  let start = Math.max(1, currentPage.value - Math.floor(maxVisible / 2));
  let end = Math.min(totalPages.value, start + maxVisible - 1);
  if (end - start + 1 < maxVisible) start = Math.max(1, end - maxVisible + 1);
  for (let i = start; i <= end; i++) {
    if (i !== 1 && i !== totalPages.value) pages.push(i);
  }
  return pages;
});
const showFirstPage = computed(
  () => !visiblePages.value.includes(1) && totalPages.value > 1,
);
const showLastPage = computed(
  () => !visiblePages.value.includes(totalPages.value) && totalPages.value > 1,
);
const showLeftEllipsis = computed(
  () => visiblePages.value.length > 0 && visiblePages.value[0]! > 2,
);
const showRightEllipsis = computed(
  () =>
    visiblePages.value.length > 0 &&
    visiblePages.value[visiblePages.value.length - 1]! < totalPages.value - 1,
);

const pageButtonClass = (page: number) => [
  "w-7 h-7 text-xs rounded transition-colors",
  currentPage.value === page
    ? "bg-[var(--accent)] text-white"
    : "border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)]",
];
const collectionVisiblePages = computed(() => {
  const pages: number[] = [];
  const maxVisible = 5;
  let start = Math.max(1, collectionPage.value - Math.floor(maxVisible / 2));
  let end = Math.min(collectionTotalPages.value, start + maxVisible - 1);
  if (end - start + 1 < maxVisible)
    start = Math.max(1, end - maxVisible + 1);
  for (let page = start; page <= end; page++) {
    if (page !== 1 && page !== collectionTotalPages.value) pages.push(page);
  }
  return pages;
});
const collectionShowFirstPage = computed(
  () =>
    !collectionVisiblePages.value.includes(1) && collectionTotalPages.value > 1,
);
const collectionShowLastPage = computed(
  () =>
    !collectionVisiblePages.value.includes(collectionTotalPages.value) &&
    collectionTotalPages.value > 1,
);
const collectionShowLeftEllipsis = computed(
  () => collectionVisiblePages.value[0]! > 2,
);
const collectionShowRightEllipsis = computed(
  () =>
    collectionVisiblePages.value[collectionVisiblePages.value.length - 1]! <
    collectionTotalPages.value - 1,
);
const collectionPageButtonClass = (page: number) => [
  "w-7 h-7 text-xs rounded transition-colors",
  collectionPage.value === page
    ? "bg-[var(--accent)] text-white"
    : "border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)]",
];

const openMobilePageJump = () => {
  mobilePageJump.value = currentPage.value;
  showMobilePageJump.value = true;
};

const applyMobilePageJump = () => {
  const page = Math.max(
    1,
    Math.min(totalPages.value, Number(mobilePageJump.value) || 1),
  );
  showMobilePageJump.value = false;
  void goToPage(page);
};

// 事件处理
const handleTopBarSearch = (query: string) => {
  searchQuery.value = query;
  currentPage.value = 1;
};

const setLibraryViewMode = (mode: "single" | "collections" | "versions") => {
  libraryViewMode.value = mode;
  if (mode === "collections") collectionPage.value = 1;
  if (mode === "collections") void refetchCollections();
  if (mode === "versions") void refetchVersionGroups();
};

const openCollection = (collection: CollectionSummary) => {
  selectedCollectionId.value = collection.id;
};

const handleCollectionContextMenu = (
  event: MouseEvent,
  collection: CollectionSummary,
) => {
  contextMenuCollection.value = collection;
  collectionContextMenuPosition.value = { x: event.clientX, y: event.clientY };
  showCollectionContextMenu.value = true;
};

const closeCollectionContextMenu = () => {
  showCollectionContextMenu.value = false;
  contextMenuCollection.value = null;
};

const handleOpenCollectionFromContext = () => {
  const collection = contextMenuCollection.value;
  closeCollectionContextMenu();
  if (collection) openCollection(collection);
};

const handleContinueCollectionFromContext = async () => {
  const collection = contextMenuCollection.value;
  closeCollectionContextMenu();
  if (!collection) return;
  const detail = await getCollection(collection.id).catch(() => null);
  const archiveId =
    detail?.members.find((member) => member.confidence >= 0.75)?.archive.id ||
    detail?.members[0]?.archive.id;
  if (archiveId) openReader(archiveId);
};

const handleEditCollectionFromContext = () => {
  editingCollection.value = contextMenuCollection.value;
  showCollectionEditModal.value = !!editingCollection.value;
  closeCollectionContextMenu();
};

const handleSaveCollection = async (title: string, subtitle: string) => {
  const collection = editingCollection.value;
  if (!collection) return;
  try {
    await updateCollection(collection.id, { displayTitle: title, subtitle });
    showCollectionEditModal.value = false;
    await Promise.all([refetchCollections(), refetchSelectedCollection()]);
  } catch (error) {
    console.error("更新合集信息失败:", error);
    await showInfoDialog("无法保存合集信息，请稍后重试。", "操作失败");
  }
};

const handleRebuildCollectionFromContext = async () => {
  closeCollectionContextMenu();
  await handleRebuildCollections();
};

const handleDeleteCollectionWithMembers = async () => {
  const collection = contextMenuCollection.value;
  closeCollectionContextMenu();
  if (!collection) return;
  const confirmed = await openDialog({
    title: "确认删除合集及成员漫画",
    message: `将永久删除合集《${collection.displayTitle}》及其中 ${collection.memberCount} 本成员漫画的物理文件。标签、分类和阅读进度等关联关系会按单本删除逻辑清理，此操作不可撤销。`,
    type: "danger",
    confirmText: "删除合集及成员漫画",
  });
  if (!confirmed) return;
  try {
    const result = await deleteCollectionWithMembers(collection.id);
    if (selectedCollectionId.value === collection.id)
      selectedCollectionId.value = null;
    await Promise.all([
      refetchCollections(),
      refetchCollectionReviews(),
      refetchSelectedCollection(),
      refetchVersionGroups(),
      refetch(),
    ]);
    await showInfoDialog(
      `已删除合集《${collection.displayTitle}》及 ${result.deletedArchives} 本成员漫画。`,
      "合集已删除",
    );
  } catch (error) {
    console.error("删除合集及成员漫画失败:", error);
    await showInfoDialog(
      "无法完整删除该合集及其成员漫画；已删除的成员不会恢复，请刷新后检查剩余内容。",
      "操作失败",
    );
  }
};

const openVersionGroup = (group: VersionGroup) => {
  selectedVersionGroup.value = group;
};

const openVersionComparison = (
  groupId: string,
  archiveIds: string[],
  memberIds: string[],
) => {
  if (archiveIds.length < 2) return;
  saveViewSnapshot();
  selectedVersionGroup.value = null;
  router.push({
    name: "version-compare",
    query: {
      ids: archiveIds.slice(0, 4).join(","),
      members: memberIds.join(","),
      group: groupId,
    },
  });
};

const toggleVersionGroupSelection = (id: string) => {
  const next = new Set(selectedVersionGroupIds.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  selectedVersionGroupIds.value = next;
};

type VersionBatchAction = "keep-all" | "restore" | "recommended-cleanup";

const handleBatchVersionGroups = async (action: VersionBatchAction) => {
  const ids = [...selectedVersionGroupIds.value];
  if (!ids.length) return;

  if (action === "recommended-cleanup") {
    const groups = selectedRecommendedVersionGroups.value;
    const skipped = ids.length - groups.length;
    const deletionCount = groups.reduce(
      (total, group) => total + group.members.length - 1,
      0,
    );
    const confirmed = await openDialog({
      title: "确认按推荐清理多版本",
      message: `将按推荐保留 ${groups.length} 组中的一个版本，并永久删除另外 ${deletionCount} 个文件。标签、静态分类和阅读进度会迁移到保留版本。${skipped ? `另有 ${skipped} 组没有可靠推荐，将跳过。` : ""}`,
      type: "danger",
      confirmText: "按推荐删除其他版本",
    });
    if (!confirmed) return;

    versionBatchBusy.value = true;
    try {
      let deleted = 0;
      let failedFiles = 0;
      let failedGroups = 0;
      for (const group of groups) {
        const recommendedArchiveId = group.recommendedArchiveId;
        if (!recommendedArchiveId) continue;
        try {
          const result = await cleanupVersions(
            group.id,
            recommendedArchiveId,
            group.members
              .filter((member) => member.archive.id !== recommendedArchiveId)
              .map((member) => member.archive.id),
          );
          deleted += result.deleted;
          failedFiles += result.failedArchiveIds.length;
        } catch (error) {
          failedGroups += 1;
          console.error("按推荐清理多版本失败:", group.id, error);
        }
      }
      selectedVersionGroupIds.value = new Set();
      await Promise.all([
        refetchVersionGroups(),
        refetchCollections(),
        refetchSelectedCollection(),
        refetch(),
      ]);
      const issues = [
        skipped ? `${skipped} 组无可靠推荐已跳过` : "",
        failedFiles ? `${failedFiles} 个文件未能删除` : "",
        failedGroups ? `${failedGroups} 组处理失败` : "",
      ].filter(Boolean);
      await showInfoDialog(
        `已按推荐处理 ${groups.length - failedGroups} 组，删除 ${deleted} 个版本。${issues.length ? ` ${issues.join("；")}。` : ""}`,
        "多版本批量清理",
      );
    } finally {
      versionBatchBusy.value = false;
    }
    return;
  }

  versionBatchBusy.value = true;
  try {
    if (action === "restore") {
      await Promise.all(ids.map((id) => restoreVersionGroup(id)));
    } else {
      await Promise.all(ids.map((id) => keepAllVersions(id)));
    }
    selectedVersionGroupIds.value = new Set();
    await refetchVersionGroups();
  } catch (error) {
    console.error("批量处理多版本失败:", error);
    await showInfoDialog(
      action === "restore"
        ? "部分版本组未能恢复待处理，请刷新后重试。"
        : "部分版本组未能标记为无需处理，请刷新后重试。",
      "操作失败",
    );
  } finally {
    versionBatchBusy.value = false;
  }
};

const handleKeepAllVersions = async (id: string) => {
  try {
    await keepAllVersions(id);
    selectedVersionGroup.value = null;
    await refetchVersionGroups();
  } catch (error) {
    console.error("保留多版本失败:", error);
    await showInfoDialog("无法记录此版本组，请稍后重试。", "操作失败");
  }
};

const handleRestoreVersionGroup = async (id: string) => {
  try {
    await restoreVersionGroup(id);
    selectedVersionGroup.value = null;
    await refetchVersionGroups();
  } catch (error) {
    console.error("恢复多版本待处理失败:", error);
    await showInfoDialog("无法恢复此版本组，请稍后重试。", "操作失败");
  }
};

const handleRebuildCollections = async () => {
  if (collectionsRebuilding.value) return;
  collectionsRebuilding.value = true;
  try {
    const result = await rebuildCollections();
    showCollectionPreview.value = false;
    await Promise.all([
      refetchCollections(),
      refetchCollectionReviews(),
      refetchSelectedCollection(),
      refetchVersionGroups(),
    ]);
    await showInfoDialog(
      `已分析 ${result.parsedArchives} 本漫画，创建或更新 ${result.createdCollections} 个合集，加入 ${result.groupedArchives} 本成员。${result.pendingReviews ? `另有 ${result.pendingReviews} 条待确认。` : ""}`,
      "合集识别完成",
    );
  } catch (error) {
    console.error("合集识别失败:", error);
    await showInfoDialog("无法完成合集识别，请稍后重试。", "操作失败");
  } finally {
    collectionsRebuilding.value = false;
  }
};

const handlePreviewCollections = async () => {
  if (collectionsPreviewLoading.value) return;
  collectionsPreviewLoading.value = true;
  try {
    collectionPreview.value = await previewCollectionRebuild();
    showCollectionPreview.value = true;
  } catch (error) {
    console.error("合集识别预览失败:", error);
    await showInfoDialog("无法生成合集识别预览，请稍后重试。", "操作失败");
  } finally {
    collectionsPreviewLoading.value = false;
  }
};

const handleCollectionReviewChanged = async () => {
  await Promise.all([
    refetchCollections(),
    refetchCollectionReviews(),
    refetchSelectedCollection(),
  ]);
};

const handleRemoveCollectionMember = async (archiveId: string) => {
  const confirmed = await openDialog({
    title: "移出合集",
    message: "移出后会记录为“不是同一合集”，后续自动识别不会再次加入。",
    type: "warning",
    confirmText: "移出",
  });
  if (!confirmed) return;
  try {
    await removeCollectionMember(archiveId);
    await Promise.all([
      refetchCollections(),
      refetchCollectionReviews(),
      refetchSelectedCollection(),
    ]);
  } catch (error) {
    console.error("移出合集失败:", error);
    await showInfoDialog("无法移出该漫画，请稍后重试。", "操作失败");
  }
};

const handleAdvancedFilters = (filters: Partial<SearchParams>) => {
  const { sortBy, sortOrder, ...memberFilters } = filters;
  const nextSortBy = sortBy || viewSorts.value[libraryViewMode.value].sortBy;
  viewSorts.value[libraryViewMode.value] = {
    sortBy: nextSortBy,
    sortOrder:
      nextSortBy === "recognitionPriority"
        ? "asc"
        : sortOrder || viewSorts.value[libraryViewMode.value].sortOrder,
  };
  advancedFilters.value = memberFilters;
  currentPage.value = 1;
  collectionPage.value = 1;
};

const handleResetFilters = () => {
  advancedFilters.value = {};
  const defaults = {
    single: { sortBy: "createdAt", sortOrder: "asc" },
    collections: { sortBy: "recognitionPriority", sortOrder: "asc" },
    versions: { sortBy: "recognitionPriority", sortOrder: "asc" },
  };
  viewSorts.value[libraryViewMode.value] = defaults[libraryViewMode.value];
  currentPage.value = 1;
  collectionPage.value = 1;
};

const handleMobileApply = (payload: {
  query: string;
  filters: Partial<SearchParams>;
}) => {
  searchQuery.value = payload.query;
  handleAdvancedFilters(payload.filters);
  libraryStore.showMobileSearch = false;
};

const handleClearSearch = () => {
  searchQuery.value = "";
  currentPage.value = 1;
};

const removeActiveTag = (tag: string) => {
  const tags = (advancedFilters.value.tags || []).filter((t) => t !== tag);
  advancedFilters.value = {
    ...advancedFilters.value,
    tags: tags.length > 0 ? tags : undefined,
  };
  currentPage.value = 1;
};

const removePageFilter = () => {
  advancedFilters.value = {
    ...advancedFilters.value,
    minPages: undefined,
    maxPages: undefined,
  };
  currentPage.value = 1;
};

const removeFileSizeFilter = () => {
  advancedFilters.value = {
    ...advancedFilters.value,
    minFileSize: undefined,
    maxFileSize: undefined,
  };
  currentPage.value = 1;
};

const removeDateFilter = () => {
  advancedFilters.value = {
    ...advancedFilters.value,
    createdAfter: undefined,
    createdBefore: undefined,
  };
  currentPage.value = 1;
};

const handleMobileClearAll = () => {
  searchQuery.value = "";
  handleResetFilters();
};

const openCreateCategoryModal = () => {
  createCategoryInitialType.value = "static";
  createCategoryInitialSearchParams.value = {};
  showCreateCategoryModal.value = true;
};

const handleSaveCurrentSearchAsDynamicCategory = () => {
  createCategoryInitialType.value = "dynamic";
  createCategoryInitialSearchParams.value = { ...currentSearchSnapshot.value };
  showCreateCategoryModal.value = true;
};

const closeCreateCategoryModal = () => {
  showCreateCategoryModal.value = false;
  createCategoryInitialType.value = "static";
  createCategoryInitialSearchParams.value = {};
};

const handleSelectCategory = async (categoryId: string | null) => {
  libraryStore.selectCategory(categoryId);
  currentPage.value = 1;
  await refetch();
};

const handleEditCategory = (category: Category) => {
  selectedCategory.value = category;
  showEditCategoryModal.value = true;
};

const openReader = (archiveId: string, collectionId?: string, recommendationSessionId?: string, recommendationPosition?: number) => {
  saveViewSnapshot();
  router.push({
    name: "reader",
    params: { id: archiveId },
    query: {
      ...(collectionId ? { collection: collectionId } : {}),
      ...(recommendationSessionId ? { recommendationSessionId } : {}),
      ...(recommendationPosition != null ? { recommendationPosition: String(recommendationPosition) } : {}),
    },
  });
};

const openReaderInNewTab = (archiveId: string) => {
  const routeLocation = router.resolve({
    name: "reader",
    params: { id: archiveId },
  });
  window.open(routeLocation.href, "_blank", "noopener,noreferrer");
};

const goToPage = async (page: number) => {
  if (page >= 1 && page <= totalPages.value) {
    currentPage.value = page;
    await refetch();
    window.scrollTo({ top: 0, behavior: "smooth" });
  }
};
const goToCollectionPage = (page: number) => {
  if (page < 1 || page > collectionTotalPages.value) return;
  collectionPage.value = page;
  window.scrollTo({ top: 0, behavior: "smooth" });
};

// 分类模态框
const handleCategoryCreated = () => {
  closeCreateCategoryModal();
  queryClient.invalidateQueries({ queryKey: ["categories"] });
  queryClient.invalidateQueries({ queryKey: ["allArchivesCount"] });
  refetch();
};

const handleCategoryUpdated = () => {
  showEditCategoryModal.value = false;
  selectedCategory.value = null;
  queryClient.invalidateQueries({ queryKey: ["categories"] });
  queryClient.invalidateQueries({ queryKey: ["allArchivesCount"] });
  refetch();
};

// 右键菜单
const handleArchiveContextMenu = (event: MouseEvent, archive: Archive) => {
  contextMenuArchive.value = archive;
  contextMenuPosition.value = { x: event.clientX, y: event.clientY };
  showContextMenu.value = true;
};

const closeContextMenu = () => {
  showContextMenu.value = false;
  contextMenuArchive.value = null;
};

const handleOpenReaderInNewTabFromContext = () => {
  if (!contextMenuArchive.value) return;
  const archiveId = contextMenuArchive.value.id;
  closeContextMenu();
  openReaderInNewTab(archiveId);
};

const handleEditMetadataFromContext = () => {
  if (!contextMenuArchive.value) return;
  const archiveId = contextMenuArchive.value.id;
  closeContextMenu();
  saveViewSnapshot();
  router.push({
    name: "reader",
    params: { id: archiveId },
    query: { panel: "info" },
  });
};

const handleRetryTitleTranslationFromContext = async () => {
  if (!contextMenuArchive.value) return;
  const archive = contextMenuArchive.value;
  closeContextMenu();

  try {
    const result = await retryArchiveTitleTranslation(archive.id);
    await showInfoDialog(
      result.queued
        ? `《${archive.title}》已重新加入标题翻译队列。旧译文会保留到新译文完成。`
        : `《${archive.title}》已经在标题翻译队列中。`,
      "标题翻译",
    );
  } catch (error) {
    console.error("重新翻译标题失败:", error);
    await showInfoDialog(
      "无法创建标题翻译任务，请检查 AI 设置后重试。",
      "操作失败",
    );
  }
};

const handleShowCollectionsFromContext = () => {
  const title = contextMenuArchive.value?.title || "";
  closeContextMenu();
  searchQuery.value = title;
  setLibraryViewMode("collections");
};

// 标签操作
const handleAddTagFromContext = () => {
  if (!contextMenuArchive.value) return;
  tagModalArchive.value = contextMenuArchive.value;
  showTagModal.value = true;
  closeContextMenu();
};

const handleAddTagToArchive = async (
  archiveId: string,
  tagName: string,
  namespace: string,
) => {
  try {
    const tag = await createTag(tagName, namespace);
    await addTagToArchive(archiveId, tag.id);
    refetch();
  } catch (err) {
    console.error("Failed to add tag:", err);
    await showInfoDialog("添加标签失败，请稍后重试", "操作失败");
  }
};

const handleTagModalSubmit = async (tagName: string, namespace: string) => {
  if (!tagModalArchive.value) return;
  await handleAddTagToArchive(tagModalArchive.value.id, tagName, namespace);
  showTagModal.value = false;
  tagModalArchive.value = null;
};

const closeTagModal = () => {
  showTagModal.value = false;
  tagModalArchive.value = null;
};

// 分类操作
const handleAddToCategoryFromContext = (categoryId: string) => {
  if (!contextMenuArchive.value) return;
  handleAddArchiveToCategory(contextMenuArchive.value.id, categoryId);
  closeContextMenu();
};

const handleRemoveFromCategoryFromContext = (categoryId: string) => {
  if (!contextMenuArchive.value) return;
  handleRemoveArchiveFromCategory(contextMenuArchive.value.id, categoryId);
  closeContextMenu();
};

const handleAddArchiveToCategory = async (
  archiveId: string,
  categoryId: string,
) => {
  try {
    await addArchivesToCategory(categoryId, { archiveIds: [archiveId] });
    queryClient.invalidateQueries({ queryKey: ["categories"] });
    refetch();
  } catch (err) {
    console.error("Failed to add to category:", err);
    await showInfoDialog("添加到分类失败，请稍后重试", "操作失败");
  }
};

const handleRemoveArchiveFromCategory = async (
  archiveId: string,
  categoryId: string,
) => {
  try {
    await removeArchivesFromCategory(categoryId, { archiveIds: [archiveId] });
    queryClient.invalidateQueries({ queryKey: ["categories"] });
    refetch();
  } catch (err) {
    console.error("Failed to remove from category:", err);
    await showInfoDialog("移出分类失败，请稍后重试", "操作失败");
  }
};

// 删除档案
const handleDeleteArchiveFromContext = async () => {
  if (!contextMenuArchive.value) return;
  const archive = contextMenuArchive.value;
  closeContextMenu();

  const confirmed = await openDialog({
    title: "确认删除漫画",
    message: `确定要删除漫画《${archive.title}》吗？此操作不可撤销。`,
    type: "danger",
    confirmText: "删除",
  });

  if (!confirmed) return;
  await handleDeleteArchive(archive.id);
};

const handleDeleteArchive = async (archiveId: string) => {
  try {
    await deleteArchive(archiveId);
    removeArchiveFromRandomCache(archiveId);
    queryClient.invalidateQueries({ queryKey: ["categories"] });
    queryClient.invalidateQueries({ queryKey: ["allArchivesCount"] });
    refetch();
  } catch (err) {
    console.error("Failed to delete archive:", err);
    await showInfoDialog("删除失败，请稍后重试", "操作失败");
  }
};

const removeArchiveFromRandomCache = (archiveId: string) => {
  queryClient.setQueriesData<Archive[]>(
    { queryKey: ["randomArchives"] },
    (cachedArchives) => {
      if (!cachedArchives) return cachedArchives;
      return cachedArchives.filter((archive) => archive.id !== archiveId);
    },
  );
};

const updateArchiveInCurrentPage = (updatedArchive: Archive) => {
  queryClient.setQueryData<PaginatedResponse<Archive> | undefined>(
    queryKey.value,
    (cachedData) => {
      if (!cachedData) return cachedData;
      const targetIndex = cachedData.data.findIndex(
        (archive) => archive.id === updatedArchive.id,
      );
      if (targetIndex < 0) return cachedData;

      const nextArchives = [...cachedData.data];
      nextArchives[targetIndex] = updatedArchive;

      return {
        ...cachedData,
        data: nextArchives,
      };
    },
  );
};

const refreshSingleArchiveFromReader = async (archiveId: string) => {
  try {
    const [latestArchive, latestProgress] = await Promise.all([
      getArchive(archiveId),
      getProgress(archiveId).catch(() => null),
    ]);

    updateArchiveInCurrentPage(latestArchive);

    const nextProgressMap = new Map(progressData.value);
    if (latestProgress) {
      nextProgressMap.set(archiveId, latestProgress);
    } else {
      nextProgressMap.delete(archiveId);
    }
    progressData.value = nextProgressMap;

    if (contextMenuArchive.value?.id === archiveId) {
      contextMenuArchive.value = latestArchive;
    }
  } catch (error) {
    console.error("Failed to refresh returning archive:", error);
  }
};

const consumeReturningArchiveId = (): string | null => {
  try {
    const archiveId = sessionStorage.getItem(LIBRARY_RETURN_ARCHIVE_KEY);
    if (!archiveId) return null;
    sessionStorage.removeItem(LIBRARY_RETURN_ARCHIVE_KEY);
    return archiveId;
  } catch (error) {
    console.error("Failed to consume returning archive id:", error);
    return null;
  }
};

let listenersBound = false;
const bindGlobalListeners = () => {
  if (listenersBound) return;
  document.addEventListener("click", closeContextMenu);
  window.addEventListener("resize", onResize);
  listenersBound = true;
};

const unbindGlobalListeners = () => {
  if (!listenersBound) return;
  document.removeEventListener("click", closeContextMenu);
  window.removeEventListener("resize", onResize);
  listenersBound = false;
};

onMounted(() => {
  bindGlobalListeners();
  if (route.name === "library") {
    queryClient.invalidateQueries({ queryKey: ["batchProgress"] });
  }
  const returningArchiveId = consumeReturningArchiveId();
  if (returningArchiveId) {
    void refreshSingleArchiveFromReader(returningArchiveId);
  }
  restoreViewScroll();
});

onActivated(() => {
  bindGlobalListeners();
  const returningArchiveId = consumeReturningArchiveId();
  if (returningArchiveId) {
    void refreshSingleArchiveFromReader(returningArchiveId);
  }
  restoreViewScroll();
});

onDeactivated(() => {
  saveViewSnapshot(window.scrollY);
  unbindGlobalListeners();
});

onUnmounted(() => {
  saveViewSnapshot(window.scrollY);
  unbindGlobalListeners();
  clearTimeout(resizeTimer);
});
</script>

<style scoped>
.library-view {
  /* Library 页面根容器 */
}
</style>
