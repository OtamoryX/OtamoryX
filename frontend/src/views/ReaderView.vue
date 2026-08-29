<template>
  <div :class="['reader-view fixed inset-0 z-50 bg-black flex flex-col', props.class]">
    <!-- 顶部信息栏 -->
    <transition
      enter-active-class="transition-all duration-300 ease-out"
      enter-from-class="opacity-0 -translate-y-full"
      enter-to-class="opacity-100 translate-y-0"
      leave-active-class="transition-all duration-300 ease-in"
      leave-from-class="opacity-100 translate-y-0"
      leave-to-class="opacity-0 -translate-y-full"
    >
      <div
        v-if="showToolbar"
        class="fixed top-0 left-0 right-0 bg-linear-to-b from-black/80 via-black/60 to-transparent text-[var(--text-primary)] px-3 pb-3 pt-[calc(env(safe-area-inset-top,0px)+0.75rem)] sm:px-6 sm:py-4 z-[60]"
        @click.stop
      >
        <div class="flex items-center justify-between max-w-6xl mx-auto">
          <div class="flex-1 min-w-0">
            <h1 class="text-sm font-semibold leading-5 truncate sm:text-lg sm:leading-6">
              {{ displayTitle || "加载中..." }}
            </h1>
            <p
              v-if="displaySubtitle"
              class="h-4 mt-0.5 text-xs leading-4 text-[var(--text-tertiary)] truncate"
              :title="displaySubtitle"
            >
              {{ displaySubtitle }}
            </p>
            <div v-else class="h-4 mt-0.5" aria-hidden="true" />
            <div class="mt-1 flex flex-wrap items-center gap-x-4 gap-y-0.5 text-sm leading-5 text-[var(--text-secondary)]">
              <span>第 {{ currentPage }} 页 / 共 {{ totalPages }} 页</span>
              <span v-if="totalPages > 0">进度:
                {{ ((currentPage / totalPages) * 100).toFixed(1) }}%</span>
              <span class="hidden sm:inline">{{
                readingMode === "single" ? "单页模式" : "双页模式"
              }}</span>
              <span class="hidden sm:inline">{{ getDisplayModeLabel() }}</span>
            </div>
          </div>
          <div v-if="collectionDetail && collectionDetail.members.length > 1" class="ml-3 flex shrink-0 items-center gap-1.5 rounded-lg border border-white/15 bg-black/25 p-1">
            <button
              class="collection-navigation-button inline-flex h-9 items-center gap-1.5 rounded-md px-2 text-xs font-medium text-white transition-colors hover:bg-white/15 disabled:cursor-not-allowed disabled:opacity-40"
              :disabled="!previousCollectionMember || isCollectionSwitching"
              :title="previousCollectionMember ? `上一册：${previousCollectionMember.archive.title}` : '已经是合集第一册'"
              :aria-label="previousCollectionMember ? `上一册：${previousCollectionMember.archive.title}` : '已经是合集第一册'"
              @click="previousCollectionMember && switchCollectionMember(previousCollectionMember.archive.id, 'previous')"
            >
              <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="m15 18-6-6 6-6" /></svg>
              <svg class="h-4 w-4 text-[var(--accent)]" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20M6.5 2H20v15H6.5A2.5 2.5 0 0 0 4 19.5V4.5A2.5 2.5 0 0 1 6.5 2Z" /></svg>
              <span class="hidden sm:inline">上一册</span>
            </button>
            <span class="min-w-9 text-center text-xs tabular-nums text-[var(--text-secondary)]">{{ collectionMemberIndex + 1 }} / {{ collectionDetail.members.length }}</span>
            <button
              class="collection-navigation-button inline-flex h-9 items-center gap-1.5 rounded-md px-2 text-xs font-medium text-white transition-colors hover:bg-white/15 disabled:cursor-not-allowed disabled:opacity-40"
              :disabled="!nextCollectionMember || isCollectionSwitching"
              :title="nextCollectionMember ? `下一册：${nextCollectionMember.archive.title}` : '已经是合集最后一册'"
              :aria-label="nextCollectionMember ? `下一册：${nextCollectionMember.archive.title}` : '已经是合集最后一册'"
              @click="nextCollectionMember && switchCollectionMember(nextCollectionMember.archive.id, 'next')"
            >
              <span class="hidden sm:inline">下一册</span>
              <svg class="h-4 w-4 text-[var(--accent)]" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20M6.5 2H20v15H6.5A2.5 2.5 0 0 0 4 19.5V4.5A2.5 2.5 0 0 1 6.5 2Z" /></svg>
              <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="m9 18 6-6-6-6" /></svg>
            </button>
          </div>
          <button
            class="toolbar-button p-2 hover:bg-[var(--bg-tertiary)] rounded-lg transition-colors ml-4"
            title="返回书库"
            @click="goBack"
          >
            <svg
              class="w-5 h-5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        </div>
      </div>
    </transition>

    <!-- 主要阅读区域 -->
    <div
      class="reader-content flex-1 relative overflow-hidden"
      @touchstart="handleTouchStart"
      @touchmove="handleTouchMove"
      @touchend="handleTouchEnd"
    >
      <!-- 点击区域 -->
      <div class="absolute inset-0">
        <!-- 左侧点击区域 -->
        <div
          class="absolute left-0 top-0 h-full cursor-pointer z-10"
          :style="{ width: `calc(50% - 60px)` }"
          @click="handleLeftClick"
          @mouseenter="showLeftHint"
          @mouseleave="hideNavHint"
        >
          <!-- 导航提示 -->
          <div
            v-if="
              (pageDirection === 'ltr' && navHint === 'prev') ||
                (pageDirection === 'rtl' && navHint === 'next')
            "
            class="absolute left-4 top-1/2 transform -translate-y-1/2 bg-black bg-opacity-60 text-white px-3 py-2 rounded-lg text-sm"
          >
            {{ pageDirection === "ltr" ? "上一页" : "下一页" }}
          </div>
        </div>

        <!-- 中间正方形点击区域（切换工具栏显示）-->
        <div
          class="absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 w-24 h-24 cursor-pointer z-20 flex items-center justify-center"
          @click="handleCenterClick"
          @mouseenter="showNavHint('toolbar')"
          @mouseleave="hideNavHint"
        >
          <!-- 可见的提示区域 -->
          <div
            v-if="navHint === 'toolbar'"
            class="absolute inset-0 bg-black/20 backdrop-blur-sm border-2 border-white/30 rounded-lg flex items-center justify-center"
          >
            <div class="text-white text-xs font-medium">{{ showToolbar ? '隐藏' : '显示' }}工具栏</div>
          </div>
        </div>

        <!-- 右侧点击区域 -->
        <div
          class="absolute right-0 top-0 h-full cursor-pointer z-10"
          :style="{ width: `calc(50% - 60px)` }"
          @click="handleRightClick"
          @mouseenter="showRightHint"
          @mouseleave="hideNavHint"
        >
          <!-- 导航提示 -->
          <div
            v-if="
              (pageDirection === 'ltr' && navHint === 'next') ||
                (pageDirection === 'rtl' && navHint === 'prev')
            "
            class="absolute right-4 top-1/2 transform -translate-y-1/2 bg-black bg-opacity-60 text-white px-3 py-2 rounded-lg text-sm"
          >
            {{ pageDirection === "ltr" ? "下一页" : "上一页" }}
          </div>
        </div>
      </div>

      <!-- 漫画图片 -->
      <div
        class="absolute inset-0 flex justify-center items-center p-2 overflow-hidden"
      >
        <div
          v-if="error"
          class="text-red-400 text-xl"
        >
          加载失败: {{ error }}
        </div>
      </div>

      <!-- 加载占位符 -->
      <LoadingPlaceholder v-if="showLoadingPlaceholder" />

      <transition name="book-switch-notice">
        <div v-if="collectionSwitchNotice" class="pointer-events-none absolute inset-0 z-40 flex items-center justify-center bg-black/35 px-6 backdrop-blur-[1px]">
          <div class="flex max-w-md items-center gap-3 rounded-lg border border-white/20 bg-black/70 px-4 py-3 text-white shadow-2xl">
            <svg class="h-7 w-7 shrink-0 text-[var(--accent)]" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20M6.5 2H20v15H6.5A2.5 2.5 0 0 0 4 19.5V4.5A2.5 2.5 0 0 1 6.5 2Z" /></svg>
            <div class="min-w-0">
              <p class="text-xs text-white/65">{{ collectionSwitchNotice.direction === 'next' ? '正在打开下一册' : '正在打开上一册' }}</p>
              <p class="mt-0.5 truncate text-sm font-medium">{{ collectionSwitchNotice.title }}</p>
            </div>
          </div>
        </div>
      </transition>

      <!-- 图片内容（有切换动画） -->
      <transition
        v-if="!showLoadingPlaceholder"
        :name="contentTransitionName"
        mode="out-in"
        @before-enter="handlePageTransitionStart"
        @after-enter="handlePageTransitionEnd"
      >
        <div
          :key="`content-${archiveId}-${pageTransitionKey}`"
          class="w-full h-full flex justify-center items-center"
        >
          <!-- 单页模式 -->
          <img
            v-if="readingMode === 'single' && currentPageUrl"
            :src="currentPageUrl"
            :alt="`第 ${currentPage} 页`"
            :class="getImageClasses()"
            :style="getImageStyles()"
            @load="handleImageLoad"
            @error="handleImageError"
          >

          <!-- 双页模式 -->
          <div
            v-else-if="readingMode === 'double' && currentPageUrl"
            class="flex justify-center items-center gap-2 w-full h-full"
          >
            <!-- 从左到右阅读方向 -->
            <template v-if="pageDirection === 'ltr'">
              <!-- 左页 (当前页) -->
              <img
                :src="currentPageUrl"
                :alt="`第 ${currentPage} 页`"
                :class="getDoublePageImageClasses()"
                :style="getDoublePageImageStyles()"
                @load="handleImageLoad"
                @error="handleImageError"
              >
              <!-- 右页 (下一页) -->
              <img
                v-if="currentPage < totalPages && nextPageUrl"
                :src="nextPageUrl"
                :alt="`第 ${currentPage + 1} 页`"
                :class="getDoublePageImageClasses()"
                :style="getDoublePageImageStyles()"
                @load="handleImageLoad"
                @error="handleImageError"
              >
            </template>

            <!-- 从右到左阅读方向 (漫画风格) -->
            <template v-else>
              <!-- 左页 (下一页) -->
              <img
                v-if="currentPage < totalPages && nextPageUrl"
                :src="nextPageUrl"
                :alt="`第 ${currentPage + 1} 页`"
                :class="getDoublePageImageClasses()"
                :style="getDoublePageImageStyles()"
                @load="handleImageLoad"
                @error="handleImageError"
              >
              <!-- 右页 (当前页) -->
              <img
                :src="currentPageUrl"
                :alt="`第 ${currentPage} 页`"
                :class="getDoublePageImageClasses()"
                :style="getDoublePageImageStyles()"
                @load="handleImageLoad"
                @error="handleImageError"
              >
            </template>
          </div>
        </div>
      </transition>
    </div>
  </div>

  <!-- 侧边信息面板 -->
  <ReaderInfoPanel
    class="z-[90]"
    :show="showInfoPanel"
    :archive-info="archiveInfo"
    :current-page="currentPage"
    :total-pages="totalPages"
    :display-mode-label="getDisplayModeLabel()"
    :reading-mode-label="getReadingModeLabel()"
    :plugin-options="readerPluginOptions"
    :plugins-loading="pluginsLoading"
    :plugin-executing="executePluginMutation.isPending.value"
    :plugin-execution-summary="lastPluginExecutionSummary"
    :translation-retrying="titleTranslationRetrying"
    :translation-retry-message="titleTranslationRetryMessage"
    :ehentai-candidates="ehentaiCandidates"
    :ehentai-searching="ehentaiSearching"
    :ehentai-search-error="ehentaiSearchError"
    :nhentai-candidates="nhentaiCandidates"
    :nhentai-searching="nhentaiSearching"
    :nhentai-search-error="nhentaiSearchError"
    @close="hideInfoPanel"
    @add-tag="handleAddTag"
    @remove-tag="handleRemoveTag"
    @switch-display-mode="switchImageDisplayMode"
    @switch-reading-mode="switchReadingMode"
    @delete-archive="handleDeleteArchive"
    @execute-plugin="handleExecutePlugin"
    @retry-title-translation="handleRetryTitleTranslation"
    @search-ehentai="handleSearchEhentai"
    @search-nhentai="handleSearchNhentai"
  />

  <!-- 始终显示的毛玻璃风格进度条 -->
  <div
    :class="[
      'fixed left-1/2 transform -translate-x-1/2 z-[70] transition-all duration-300',
      showToolbar ? 'bottom-[82px] sm:bottom-[88px]' : 'bottom-4 sm:bottom-6',
    ]"
    style="width: clamp(220px, 72vw, 450px)"
  >
    <div
      :class="[
        'border rounded-2xl px-3 sm:px-6 py-2 sm:py-3 shadow-2xl transition-all duration-300',
        showToolbar 
          ? 'bg-black/60 backdrop-blur-md border-[var(--border)]'
          : 'bg-black/10 border-white/10'
      ]"
      @mouseenter="handleProgressHoverStart"
      @mouseleave="handleProgressHoverEnd"
    >
      <!-- 进度条容器 -->
      <div class="relative">
        <input
          v-model.number="progressValue"
          type="range"
          :min="1"
          :max="totalPages"
          :step="1"
          class="w-full h-1.5 sm:h-2 appearance-none cursor-pointer slider-glass"
          @input="handleProgressChange"
          @mousedown="handleProgressDragStart"
          @mouseup="handleProgressDragEnd"
          @mousemove="handleProgressHover"
          @touchstart="isDraggingProgress = true"
          @touchend="handleProgressDragEnd"
        >

        <!-- 进度条预览工具提示 -->
        <transition
          enter-active-class="transition-all duration-200 ease-out"
          enter-from-class="opacity-0 scale-90 translate-y-2"
          enter-to-class="opacity-100 scale-100 translate-y-0"
          leave-active-class="transition-all duration-200 ease-in"
          leave-from-class="opacity-100 scale-100 translate-y-0"
          leave-to-class="opacity-0 scale-90 translate-y-2"
        >
          <div
            v-if="showProgressPreview && progressPreviewPage"
            :style="{ left: `${progressPreviewPosition}px` }"
            class="absolute -top-32 transform -translate-x-1/2 z-[80] pointer-events-none"
          >
            <div
              class="bg-black/90 backdrop-blur-md border border-[var(--border)] rounded-lg p-3 shadow-2xl"
            >
              <div class="flex flex-col items-center space-y-3">
                <!-- 缩略图容器 - 增大尺寸 -->
                <div class="w-24 h-32 bg-[var(--bg-secondary)] rounded overflow-hidden">
                  <img
                    v-if="progressPreviewImage"
                    :src="progressPreviewImage"
                    :alt="`第 ${progressPreviewPage} 页`"
                    class="w-full h-full object-cover"
                    @error="handlePreviewImageError"
                  >
                  <div
                    v-else
                    class="w-full h-full flex items-center justify-center text-[var(--text-tertiary)] text-xs"
                  >
                    第{{ progressPreviewPage }}页
                  </div>
                </div>
                <!-- 页码信息 -->
                <div class="text-xs text-white text-center">
                  <div>第 {{ progressPreviewPage }} 页</div>
                  <div class="text-[var(--text-secondary)]">
                    {{ ((progressPreviewPage / totalPages) * 100).toFixed(1) }}%
                  </div>
                </div>
              </div>
            </div>
          </div>
        </transition>
      </div>
    </div>
  </div>

  <!-- 底部现代化工具栏 -->
  <transition
    enter-active-class="transition-all duration-300 ease-out"
    enter-from-class="opacity-0 translate-y-full"
    enter-to-class="opacity-100 translate-y-0"
    leave-active-class="transition-all duration-300 ease-in"
    leave-from-class="opacity-100 translate-y-0"
    leave-to-class="opacity-0 translate-y-full"
  >
    <div
      v-if="showToolbar"
      class="fixed bottom-0 left-0 right-0 bg-black/80 backdrop-blur-md border-t border-white/10 text-[var(--text-primary)] px-2 sm:px-6 pt-2 sm:pt-4 pb-[calc(env(safe-area-inset-bottom,0px)+0.5rem)] sm:pb-4 z-[60]"
      @click.stop
    >
      <div class="flex items-center max-w-6xl mx-auto w-full gap-2 sm:gap-4">
        <!-- 左侧控制按钮 -->
        <div class="flex items-center gap-1 sm:gap-3 shrink-0">
          <!-- 翻页按钮 - 根据翻页方向调整顺序 -->
          <template v-if="pageDirection === 'ltr'">
            <!-- 从左到右：上一页在左，下一页在右 -->
            <button
              :disabled="currentPage <= 1"
              class="toolbar-button flex h-11 w-11 items-center justify-center md:h-auto md:w-auto md:p-2 hover:bg-[var(--bg-tertiary)] disabled:opacity-50 disabled:cursor-not-allowed rounded-lg transition-colors shrink-0"
              title="上一页 (←)"
              @click="prevPage"
            >
              <svg
                class="w-4 h-4 sm:w-5 sm:h-5"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M15 19l-7-7 7-7"
                />
              </svg>
            </button>
            <button
              :disabled="currentPage >= totalPages"
              class="toolbar-button flex h-11 w-11 items-center justify-center md:h-auto md:w-auto md:p-2 hover:bg-[var(--bg-tertiary)] disabled:opacity-50 disabled:cursor-not-allowed rounded-lg transition-colors shrink-0"
              title="下一页 (→)"
              @click="nextPage"
            >
              <svg
                class="w-4 h-4 sm:w-5 sm:h-5"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M9 5l7 7-7 7"
                />
              </svg>
            </button>
          </template>

          <template v-else>
            <!-- 从右到左：下一页在左，上一页在右 -->
            <button
              :disabled="currentPage >= totalPages"
              class="toolbar-button flex h-11 w-11 items-center justify-center md:h-auto md:w-auto md:p-2 hover:bg-[var(--bg-tertiary)] disabled:opacity-50 disabled:cursor-not-allowed rounded-lg transition-colors shrink-0"
              title="下一页 (←)"
              @click="nextPage"
            >
              <svg
                class="w-4 h-4 sm:w-5 sm:h-5"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M15 19l-7-7 7-7"
                />
              </svg>
            </button>
            <button
              :disabled="currentPage <= 1"
              class="toolbar-button flex h-11 w-11 items-center justify-center md:h-auto md:w-auto md:p-2 hover:bg-[var(--bg-tertiary)] disabled:opacity-50 disabled:cursor-not-allowed rounded-lg transition-colors shrink-0"
              title="上一页 (→)"
              @click="prevPage"
            >
              <svg
                class="w-4 h-4 sm:w-5 sm:h-5"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M9 5l7 7-7 7"
                />
              </svg>
            </button>
          </template>
        </div>

        <!-- 中间页面信息 -->
        <div class="flex-1 min-w-0 mx-1 sm:mx-8 text-center">
          <div class="text-xs sm:text-sm text-[var(--text-secondary)] truncate tabular-nums">
            <span class="sm:hidden">{{ currentPage }}/{{ totalPages }}</span>
            <span class="hidden sm:inline">{{ currentPage }} / {{ totalPages }}</span>
            <span> · {{ ((currentPage / totalPages) * 100).toFixed(1) }}%</span>
          </div>
        </div>

        <!-- 右侧工具按钮 -->
        <button class="toolbar-button flex h-11 w-11 items-center justify-center rounded-lg transition-colors md:hidden" title="更多阅读操作" @click="showMobileReaderActions = true">
          <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6.75h.01M12 12h.01M12 17.25h.01" /></svg>
        </button>
        <div class="reader-toolbar-actions hidden md:flex items-center gap-1 sm:gap-3 shrink-0 max-w-[52vw] sm:max-w-none overflow-x-auto sm:overflow-visible">
          <button
            class="toolbar-button p-1.5 sm:p-2 hover:bg-[var(--bg-tertiary)] rounded-lg transition-colors shrink-0"
            :title="`显示模式: ${getDisplayModeLabel()} (V)`"
            @click="switchImageDisplayMode"
          >
            <svg
              class="w-4 h-4 sm:w-5 sm:h-5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"
              />
            </svg>
          </button>
          <button
            class="toolbar-button p-1.5 sm:p-2 hover:bg-[var(--bg-tertiary)] rounded-lg transition-colors shrink-0"
            :title="`阅读模式: ${getReadingModeLabel()} (D)`"
            @click="switchReadingMode"
          >
            <svg
              v-if="readingMode === 'single'"
              class="w-4 h-4 sm:w-5 sm:h-5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
              />
            </svg>
            <svg
              v-else
              class="w-4 h-4 sm:w-5 sm:h-5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.746 0 3.332.477 4.5 1.253v13C19.832 18.477 18.246 18 16.5 18c-1.746 0-3.332.477-4.5 1.253"
              />
            </svg>
          </button>
          <button
            class="toolbar-button p-1.5 sm:p-2 hover:bg-[var(--bg-tertiary)] rounded-lg transition-colors shrink-0"
            title="显示详情 (Space)"
            @click="showInfoPanelWithAutoHide"
          >
            <svg
              class="w-4 h-4 sm:w-5 sm:h-5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
          </button>
          <button
            class="toolbar-button p-1.5 sm:p-2 hover:bg-[var(--bg-tertiary)] rounded-lg transition-colors shrink-0"
            title="设置"
            @click="handleSettingsToggle"
          >
            <svg
              class="w-4 h-4 sm:w-5 sm:h-5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
              />
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
              />
            </svg>
          </button>
        </div>
      </div>
    </div>
  </transition>

  <!-- 阅读设置面板 -->
  <ReaderSettingsPanel
    class="z-[90]"
    :show="showSettings"
    :image-display-mode="imageDisplayMode"
    :reading-mode="readingMode"
    :page-direction="pageDirection"
    :page-animation-enabled="pageAnimationEnabled"
    :is-fullscreen="isFullscreen"
    :auto-hide-u-i="autoHideUI"
    :show-page-numbers="showPageNumbers"
    @close="handleSettingsClose"
    @set-display-mode="handleSetDisplayMode"
    @set-reading-mode="handleSetReadingMode"
    @switch-page-direction="switchPageDirection"
    @toggle-animation="handleToggleAnimation"
    @toggle-fullscreen="toggleFullscreen"
    @toggle-auto-hide="handleToggleAutoHide"
    @toggle-page-numbers="handleTogglePageNumbers"
  />

  <div v-if="showMobileReaderActions" class="fixed inset-0 z-[95] md:hidden" @click.self="showMobileReaderActions = false">
    <div class="absolute inset-x-2 bottom-[calc(env(safe-area-inset-bottom,0px)+0.5rem)] overflow-hidden rounded-xl border border-white/15 bg-black/95 shadow-2xl">
      <div class="px-4 py-3 text-xs font-medium text-[var(--text-secondary)]">阅读操作</div>
      <button class="flex h-12 w-full items-center px-4 text-left text-sm text-white hover:bg-white/10" @click="showMobileReaderActions = false; showInfoPanelWithAutoHide()">详细信息</button>
      <button class="flex h-12 w-full items-center border-t border-white/10 px-4 text-left text-sm text-white hover:bg-white/10" @click="showMobileReaderActions = false; handleSettingsToggle()">阅读设置</button>
      <button class="flex h-12 w-full items-center border-t border-white/10 px-4 text-left text-sm text-[var(--text-secondary)] hover:bg-white/10" @click="showMobileReaderActions = false">取消</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from "vue";
import { useRoute, useRouter, onBeforeRouteLeave } from "vue-router";
import { useQuery, useMutation, useQueryClient } from "@tanstack/vue-query";
import {
  getArchive,
  getCollection,
  getProgress,
  recordBehaviorEvent,
  updateProgress,
  removeTagFromArchive,
  getArchivePage,
  createTag,
  addTagToArchive,
  deleteArchive,
  getPlugins,
  retryArchiveTitleTranslation,
  searchEhentaiCandidates,
  searchNhentaiCandidates,
} from "@/utils/api";
import type { Archive, CollectionDetail, EhentaiCandidate, NhentaiCandidate, Tag, ReadingProgress, Plugin } from "@/types/api";
import { useTitleDisplayStore } from "@/stores/titleDisplay";
import { archiveDisplaySubtitle, archiveDisplayTitle } from "@/utils/archiveTitle";
import LoadingPlaceholder from "@/components/LoadingPlaceholder.vue";
import ReaderInfoPanel from "@/components/reader/ReaderInfoPanel.vue";
import ReaderSettingsPanel from "@/components/reader/ReaderSettingsPanel.vue";

// Define props to accept class and other attributes
interface Props {
  class?: string;
}

interface ReaderPluginOption {
  id: string;
  name: string;
  enabled: boolean;
}

interface ReaderPluginExecutionSummary {
  status: "success" | "failure";
  message: string;
}

interface ExecutePluginPayload {
  pluginId: string;
  oneshotParam?: string;
}

interface ExecutePluginResponse {
  accepted?: number;
  failed?: number;
  results?: Array<{ error?: string | null }>;
  message?: string;
}

const props = withDefaults(defineProps<Props>(), {
  class: '',
});

const route = useRoute();
const router = useRouter();
const queryClient = useQueryClient();
const titleDisplayStore = useTitleDisplayStore();
const LIBRARY_RETURN_ARCHIVE_KEY = "library-return-archive-id";

const archiveId = computed(() => route.params.id as string);
const collectionId = computed(() => typeof route.query.collection === "string" ? route.query.collection : null);
const recommendationSessionId = computed(() => typeof route.query.recommendationSessionId === "string" ? route.query.recommendationSessionId : null);
const recommendationPosition = computed(() => typeof route.query.recommendationPosition === "string" ? Number(route.query.recommendationPosition) : null);
const currentPage = ref(1);
const totalPages = ref(1);
const isLoading = ref(false);
const isCollectionSwitching = ref(false);
const collectionSwitchNotice = ref<{
  archiveId: string;
  direction: "previous" | "next";
  title: string;
} | null>(null);
const isPageTransitionLoading = ref(false); // 区分主动翻页加载和预加载
const showLoadingPlaceholder = ref(false); // 控制是否显示占位符
const error = ref<string | null>(null);
const currentPageUrl = ref<string | null>(null);
const nextPageUrl = ref<string | null>(null);
type TimeoutHandle = ReturnType<typeof setTimeout>;
type ImageDisplayMode = "fit" | "fill" | "original";
type ReadingMode = "single" | "double";
type PreloadPriority = "high" | "medium" | "low";

// 信息面板相关状态
const showInfoPanel = ref(false);
const titleTranslationRetrying = ref(false);
const titleTranslationRetryMessage = ref<string | null>(null);
const navHint = ref<string | null>(null);
const autoHideTimeout = ref<TimeoutHandle | null>(null);
const shouldOpenInfoPanelFromQuery = computed(() => {
  const panel = route.query.panel;
  if (Array.isArray(panel)) return panel.includes("info");
  return panel === "info";
});

// 图片显示模式
const imageDisplayMode = ref<ImageDisplayMode>("fit");

// 阅读模式
const readingMode = ref<ReadingMode>("single");
// 翻页方向 ('ltr' = 从左到右, 'rtl' = 从右到左)
const pageDirection = ref<"ltr" | "rtl">("ltr");

// 进度条相关状态（工具栏内）
const progressValue = ref(1);
const isDraggingProgress = ref(false);

// 组件事件处理函数会在后面定义

// 工具栏状态
const showToolbar = ref(false);
const toolbarTimer = ref<TimeoutHandle | null>(null);
const showSettings = ref(false);
const showMobileReaderActions = ref(false);
const isHoveringProgress = ref(false);

// 翻页动画相关状态
const pageTransitionName = ref("slide-left");
const pageTransitionKey = ref(0);
const isTransitioning = ref(false);
const bookSwitchTransitionName = ref<"book-switch-forward" | "book-switch-backward" | null>(null);
const collectionSwitchContentReady = ref(false);
const pageLoadRequestId = ref(0);

const contentTransitionName = computed(() =>
  bookSwitchTransitionName.value ?? (pageAnimationEnabled.value ? pageTransitionName.value : "fade"),
);

// 设置面板相关状态
const pageAnimationEnabled = ref(true);
const isFullscreen = ref(false);
const autoHideUI = ref(true);
const showPageNumbers = ref(true);

// 移动端触摸交互状态
const touchStartX = ref(0);
const touchStartY = ref(0);
const touchStartTime = ref(0);
const isSwiping = ref(false);
const isTouchInteracting = ref(false); // 标记是否正在进行触摸交互

// 进度条预览相关状态
const showProgressPreview = ref(false);
const progressPreviewPage = ref(0);
const progressPreviewPosition = ref(0);
const progressPreviewImage = ref<string | null>(null);
const previewImageCache = ref<Map<number, string>>(new Map());
const previewLoadTimer = ref<TimeoutHandle | null>(null);

// 窗口尺寸响应
const windowSize = ref({
  width: window.innerWidth,
  height: window.innerHeight,
});

const resolvePluginId = (plugin: Plugin): string => {
  const pluginWithCompatKey = plugin as Plugin & { plugin_id?: string };
  return pluginWithCompatKey.id || pluginWithCompatKey.plugin_id || "";
};

// 获取插件列表（Reader 详情 one-shot 入口使用）
const pluginsQuery = useQuery({
  queryKey: ["plugins"],
  queryFn: () => getPlugins(),
  enabled: computed(() => !!archiveId.value),
});

const pluginsLoading = computed(() => {
  return pluginsQuery.isLoading.value || (pluginsQuery.isFetching.value && !pluginsQuery.data.value);
});

const readerPluginOptions = computed<ReaderPluginOption[]>(() => {
  const plugins = pluginsQuery.data.value ?? [];
  return plugins
    .map((plugin) => ({
      id: resolvePluginId(plugin),
      name: plugin.name,
      enabled: plugin.enabled,
    }))
    .filter((plugin) => plugin.enabled && !!plugin.id);
});

const lastPluginExecutionSummary = ref<ReaderPluginExecutionSummary | null>(null);
const ehentaiCandidates = ref<EhentaiCandidate[]>([]);
const ehentaiSearching = ref(false);
const ehentaiSearchError = ref<string | null>(null);
const nhentaiCandidates = ref<NhentaiCandidate[]>([]);
const nhentaiSearching = ref(false);
const nhentaiSearchError = ref<string | null>(null);

const executePluginMutation = useMutation({
  mutationFn: async (payload: ExecutePluginPayload): Promise<ExecutePluginResponse> => {
    if (!archiveId.value) {
      throw new Error("缺少档案 ID，无法执行插件");
    }

    const headers: HeadersInit = {
      "Content-Type": "application/json",
    };

    const apiKey = localStorage.getItem("apiKey");
    if (apiKey) {
      headers.Authorization = `Bearer ${apiKey}`;
    }

    const trimmedOneshotParam = payload.oneshotParam?.trim();
    const requestBody = trimmedOneshotParam
      ? { oneshot_param: trimmedOneshotParam }
      : {};

    const response = await fetch(
      `/api/v1/plugins/${encodeURIComponent(payload.pluginId)}/execute/${encodeURIComponent(archiveId.value)}`,
      {
        method: "POST",
        headers,
        body: JSON.stringify(requestBody),
      },
    );

    const responseData = await response
      .json()
      .catch(() => ({ message: `执行失败（HTTP ${response.status}）` }));

    if (!response.ok) {
      const message = typeof responseData?.message === "string"
        ? responseData.message
        : `执行失败（HTTP ${response.status}）`;
      throw new Error(message);
    }

    return responseData as ExecutePluginResponse;
  },
  onSuccess: (data, variables) => {
    const pluginName = readerPluginOptions.value.find((plugin) => plugin.id === variables.pluginId)?.name || variables.pluginId;
    const accepted = data.accepted ?? 0;
    const failed = data.failed ?? 0;
    const firstError = data.results?.find((result) => !!result.error)?.error || "";

    if (accepted > 0 && failed === 0) {
      queryClient.invalidateQueries({ queryKey: ["archive", archiveId.value] });
      if (variables.pluginId === "ehentai-metadata") {
        ehentaiCandidates.value = [];
      }
      if (variables.pluginId === "nhentai-metadata") {
        nhentaiCandidates.value = [];
      }
      lastPluginExecutionSummary.value = {
        status: "success",
        message: `${pluginName} 执行已提交（${accepted} 个任务）`,
      };
      return;
    }

    const fallbackMessage = accepted > 0
      ? `${pluginName} 部分执行失败：已提交 ${accepted}，失败 ${failed}`
      : `${pluginName} 执行失败`;

    lastPluginExecutionSummary.value = {
      status: "failure",
      message: firstError || data.message || fallbackMessage,
    };
  },
  onError: (error, variables) => {
    const pluginName = readerPluginOptions.value.find((plugin) => plugin.id === variables.pluginId)?.name || variables.pluginId;
    lastPluginExecutionSummary.value = {
      status: "failure",
      message: `${pluginName}：${error instanceof Error ? error.message : "执行失败"}`,
    };
  },
});

watch(
  shouldOpenInfoPanelFromQuery,
  (open) => {
    if (!open) return;
    showInfoPanel.value = true;
    showToolbar.value = true;
  },
  { immediate: true },
);

// 获取漫画信息
const { data: archiveInfo, isLoading: isArchiveLoading } = useQuery({
  queryKey: computed(() => ["archive", archiveId.value]),
  queryFn: () => getArchive(archiveId.value),
  enabled: computed(() => !!archiveId.value),
});
const displayTitle = computed(() =>
  archiveDisplayTitle(archiveInfo.value, titleDisplayStore.displayTranslatedTitle),
);
const displaySubtitle = computed(() =>
  archiveDisplaySubtitle(archiveInfo.value, titleDisplayStore.displayTranslatedTitle),
);

const { data: collectionDetail } = useQuery<CollectionDetail>({
  queryKey: computed(() => ["collection", collectionId.value]),
  queryFn: () => getCollection(collectionId.value!),
  enabled: computed(() => !!collectionId.value),
  retry: false,
});
const collectionMemberIndex = computed(() => collectionDetail.value?.members.findIndex(member => member.archive.id === archiveId.value) ?? -1);
const previousCollectionMember = computed(() => {
  const index = collectionMemberIndex.value
  return index > 0 ? collectionDetail.value?.members[index - 1] : undefined
});
const nextCollectionMember = computed(() => {
  const index = collectionMemberIndex.value
  return index >= 0 ? collectionDetail.value?.members[index + 1] : undefined
});

// 获取阅读进度
const { data: progressData, isLoading: isProgressLoading } = useQuery({
  queryKey: computed(() => ["progress", archiveId.value]),
  queryFn: () => getProgress(archiveId.value),
  enabled: computed(() => !!archiveId.value),
  retry: false, // 如果没有进度记录，不重试
});

// 更新进度的mutation
const updateProgressMutation = useMutation({
  mutationFn: ({
    archiveId,
    currentPage,
    readerSessionId,
    recommendationSessionId,
  }: {
    archiveId: string;
    currentPage: number;
    readerSessionId?: string | null;
    recommendationSessionId?: string | null;
  }) => updateProgress(archiveId, { currentPage, readerSessionId: readerSessionId || undefined, recommendationSessionId: recommendationSessionId || undefined }),
  onSuccess: () => {
    // 刷新进度数据
    queryClient.invalidateQueries({ queryKey: ["progress", archiveId.value] });
    // 刷新漫画信息（后端可能自动移除了"new"标签）
    queryClient.invalidateQueries({ queryKey: ["archive", archiveId.value] });
  },
  onError: (error) => {
    console.error("Failed to update progress:", error);
  },
});

// 监听漫画信息变化，更新总页数
watch(
  archiveInfo,
  (newInfo) => {
    console.log("archiveInfo watch triggered:", {
      newInfo: newInfo
        ? { id: newInfo.id, title: newInfo.title, pageCount: newInfo.pageCount }
        : null,
      totalPagesBefore: totalPages.value,
      currentPage: currentPage.value,
    });
    if (newInfo && newInfo.id === archiveId.value) {
      totalPages.value = newInfo.pageCount;
      console.log("Updated totalPages to:", newInfo.pageCount);
    }
  },
  { immediate: true },
);

// 加载当前页面图片
const loadCurrentPage = async (isUserNavigation = false) => {
  const requestedArchiveId = archiveId.value;
  const requestedPage = currentPage.value;
  const requestId = ++pageLoadRequestId.value;
  const isCurrentRequest = () =>
    requestId === pageLoadRequestId.value &&
    requestedArchiveId === archiveId.value &&
    requestedPage === currentPage.value;

  console.log("loadCurrentPage called:", {
    archiveId: requestedArchiveId,
    currentPage: requestedPage,
    totalPages: totalPages.value,
    readingMode: readingMode.value,
    isUserNavigation,
  });

  if (!requestedArchiveId) {
    console.log("No archiveId, returning");
    return;
  }

  try {
    // 检查是否已经预加载了这个页面
    const isPreloaded = preloadedUrls.value.has(requestedPage);

    // 只在没有预加载且网络较慢时显示加载占位符
    if (isUserNavigation && !isPreloaded) {
      isPageTransitionLoading.value = true;
      // 设置一个短暂的延迟，如果页面快速加载完成就不显示占位符
      setTimeout(() => {
        if (isCurrentRequest() && isPageTransitionLoading.value) {
          showLoadingPlaceholder.value = true;
          currentPageUrl.value = null;
          nextPageUrl.value = null;
        }
      }, 200); // 200ms后如果还在加载才显示占位符
    } else if (isUserNavigation) {
      isPageTransitionLoading.value = true;
      // 预加载的页面，直接开始动画转换
      showLoadingPlaceholder.value = false;
    }

    isLoading.value = true;
    error.value = null;

    // 加载当前页 - 优先使用预加载的URL
    console.log(
      "Calling getArchivePage with:",
      requestedArchiveId,
      requestedPage,
    );
    let pageUrl = preloadedUrls.value.get(requestedPage);
    let createdPageUrl = false;
    if (!pageUrl) {
      pageUrl = await getArchivePage(requestedArchiveId, requestedPage);
      createdPageUrl = true;
    } else {
      console.log("Using preloaded URL for page:", currentPage.value);
    }
    console.log("Got page URL:", pageUrl);

    // 双页模式下加载下一页 - 优先使用预加载的URL
    let nextPageUrlResult: string | null = null;
    let createdNextPageUrl = false;
    if (
      readingMode.value === "double" &&
      requestedPage < totalPages.value
    ) {
      try {
        console.log(
          "Loading next page for double mode:",
          requestedPage + 1,
        );
        nextPageUrlResult =
          preloadedUrls.value.get(requestedPage + 1) ?? null;
        if (!nextPageUrlResult) {
          nextPageUrlResult = await getArchivePage(
            requestedArchiveId,
            requestedPage + 1,
          );
          createdNextPageUrl = true;
        } else {
          console.log(
            "Using preloaded URL for next page:",
            currentPage.value + 1,
          );
        }
        console.log("Got next page URL:", nextPageUrlResult);
      } catch (nextErr: any) {
        console.warn("Failed to load next page:", nextErr);
      }
    }

    if (!isCurrentRequest()) {
      if (createdPageUrl) URL.revokeObjectURL(pageUrl);
      if (createdNextPageUrl && nextPageUrlResult) URL.revokeObjectURL(nextPageUrlResult);
      return;
    }

    // 所有URL都准备好后，一次性设置，确保显示的是当前页面的内容
    currentPageUrl.value = pageUrl;
    nextPageUrl.value = nextPageUrlResult;
    if (collectionSwitchNotice.value?.archiveId === requestedArchiveId) {
      collectionSwitchContentReady.value = true;
      pageTransitionKey.value++;
    }
    isLoading.value = false;
    isPageTransitionLoading.value = false;
    showLoadingPlaceholder.value = false;
  } catch (err: any) {
    if (!isCurrentRequest()) return;
    console.error("Failed to load page:", err);
    error.value = err.response?.data?.message || err.message || "加载页面失败";
    currentPageUrl.value = null;
    nextPageUrl.value = null;
    isLoading.value = false;
    isPageTransitionLoading.value = false;
    showLoadingPlaceholder.value = false;
  }
};

const restoredProgressArchiveId = ref<string | null>(null);

// 监听进度数据变化，恢复阅读位置
watch(
  progressData,
  (newProgress) => {
    console.log("progressData watch triggered:", {
      newProgress,
      currentPageBefore: currentPage.value,
      archiveId: archiveId.value,
    });

    // 查询切换时可能短暂保留上一册的数据，只接受当前档案的进度。
    if (newProgress?.archiveId && newProgress.archiveId !== archiveId.value) return;

    if (newProgress && restoredProgressArchiveId.value !== archiveId.value) {
      currentPage.value = newProgress.currentPage > 0 ? newProgress.currentPage : 1;
      restoredProgressArchiveId.value = archiveId.value;
      console.log("Set currentPage from initial progress:", currentPage.value);
    } else if (!newProgress && currentPage.value <= 0) {
      currentPage.value = 1;
      console.log("No progress data, setting currentPage to 1");
    }
  },
  { immediate: true },
);

// 预加载相关
const preloadedPages = ref<Set<number>>(new Set());
const preloadedUrls = ref<Map<number, string>>(new Map());

const clearArchivePageResources = () => {
  pageLoadRequestId.value++;
  const urls = new Set<string>();
  if (currentPageUrl.value) urls.add(currentPageUrl.value);
  if (nextPageUrl.value) urls.add(nextPageUrl.value);
  preloadedUrls.value.forEach(url => urls.add(url));
  urls.forEach(url => URL.revokeObjectURL(url));

  currentPageUrl.value = null;
  nextPageUrl.value = null;
  preloadedPages.value.clear();
  preloadedUrls.value.clear();
  loadedImages.value.clear();
  showLoadingPlaceholder.value = false;
  isPageTransitionLoading.value = false;
};

// 用于跟踪双页模式下的加载状态
const loadedImages = ref<Set<number>>(new Set());

// 智能预加载函数
const preloadPages = async () => {
  if (!archiveId.value || isLoading.value) {
    console.log("Skipping preload: no archiveId or currently loading");
    return;
  }

  console.log("Starting preload process for current page:", currentPage.value);
  const pagesToPreload: Array<{ page: number; priority: PreloadPriority }> = [];

  // 预加载下一页/下两页（优先级高）
  if (readingMode.value === "single") {
    if (currentPage.value + 1 <= totalPages.value) {
      pagesToPreload.push({ page: currentPage.value + 1, priority: "high" });
    }
    if (currentPage.value + 2 <= totalPages.value) {
      pagesToPreload.push({ page: currentPage.value + 2, priority: "medium" });
    }
  } else {
    // 双页模式下预加载下一对页面
    if (currentPage.value + 2 <= totalPages.value) {
      pagesToPreload.push({ page: currentPage.value + 2, priority: "high" });
      if (currentPage.value + 3 <= totalPages.value) {
        pagesToPreload.push({ page: currentPage.value + 3, priority: "high" });
      }
    }
  }

  // 预加载上一页（用于后退，优先级低）
  if (readingMode.value === "single") {
    if (currentPage.value - 1 >= 1) {
      pagesToPreload.push({ page: currentPage.value - 1, priority: "low" });
    }
  } else {
    if (currentPage.value - 2 >= 1) {
      pagesToPreload.push({ page: currentPage.value - 2, priority: "low" });
      if (currentPage.value - 1 >= 1) {
        pagesToPreload.push({ page: currentPage.value - 1, priority: "low" });
      }
    }
  }

  // 按优先级排序，高优先级先预加载
  const sortedPages = pagesToPreload.sort((a, b) => {
    const priorityOrder: Record<PreloadPriority, number> = {
      high: 0,
      medium: 1,
      low: 2,
    };
    return priorityOrder[a.priority] - priorityOrder[b.priority];
  });

  // 顺序执行预加载，避免并发请求过多
  for (const { page: pageNum, priority } of sortedPages) {
    if (!preloadedPages.value.has(pageNum)) {
      preloadedPages.value.add(pageNum);
      try {
        console.log(
          `Starting ${priority} priority preload for page ${pageNum}`,
        );
        const pageUrl = await getArchivePage(archiveId.value, pageNum);
        preloadedUrls.value.set(pageNum, pageUrl);
        console.log(`Successfully preloaded page ${pageNum}`);

        // 创建Image对象强制浏览器缓存
        const img = new Image();
        img.src = pageUrl;

        // 高优先级页面预加载后稍作延迟，让浏览器有时间处理
        if (priority === "high") {
          await new Promise((resolve) => setTimeout(resolve, 50));
        }
      } catch (error) {
        console.warn(`Failed to preload page ${pageNum}:`, error);
        preloadedPages.value.delete(pageNum);
      }
    } else {
      console.log(`Page ${pageNum} already preloaded, skipping`);
    }
  }

  console.log("Preload process completed");
};

// 监听当前页码变化，自动加载页面
watch(
  currentPage,
  (newPage, oldPage) => {
    console.log("currentPage watch triggered:", {
      newPage,
      oldPage,
      archiveId: archiveId.value,
      totalPages: totalPages.value,
      isDragging: isDraggingProgress.value,
    });

    if (currentPage.value > 0) {
      // 如果正在拖拽进度条，不要加载页面，只更新显示
      if (isDraggingProgress.value) {
        console.log("Skipping page load during progress drag");
        return;
      }

      // 清理之前的加载状态
      loadedImages.value.clear();

      // 判断是否为用户导航（不是初始加载或进度恢复）
      const isUserNavigation =
        oldPage !== undefined && oldPage !== newPage && !isLoading.value;
      loadCurrentPage(isUserNavigation);
      // 预加载会在当前页加载完成后触发（通过handleImageLoad）
    } else {
      console.log("currentPage <= 0, not loading page");
    }
  },
  { immediate: true },
); // 添加immediate: true确保初始值也会触发

// 监听阅读模式变化，清理预加载缓存
watch(readingMode, () => {
  // 清理预加载缓存
  preloadedPages.value.clear();
  // 清理URL缓存（释放内存）
  preloadedUrls.value.forEach((url) => URL.revokeObjectURL(url));
  preloadedUrls.value.clear();
  // 清理加载状态
  loadedImages.value.clear();
  // 重新加载当前页面（处理双页模式），预加载会在加载完成后触发
  loadCurrentPage(true);
});

// 监听当前页变化，同步进度条值
watch(
  currentPage,
  (newPage) => {
    progressValue.value = newPage;
  },
  { immediate: true },
);

// 导航方法
const goBack = () => {
  if (window.history.state?.back) {
    router.back();
    return;
  }
  router.replace("/library");
};

const switchCollectionMember = async (
  targetArchiveId: string,
  direction: "previous" | "next",
) => {
  if (!collectionId.value || isCollectionSwitching.value || targetArchiveId === archiveId.value) return;
  const target = collectionDetail.value?.members.find(member => member.archive.id === targetArchiveId);
  collectionSwitchNotice.value = {
    archiveId: targetArchiveId,
    direction,
    title: target?.archive.title || "加载中...",
  };
  collectionSwitchContentReady.value = false;
  bookSwitchTransitionName.value = direction === "next" ? "book-switch-forward" : "book-switch-backward";
  isCollectionSwitching.value = true;
  try {
    await flushProgressBeforeLeave();
    await router.replace({
      name: "reader",
      params: { id: targetArchiveId },
      query: { ...route.query, collection: collectionId.value },
    });
  } finally {
    isCollectionSwitching.value = false;
  }
};

const prevPage = () => {
  if (isTransitioning.value) return; // 防止动画期间重复触发

  const decrement = readingMode.value === "double" ? 2 : 1;
  if (currentPage.value > 1) {
    // 根据翻页方向设置动画方向
    // LTR: 上一页向右滑动，RTL: 上一页向左滑动
    pageTransitionName.value =
      pageDirection.value === "ltr" ? "slide-right" : "slide-left";
    if (pageAnimationEnabled.value) {
      triggerPageTransition();
    }

    currentPage.value = Math.max(currentPage.value - decrement, 1);
    hideInfoPanel();
    // 自动保存进度
    saveProgress();
  }
};

const nextPage = () => {
  if (isTransitioning.value) return; // 防止动画期间重复触发

  const increment = readingMode.value === "double" ? 2 : 1;
  if (currentPage.value < totalPages.value) {
    // 根据翻页方向设置动画方向
    // LTR: 下一页向左滑动，RTL: 下一页向右滑动
    pageTransitionName.value =
      pageDirection.value === "ltr" ? "slide-left" : "slide-right";
    if (pageAnimationEnabled.value) {
      triggerPageTransition();
    }

    currentPage.value = Math.min(
      currentPage.value + increment,
      totalPages.value,
    );
    hideInfoPanel();
    // 自动保存进度
    saveProgress();
  }
};

// 防抖进度保存
const saveProgressTimer = ref<TimeoutHandle | null>(null);
const pendingProgressPage = ref<number | null>(null);
const leaveProgressFlushed = ref(false);
const readerSessionKey = ref<string | null>(null);
const readerSessionStartedAt = ref<number | null>(null);

const emitBehaviorEvent = (eventType: string, payload: Record<string, unknown> = {}) => {
  const archive = archiveId.value;
  if (!archive) return;
  const sessionId = readerSessionKey.value;
  const eventKey = payload.eventKey as string | undefined;
  void recordBehaviorEvent({
    archiveId: archive,
    eventType,
    eventKey: eventKey || (sessionId ? `${sessionId}:${eventType}:${Date.now()}` : undefined),
    page: typeof payload.page === "number" ? payload.page : undefined,
    metadata: {
      ...payload,
      ...(recommendationSessionId.value ? { source: "random" } : {}),
      ...(sessionId ? { readerSessionId: sessionId } : {}),
      ...(recommendationSessionId.value ? { recommendationSessionId: recommendationSessionId.value } : {}),
      ...(recommendationPosition.value != null ? { recommendationPosition: recommendationPosition.value } : {}),
    },
  }).catch((error) => {
    console.debug("Behavior event was not recorded", error);
  });
};

// 保存阅读进度（带防抖）
const saveProgress = () => {
  if (!archiveId.value || currentPage.value <= 0) return;

  // 记录待保存的页码
  pendingProgressPage.value = currentPage.value;

  // 清除之前的定时器
  if (saveProgressTimer.value) {
    clearTimeout(saveProgressTimer.value);
  }

  // 设置新的防抖定时器
  saveProgressTimer.value = setTimeout(() => {
    const pageToSave = pendingProgressPage.value;
    if (pageToSave && archiveId.value) {
      console.log(`Saving progress: page ${pageToSave}`);
      updateProgressMutation.mutate({
        archiveId: archiveId.value,
        currentPage: pageToSave,
        readerSessionId: readerSessionKey.value,
        recommendationSessionId: recommendationSessionId.value,
      });
    }
    saveProgressTimer.value = null;
    pendingProgressPage.value = null;
  }, 500); // 0.5秒防抖延迟
};

const flushProgressBeforeLeave = async () => {
  if (leaveProgressFlushed.value) return;
  if (!archiveId.value || currentPage.value <= 0) return;

  if (saveProgressTimer.value) {
    clearTimeout(saveProgressTimer.value);
    saveProgressTimer.value = null;
  }

  const finalPage = pendingProgressPage.value || currentPage.value;
  if (finalPage <= 0) return;
  pendingProgressPage.value = null;

  try {
    await updateProgress(archiveId.value, { currentPage: finalPage, recommendationSessionId: recommendationSessionId.value || undefined });
    leaveProgressFlushed.value = true;
    queryClient.invalidateQueries({ queryKey: ["progress", archiveId.value] });
    queryClient.invalidateQueries({ queryKey: ["archive", archiveId.value] });
  } catch (error) {
    console.error("Failed to flush progress before leaving:", error);
  }
};

onBeforeRouteLeave(async (to) => {
  if (to.name === "library" && archiveId.value) {
    sessionStorage.setItem(LIBRARY_RETURN_ARCHIVE_KEY, archiveId.value);
  }
  await flushProgressBeforeLeave();
  emitBehaviorEvent("exit", {
    eventKey: readerSessionKey.value ? `${readerSessionKey.value}:exit` : undefined,
    startPage: Math.max(1, Number(progressData.value?.currentPage || 1)),
    endPage: currentPage.value,
    totalPages: totalPages.value,
    durationMs: readerSessionStartedAt.value ? Math.max(0, Date.now() - readerSessionStartedAt.value) : undefined,
    page: currentPage.value,
    source: "reader",
  });
});

// 注意：移除"new"标签的逻辑已移到后端Progress处理器中统一处理
// 避免前端重复调用造成竞态条件和404错误

// 翻页动画控制函数
const triggerPageTransition = () => {
  if (pageAnimationEnabled.value) {
    pageTransitionKey.value++;
  }
};

const handlePageTransitionStart = () => {
  isTransitioning.value = true;
};

const handlePageTransitionEnd = () => {
  isTransitioning.value = false;
  if (
    bookSwitchTransitionName.value &&
    collectionSwitchContentReady.value &&
    collectionSwitchNotice.value?.archiveId === archiveId.value
  ) {
    bookSwitchTransitionName.value = null;
    collectionSwitchNotice.value = null;
    collectionSwitchContentReady.value = false;
  }
};

// 全屏控制
const toggleFullscreen = async () => {
  try {
    if (!isFullscreen.value) {
      // 进入全屏
      const element = document.documentElement;
      if (element.requestFullscreen) {
        await element.requestFullscreen();
      } else if (element.webkitRequestFullscreen) {
        await element.webkitRequestFullscreen();
      } else if (element.msRequestFullscreen) {
        await element.msRequestFullscreen();
      }
    } else {
      // 退出全屏
      if (document.exitFullscreen) {
        await document.exitFullscreen();
      } else if (document.webkitExitFullscreen) {
        await document.webkitExitFullscreen();
      } else if (document.msExitFullscreen) {
        await document.msExitFullscreen();
      }
    }
  } catch (error) {
    console.error("Fullscreen toggle failed:", error);
  }
};

// 监听全屏状态变化
const handleFullscreenChange = () => {
  isFullscreen.value = !!(
    document.fullscreenElement ||
    document.webkitFullscreenElement ||
    document.msFullscreenElement
  );
};

// 中间区域点击处理
const handleCenterClick = (event: MouseEvent) => {
  if (isTouchInteracting.value) {
    event.preventDefault();
    return;
  }
  toggleToolbar();
};

// 工具栏切换控制
const toggleToolbar = () => {
  showToolbar.value ? hideToolbar() : showToolbarWithAutoHide();
};

// 信息面板控制
const toggleInfoPanel = () => {
  showInfoPanel.value ? hideInfoPanel() : showInfoPanelWithAutoHide();
};

const showInfoPanelWithAutoHide = () => {
  showInfoPanel.value = true;
};

const hideInfoPanel = () => {
  showInfoPanel.value = false;
  clearAutoHideTimer();
};

const handleRetryTitleTranslation = async () => {
  if (!archiveId.value || titleTranslationRetrying.value) return;

  titleTranslationRetrying.value = true;
  titleTranslationRetryMessage.value = null;
  try {
    const result = await retryArchiveTitleTranslation(archiveId.value);
    titleTranslationRetryMessage.value = result.queued
      ? "已加入翻译队列，旧译文会保留到新译文完成。"
      : "该标题已在翻译队列中。";
    await queryClient.invalidateQueries({ queryKey: ["ai-status"] });
  } catch (error) {
    console.error("重新翻译标题失败:", error);
    titleTranslationRetryMessage.value = "无法创建翻译任务，请检查 AI 设置后重试。";
  } finally {
    titleTranslationRetrying.value = false;
  }
};

const setAutoHideTimer = () => {
  clearAutoHideTimer();
  autoHideTimeout.value = setTimeout(() => {
    hideInfoPanel();
  }, 5000); // 5秒后自动隐藏
};

const clearAutoHideTimer = () => {
  if (autoHideTimeout.value) {
    clearTimeout(autoHideTimeout.value);
    autoHideTimeout.value = null;
  }
};

// 点击区域处理
const handleLeftClick = () => {
  // 触摸点击已经在 touchend 中处理过，忽略紧随其后的合成 click。
  if (isTouchInteracting.value) {
    return;
  }

  if (pageDirection.value === "ltr") {
    prevPage();
  } else {
    nextPage();
  }
  // 翻页时不改变工具栏状态
};

const handleRightClick = () => {
  // 触摸点击已经在 touchend 中处理过，忽略紧随其后的合成 click。
  if (isTouchInteracting.value) {
    return;
  }

  if (pageDirection.value === "ltr") {
    nextPage();
  } else {
    prevPage();
  }
  // 翻页时不改变工具栏状态
};

// 导航提示
const showLeftHint = () => {
  navHint.value = pageDirection.value === "ltr" ? "prev" : "next";
};

const showRightHint = () => {
  navHint.value = pageDirection.value === "ltr" ? "next" : "prev";
};

const showNavHint = (type: string) => {
  navHint.value = type;
};

const hideNavHint = () => {
  navHint.value = null;
};

// 进度条控制（工具栏内）
// 进度条拖拽开始
const handleProgressDragStart = () => {
  isDraggingProgress.value = true;
  // 拖拽开始时清除自动隐藏定时器
  clearToolbarTimer();
};

const handleProgressChange = () => {
  // 拖拽时实时更新当前页码显示，但不保存进度
  if (progressValue.value !== currentPage.value) {
    if (isDraggingProgress.value) {
      // 拖拽时只更新显示，不保存进度
      currentPage.value = progressValue.value;
    } else {
      // 非拖拽时的正常更新（比如程序化设置）
      currentPage.value = progressValue.value;
      saveProgress();
    }
  }
};

const handleProgressDragEnd = () => {
  const wasProgress = isDraggingProgress.value;
  isDraggingProgress.value = false;

  // 隐藏预览
  hideProgressPreview();

  if (progressValue.value !== currentPage.value) {
    // 设置跳转动画（根据跳转方向和翻页设置）
    const isForward = progressValue.value > currentPage.value;
    if (pageDirection.value === "ltr") {
      pageTransitionName.value = isForward ? "slide-left" : "slide-right";
    } else {
      pageTransitionName.value = isForward ? "slide-right" : "slide-left";
    }
    if (pageAnimationEnabled.value) {
      triggerPageTransition();
    }

    // 更新当前页码（这会触发页面加载，因为isDraggingProgress现在是false）
    currentPage.value = progressValue.value;
    saveProgress(); // 这里会使用防抖逻辑
  } else if (wasProgress) {
    // 如果页码没变但刚结束拖拽，仍需要加载页面（因为拖拽期间跳过了加载）
    loadedImages.value.clear();
    loadCurrentPage(true);
  }

  // 拖拽结束后，如果鼠标不在进度条上，设置自动隐藏定时器
  if (!isHoveringProgress.value && showToolbar.value) {
    setToolbarTimer();
  }
};

// 工具栏控制
const showToolbarWithAutoHide = () => {
  showToolbar.value = true;
  setToolbarTimer();
};

const hideToolbar = () => {
  showToolbar.value = false;
  clearToolbarTimer();
};

const setToolbarTimer = () => {
  clearToolbarTimer();
  if (!autoHideUI.value) return;
  
  toolbarTimer.value = setTimeout(() => {
    if (!showSettings.value && !isHoveringProgress.value && !isDraggingProgress.value) {
      hideToolbar();
    }
  }, 3000);
};

const clearToolbarTimer = () => {
  if (toolbarTimer.value) {
    clearTimeout(toolbarTimer.value);
    toolbarTimer.value = null;
  }
};

const showToolbarOnMouseMove = (event: MouseEvent) => {
  if (showToolbar.value || showInfoPanel.value) return;
  
  const { clientY } = event;
  const threshold = 50;
  
  if (clientY <= threshold || clientY >= window.innerHeight - threshold) {
    showToolbarWithAutoHide();
  }
};

// 进度条悬停处理
const handleProgressHoverStart = () => {
  isHoveringProgress.value = true;
  if (!showToolbar.value) showToolbar.value = true;
  clearToolbarTimer();
};

const handleProgressHoverEnd = () => {
  isHoveringProgress.value = false;
  if (!isDraggingProgress.value && showToolbar.value) {
    setToolbarTimer();
  }
  hideProgressPreview();
};

// 进度条预览功能
const handleProgressHover = (event: MouseEvent) => {
  if (!isDraggingProgress.value) return;

  const progressBar = event.currentTarget as HTMLInputElement;
  const rect = progressBar.getBoundingClientRect();
  const x = event.clientX - rect.left;
  // 直接使用 v-model 的 progressValue，保证预览与释放后的跳转使用同一变量。
  const previewPage = Math.max(
    1,
    Math.min(totalPages.value, progressValue.value),
  );

  showProgressPreview.value = true;
  progressPreviewPage.value = previewPage;
  progressPreviewPosition.value = x;

  // 加载预览图片
  loadPreviewImage(previewPage);
};

const loadPreviewImage = async (pageNum: number) => {
  if (!archiveId.value) return;

  // 检查缓存
  if (previewImageCache.value.has(pageNum)) {
    progressPreviewImage.value = previewImageCache.value.get(pageNum) || null;
    return;
  }

  // 清除之前的加载定时器
  if (previewLoadTimer.value) {
    clearTimeout(previewLoadTimer.value);
    previewLoadTimer.value = null;
  }

  // 设置防抖延迟
  previewLoadTimer.value = setTimeout(async () => {
    try {
      // 再次检查页面是否仍然匹配（防止用户快速滑动后页面已经改变）
      if (progressPreviewPage.value !== pageNum) {
        return;
      }

      // 加载缩略图（如果可用），否则加载完整页面
      const imageUrl = await getArchivePage(archiveId.value!, pageNum);
      previewImageCache.value.set(pageNum, imageUrl);

      // 只有在当前预览页面匹配时才设置图片
      if (progressPreviewPage.value === pageNum) {
        progressPreviewImage.value = imageUrl;
      }
    } catch (error) {
      console.warn("Failed to load preview image:", error);
      progressPreviewImage.value = null;
    }
    previewLoadTimer.value = null;
  }, 200); // 200ms 防抖延迟
};

const hideProgressPreview = () => {
  // 清除预览加载定时器
  if (previewLoadTimer.value) {
    clearTimeout(previewLoadTimer.value);
    previewLoadTimer.value = null;
  }

  showProgressPreview.value = false;
  progressPreviewPage.value = 0;
  progressPreviewImage.value = null;
};

const handlePreviewImageError = () => {
  progressPreviewImage.value = null;
};

// 组件事件处理函数
const handleAddTag = async (tag: { namespace: string; name: string }) => {
  try {
    // 先创建或获取tag
    const createdTag = await createTag(tag.name, tag.namespace);

    // 将tag关联到archive
    await addTagToArchive(archiveId.value, createdTag.id);

    // 刷新漫画信息
    queryClient.invalidateQueries({ queryKey: ["archive", archiveId.value] });
  } catch (error) {
    console.error("Failed to add tag:", error);
  }
};

const handleRemoveTag = async (tagId: string) => {
  try {
    // 从档案中移除标签关联
    await removeTagFromArchive(archiveId.value, tagId);

    // 刷新漫画信息
    queryClient.invalidateQueries({ queryKey: ["archive", archiveId.value] });
  } catch (error) {
    console.error("Failed to remove tag:", error);
  }
};

// 删除漫画
const handleDeleteArchive = async () => {
  try {
    // 删除档案
    await deleteArchive(archiveId.value, {
      recommendationSessionId: recommendationSessionId.value || undefined,
      recommendationPosition: recommendationPosition.value ?? undefined,
    });

    router.push("/library");
  } catch (error) {
    console.error("Failed to delete archive:", error);
  }
};

const handleExecutePlugin = (payload: ExecutePluginPayload) => {
  if (!payload.pluginId || executePluginMutation.isPending.value) return;
  executePluginMutation.mutate(payload);
};

const handleSearchEhentai = async () => {
  if (!archiveId.value || ehentaiSearching.value || executePluginMutation.isPending.value) return;
  ehentaiSearching.value = true;
  ehentaiSearchError.value = null;
  ehentaiCandidates.value = [];
  try {
    const response = await searchEhentaiCandidates(archiveId.value);
    ehentaiCandidates.value = response.candidates;
    if (!response.candidates.length) {
      ehentaiSearchError.value = "没有找到候选。可以直接粘贴 E-Hentai 画廊链接。";
    }
  } catch (error) {
    ehentaiSearchError.value = error instanceof Error ? error.message : "搜索 E-Hentai 候选失败";
  } finally {
    ehentaiSearching.value = false;
  }
};

const handleSearchNhentai = async () => {
  if (!archiveId.value || nhentaiSearching.value || executePluginMutation.isPending.value) return;
  nhentaiSearching.value = true;
  nhentaiSearchError.value = null;
  nhentaiCandidates.value = [];
  try {
    const response = await searchNhentaiCandidates(archiveId.value);
    nhentaiCandidates.value = response.candidates;
    if (!response.candidates.length) {
      nhentaiSearchError.value = "没有找到候选。可以直接粘贴 nHentai 画廊链接或编号。";
    }
  } catch (error) {
    nhentaiSearchError.value = error instanceof Error ? error.message : "搜索 nHentai 候选失败";
  } finally {
    nhentaiSearching.value = false;
  }
};

// 设置面板事件处理函数
const handleSetDisplayMode = (mode: ImageDisplayMode) => {
  imageDisplayMode.value = mode;
  triggerPageTransition();
};

const handleSetReadingMode = (mode: ReadingMode) => {
  readingMode.value = mode;
  triggerPageTransition();
};

const handleToggleAnimation = () => {
  pageAnimationEnabled.value = !pageAnimationEnabled.value;
};

const handleToggleAutoHide = () => {
  autoHideUI.value = !autoHideUI.value;
};

const handleTogglePageNumbers = () => {
  showPageNumbers.value = !showPageNumbers.value;
};

const handleSettingsClose = () => {
  showSettings.value = false;
  if (showToolbar.value && autoHideUI.value) {
    setToolbarTimer();
  }
};

const handleSettingsToggle = () => {
  showSettings.value = !showSettings.value;
  
  if (showSettings.value) {
    clearToolbarTimer();
  } else if (showToolbar.value && autoHideUI.value) {
    setToolbarTimer();
  }
};

// 图片显示模式控制
const getImageClasses = () => {
  const baseClasses = "transition-all duration-300";
  switch (imageDisplayMode.value) {
    case "fit":
      return `${baseClasses} max-w-full max-h-full object-contain`;
    case "fill":
      return `${baseClasses} w-full h-full object-cover`;
    case "original":
      return `${baseClasses} object-none`;
    default:
      return `${baseClasses} max-w-full max-h-full object-contain`;
  }
};

const getImageStyles = () => {
  if (imageDisplayMode.value === "original") {
    return {
      maxWidth: "none",
      maxHeight: "none",
      width: "auto",
      height: "auto",
    };
  }
  return {};
};

const switchImageDisplayMode = () => {
  // 设置淡入淡出动画
  pageTransitionName.value = "fade";
  triggerPageTransition();

  const modes: Array<"fit" | "fill" | "original"> = ["fit", "fill", "original"];
  const currentIndex = modes.indexOf(imageDisplayMode.value);
  const nextIndex = (currentIndex + 1) % modes.length;
  imageDisplayMode.value = modes[nextIndex];
};

// 阅读模式控制
const getDoublePageImageClasses = () => {
  const baseClasses = "transition-all duration-300";
  switch (imageDisplayMode.value) {
    case "fit":
      return `${baseClasses} object-contain shrink-0`;
    case "fill":
      return `${baseClasses} object-cover shrink-0`;
    case "original":
      return `${baseClasses} object-none shrink-0`;
    default:
      return `${baseClasses} object-contain shrink-0`;
  }
};

const getDoublePageImageStyles = () => {
  const containerWidth = windowSize.value.width;
  const containerHeight = windowSize.value.height - 120; // 减去顶部和底部工具栏的高度

  switch (imageDisplayMode.value) {
    case "fit":
      return {
        maxWidth: `${Math.floor((containerWidth - 16) / 2)}px`, // 容器宽度的一半，减去gap
        maxHeight: `${containerHeight}px`,
      };
    case "fill":
      return {
        width: `${Math.floor((containerWidth - 16) / 2)}px`,
        height: `${containerHeight}px`,
      };
    case "original":
      return {
        maxWidth: `${Math.floor((containerWidth - 16) / 2)}px`,
        maxHeight: "none",
      };
    default:
      return {
        maxWidth: `${Math.floor((containerWidth - 16) / 2)}px`,
        maxHeight: `${containerHeight}px`,
      };
  }
};

const switchReadingMode = () => {
  // 设置淡入淡出动画
  pageTransitionName.value = "fade";
  triggerPageTransition();

  readingMode.value = readingMode.value === "single" ? "double" : "single";
  // 双页模式下，确保当前页是奇数页
  if (readingMode.value === "double" && currentPage.value % 2 === 0) {
    if (currentPage.value > 1) {
      currentPage.value--;
    }
  }
};

const getReadingModeLabel = () => {
  return readingMode.value === "single" ? "单页" : "双页";
};

const switchPageDirection = () => {
  pageDirection.value = pageDirection.value === "ltr" ? "rtl" : "ltr";
};

const getPageDirectionLabel = () => {
  return pageDirection.value === "ltr" ? "从左到右" : "从右到左";
};

const getDisplayModeLabel = () => {
  switch (imageDisplayMode.value) {
    case "fit":
      return "适应屏幕";
    case "fill":
      return "填充屏幕";
    case "original":
      return "原始尺寸";
    default:
      return "适应屏幕";
  }
};

// 工具方法
const formatFileSize = (bytes?: number): string => {
  if (!bytes) return "未知";

  const units = ["B", "KB", "MB", "GB"];
  let size = bytes;
  let unitIndex = 0;

  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex++;
  }

  return `${size.toFixed(1)} ${units[unitIndex]}`;
};

// 图片加载处理
const handleImageLoad = (event: Event) => {
  const img = event.target as HTMLImageElement;
  const altText = img.alt || "";
  const pageMatch = altText.match(/第 (\d+) 页/);
  const pageNum = pageMatch ? parseInt(pageMatch[1]) : currentPage.value;

  loadedImages.value.add(pageNum);
  console.log(`Image loaded for page ${pageNum}`);

  // 检查是否所有当前显示的页面都已加载完成
  let allCurrentPagesLoaded = false;
  if (readingMode.value === "single") {
    allCurrentPagesLoaded = loadedImages.value.has(currentPage.value);
  } else {
    // 双页模式：检查当前页和下一页是否都加载完成
    const currentLoaded = loadedImages.value.has(currentPage.value);
    const nextLoaded =
      currentPage.value >= totalPages.value ||
      loadedImages.value.has(currentPage.value + 1);
    allCurrentPagesLoaded = currentLoaded && nextLoaded;
  }

  if (allCurrentPagesLoaded) {
    // 当前显示的所有页面加载完成后，启动预加载
    setTimeout(() => {
      preloadPages();
    }, 100); // 很短的延迟，确保当前页面渲染完成
  }
};

const handleImageError = () => {
  isLoading.value = false;
  error.value = "图片加载失败";
};

// 键盘事件处理
const handleKeydown = (event: KeyboardEvent) => {
  switch (event.key) {
    case "ArrowLeft":
      prevPage();
      break;
    case "ArrowRight":
      nextPage();
      break;
    case "Escape":
      if (showInfoPanel.value) {
        hideInfoPanel();
      } else {
        goBack();
      }
      break;
    case " ":
      event.preventDefault();
      toggleInfoPanel();
      break;
    case "v":
    case "V":
      event.preventDefault();
      switchImageDisplayMode();
      break;
    case "d":
    case "D":
      event.preventDefault();
      switchReadingMode();
      break;
  }
};

// 鼠标移动事件处理
const handleMouseMove = (event: MouseEvent) => {
  // 忽略触摸交互期间的鼠标移动事件（防止触摸生成的合成鼠标事件触发工具栏）
  if (isTouchInteracting.value) {
    return;
  }
  showToolbarOnMouseMove(event);
};

// 窗口大小变化处理
const handleResize = () => {
  windowSize.value = { width: window.innerWidth, height: window.innerHeight };
};

// 移动端触摸手势处理
const handleTouchStart = (event: TouchEvent) => {
  if (event.touches.length === 1) {
    isTouchInteracting.value = true; // 标记开始触摸交互
    const touch = event.touches[0];
    touchStartX.value = touch.clientX;
    touchStartY.value = touch.clientY;
    touchStartTime.value = Date.now();
    isSwiping.value = false;
  }
};

const handleTouchMove = (event: TouchEvent) => {
  if (event.touches.length === 1 && !isSwiping.value) {
    const touch = event.touches[0];
    const deltaX = Math.abs(touch.clientX - touchStartX.value);
    const deltaY = Math.abs(touch.clientY - touchStartY.value);

    // 检测是否为水平滑动
    if (deltaX > deltaY && deltaX > 30) {
      isSwiping.value = true;
      event.preventDefault(); // 阻止默认的滚动行为
    }
  }
};

const handleTouchEnd = (event: TouchEvent) => {
  if (event.changedTouches.length === 1) {
    const touch = event.changedTouches[0];
    const deltaX = touch.clientX - touchStartX.value;
    const deltaY = touch.clientY - touchStartY.value;
    const deltaTime = Date.now() - touchStartTime.value;

    // 检测滑动手势
    if (
      Math.abs(deltaX) > Math.abs(deltaY) &&
      Math.abs(deltaX) > 50 &&
      deltaTime < 300
    ) {
      if (deltaX > 0) {
        // 向右滑动 - 根据翻页方向决定功能
        if (pageDirection.value === "ltr") {
          prevPage(); // 从左到右：向右滑动是上一页
        } else {
          nextPage(); // 从右到左：向右滑动是下一页
        }
      } else {
        // 向左滑动 - 根据翻页方向决定功能
        if (pageDirection.value === "ltr") {
          nextPage(); // 从左到右：向左滑动是下一页
        } else {
          prevPage(); // 从右到左：向左滑动是上一页
        }
      }
    } else if (
      Math.abs(deltaX) < 10 &&
      Math.abs(deltaY) < 10 &&
      deltaTime < 300
    ) {
      // 短时间内的小幅移动认为是点击
      handleTouchTap(touch);
    }

    // 重置状态
    isSwiping.value = false;
    touchStartX.value = 0;
    touchStartY.value = 0;
    touchStartTime.value = 0;
    
    // 延迟重置触摸交互标记，给合成鼠标事件一些时间处理完成
    setTimeout(() => {
      isTouchInteracting.value = false;
    }, 100);
  }
};

const handleTouchTap = (touch: Touch) => {
  const { clientX: x, clientY: y } = touch;
  const { innerWidth, innerHeight } = window;
  const centerX = innerWidth / 2;
  const centerY = innerHeight / 2;
  const squareHalf = 48; // 96px / 2
  
  // 中间正方形区域
  if (x >= centerX - squareHalf && x <= centerX + squareHalf && 
      y >= centerY - squareHalf && y <= centerY + squareHalf) {
    toggleToolbar();
    return;
  }
  
  // 左右翻页区域
  const isLeft = x < centerX - 60;
  const isRight = x > centerX + 60;
  
  if (isLeft || isRight) {
    const shouldPrev = (isLeft && pageDirection.value === "ltr") || 
                       (isRight && pageDirection.value === "rtl");
    shouldPrev ? prevPage() : nextPage();
  }
};

// 初始化阅读器状态
const initializeReader = () => {
  if (!archiveId.value) return;
  isLoading.value = true;
  error.value = null;
};

// 监听archiveId变化
watch(
  archiveId,
  (newArchiveId, oldArchiveId) => {
    if (!newArchiveId) return;
    leaveProgressFlushed.value = false;
    readerSessionKey.value =
      typeof crypto !== "undefined" && "randomUUID" in crypto
        ? crypto.randomUUID()
        : `${newArchiveId}-${Date.now()}`;
    readerSessionStartedAt.value = Date.now();
    emitBehaviorEvent("open", {
      eventKey: readerSessionKey.value,
      source: "reader",
    });
    
    // 每本书的页图、预加载缓存和请求生命周期都必须隔离。
    if (oldArchiveId !== undefined && newArchiveId !== oldArchiveId) {
      clearArchivePageResources();
      restoredProgressArchiveId.value = null;
      currentPage.value = 1;
      totalPages.value = 1;
      lastPluginExecutionSummary.value = null;
    }
    initializeReader();

    if (oldArchiveId !== undefined && newArchiveId !== oldArchiveId) {
      void loadCurrentPage(true);
    }
  },
  { immediate: true }
);

// 页面可见性变化处理
const handleVisibilityChange = () => {
  if (!document.hidden || !saveProgressTimer.value) return;
  
  clearTimeout(saveProgressTimer.value);
  saveProgressTimer.value = null;

  const finalPage = pendingProgressPage.value || currentPage.value;
  if (archiveId.value && finalPage > 0) {
    updateProgressMutation.mutate({
      archiveId: archiveId.value,
      currentPage: finalPage,
      readerSessionId: readerSessionKey.value,
      recommendationSessionId: recommendationSessionId.value,
    });
    pendingProgressPage.value = null;
  }
};

onMounted(() => {
  // 添加键盘和鼠标事件监听
  document.addEventListener("keydown", handleKeydown);
  document.addEventListener("mousemove", handleMouseMove);
  window.addEventListener("resize", handleResize);
  document.addEventListener("visibilitychange", handleVisibilityChange);
  // 全屏状态监听
  document.addEventListener("fullscreenchange", handleFullscreenChange);
  document.addEventListener("webkitfullscreenchange", handleFullscreenChange);
  document.addEventListener("msfullscreenchange", handleFullscreenChange);
});

onUnmounted(() => {
  // 立即保存当前进度（不使用防抖）
  if (saveProgressTimer.value) {
    clearTimeout(saveProgressTimer.value);
    saveProgressTimer.value = null;
  }

  // 直接保存最新进度
  if (!leaveProgressFlushed.value && archiveId.value && currentPage.value > 0) {
    const finalPage = pendingProgressPage.value || currentPage.value;
    console.log(`Saving final progress on unmount: page ${finalPage}`);
    updateProgressMutation.mutate({
      archiveId: archiveId.value,
      currentPage: finalPage,
      readerSessionId: readerSessionKey.value,
      recommendationSessionId: recommendationSessionId.value,
    });
  }

  // 清理事件监听和定时器
  document.removeEventListener("keydown", handleKeydown);
  document.removeEventListener("mousemove", handleMouseMove);
  window.removeEventListener("resize", handleResize);
  document.removeEventListener("visibilitychange", handleVisibilityChange);
  document.removeEventListener("fullscreenchange", handleFullscreenChange);
  document.removeEventListener(
    "webkitfullscreenchange",
    handleFullscreenChange,
  );
  document.removeEventListener("msfullscreenchange", handleFullscreenChange);
  clearAutoHideTimer();
  clearToolbarTimer();

  // 清理预览加载定时器
  if (previewLoadTimer.value) {
    clearTimeout(previewLoadTimer.value);
    previewLoadTimer.value = null;
  }

  // 清理图片URL对象，防止内存泄露
  if (currentPageUrl.value) {
    URL.revokeObjectURL(currentPageUrl.value);
  }
  if (nextPageUrl.value) {
    URL.revokeObjectURL(nextPageUrl.value);
  }
  preloadedUrls.value.forEach((url) => URL.revokeObjectURL(url));
  preloadedUrls.value.clear();

  // 清理预览图片缓存
  previewImageCache.value.forEach((url) => URL.revokeObjectURL(url));
  previewImageCache.value.clear();
});
</script>

<style scoped>
/* 进度条样式 */
.slider {
  -webkit-appearance: none;
  appearance: none;
  background: transparent;
  cursor: pointer;
}

.slider::-webkit-slider-track {
  background: rgb(55, 65, 81);
  height: 8px;
  border-radius: 4px;
}

.slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  background: rgb(59, 130, 246);
  height: 16px;
  width: 16px;
  border-radius: 50%;
  cursor: pointer;
  transition: all 0.2s ease-in-out;
}

.slider::-webkit-slider-thumb:hover {
  background: rgb(37, 99, 235);
  height: 18px;
  width: 18px;
}

.slider::-moz-range-track {
  background: rgb(55, 65, 81);
  height: 8px;
  border-radius: 4px;
  border: none;
}

.slider::-moz-range-thumb {
  background: rgb(59, 130, 246);
  height: 16px;
  width: 16px;
  border-radius: 50%;
  cursor: pointer;
  border: none;
  transition: all 0.2s ease-in-out;
}

.slider::-moz-range-thumb:hover {
  background: rgb(37, 99, 235);
  height: 18px;
  width: 18px;
}

/* 现代化工具栏进度条样式 */
.slider-modern {
  -webkit-appearance: none;
  appearance: none;
  background: transparent;
  cursor: pointer;
}

.slider-modern::-webkit-slider-track {
  background: rgba(255, 255, 255, 0.2);
  height: 6px;
  border-radius: 3px;
}

.slider-modern::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  background: rgb(59, 130, 246);
  height: 18px;
  width: 18px;
  border-radius: 50%;
  cursor: pointer;
  transition: all 0.2s ease-in-out;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.3);
}

.slider-modern::-webkit-slider-thumb:hover {
  background: rgb(37, 99, 235);
  height: 20px;
  width: 20px;
  box-shadow: 0 4px 12px rgba(59, 130, 246, 0.4);
}

.slider-modern::-moz-range-track {
  background: rgba(255, 255, 255, 0.2);
  height: 6px;
  border-radius: 3px;
  border: none;
}

.slider-modern::-moz-range-thumb {
  background: rgb(59, 130, 246);
  height: 18px;
  width: 18px;
  border-radius: 50%;
  cursor: pointer;
  border: none;
  transition: all 0.2s ease-in-out;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.3);
}

.slider-modern::-moz-range-thumb:hover {
  background: rgb(37, 99, 235);
  height: 20px;
  width: 20px;
  box-shadow: 0 4px 12px rgba(59, 130, 246, 0.4);
}

/* 毛玻璃风格进度条样式 */
.slider-glass {
  -webkit-appearance: none;
  appearance: none;
  background: transparent;
  cursor: pointer;
  height: 8px;
  border-radius: 4px;
}

.slider-glass::-webkit-slider-track {
  background: linear-gradient(
    90deg,
    rgba(255, 255, 255, 0.1) 0%,
    rgba(255, 255, 255, 0.2) 50%,
    rgba(255, 255, 255, 0.1) 100%
  );
  height: 8px;
  border-radius: 4px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.3);
}

.slider-glass::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  background: linear-gradient(135deg, #ffffff, #e5e7eb);
  height: 20px;
  width: 20px;
  border-radius: 50%;
  cursor: pointer;
  transition: all 0.4s cubic-bezier(0.68, -0.55, 0.265, 1.55);
  transform: scale(1);
  box-shadow:
    0 2px 8px rgba(107, 114, 128, 0.3),
    0 1px 3px rgba(0, 0, 0, 0.2);
  border: 2px solid rgba(107, 114, 128, 0.6);
  backdrop-filter: blur(6px);
}

.slider-glass::-webkit-slider-thumb:hover {
  background: linear-gradient(135deg, #f3f4f6, #d1d5db);
  transform: scale(1.3) translateY(-2px);
  box-shadow:
    0 6px 20px rgba(107, 114, 128, 0.5),
    0 4px 8px rgba(0, 0, 0, 0.3),
    0 0 0 4px rgba(255, 255, 255, 0.1);
  border: 2px solid rgba(75, 85, 99, 0.8);
}

.slider-glass::-webkit-slider-thumb:active {
  transform: scale(1.3) translateY(-2px);
  transition: all 0.15s cubic-bezier(0.68, -0.55, 0.265, 1.55);
  box-shadow:
    0 6px 20px rgba(107, 114, 128, 0.6),
    0 4px 8px rgba(0, 0, 0, 0.4),
    0 0 0 4px rgba(255, 255, 255, 0.2);
}

.slider-glass::-moz-range-track {
  background: linear-gradient(
    90deg,
    rgba(255, 255, 255, 0.1) 0%,
    rgba(255, 255, 255, 0.2) 50%,
    rgba(255, 255, 255, 0.1) 100%
  );
  height: 8px;
  border-radius: 4px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.3);
}

.slider-glass::-moz-range-thumb {
  background: linear-gradient(135deg, #ffffff, #e5e7eb);
  height: 20px;
  width: 20px;
  border-radius: 50%;
  cursor: pointer;
  border: 2px solid rgba(107, 114, 128, 0.6);
  transition: all 0.4s cubic-bezier(0.68, -0.55, 0.265, 1.55);
  transform: scale(1);
  box-shadow:
    0 2px 8px rgba(107, 114, 128, 0.3),
    0 1px 3px rgba(0, 0, 0, 0.2);
}

.slider-glass::-moz-range-thumb:hover {
  background: linear-gradient(
    145deg,
    rgba(37, 99, 235, 0.95),
    rgba(29, 78, 216, 1)
  );
  background: linear-gradient(135deg, #f3f4f6, #d1d5db);
  transform: scale(1.3);
  box-shadow:
    0 6px 20px rgba(107, 114, 128, 0.5),
    0 4px 8px rgba(0, 0, 0, 0.3),
    0 0 0 4px rgba(255, 255, 255, 0.1);
  border: 2px solid rgba(75, 85, 99, 0.8);
}

.slider-glass::-moz-range-thumb:active {
  transform: scale(1.3);
  transition: all 0.15s cubic-bezier(0.68, -0.55, 0.265, 1.55);
  box-shadow:
    0 6px 20px rgba(107, 114, 128, 0.6),
    0 4px 8px rgba(0, 0, 0, 0.4),
    0 0 0 4px rgba(255, 255, 255, 0.2);
}

/* 翻页动画样式 */
.slide-left-enter-active,
.slide-left-leave-active,
.slide-right-enter-active,
.slide-right-leave-active {
  transition: all 0.35s cubic-bezier(0.4, 0, 0.2, 1);
}

.slide-left-enter-from {
  transform: translateX(100%);
  opacity: 0;
}

.slide-left-leave-to {
  transform: translateX(-100%);
  opacity: 0;
}

.slide-right-enter-from {
  transform: translateX(-100%);
  opacity: 0;
}

.slide-right-leave-to {
  transform: translateX(100%);
  opacity: 0;
}

/* 合集内换册使用不同于翻页的纵向翻书过渡。 */
.book-switch-forward-enter-active,
.book-switch-forward-leave-active,
.book-switch-backward-enter-active,
.book-switch-backward-leave-active {
  transition: transform 0.45s cubic-bezier(0.22, 0.8, 0.24, 1), opacity 0.32s ease;
}

.book-switch-forward-enter-from {
  transform: translateY(14%) scale(0.9);
  opacity: 0;
}

.book-switch-forward-leave-to {
  transform: translateY(-5%) scale(1.04);
  opacity: 0;
}

.book-switch-backward-enter-from {
  transform: translateY(-14%) scale(0.9);
  opacity: 0;
}

.book-switch-backward-leave-to {
  transform: translateY(5%) scale(1.04);
  opacity: 0;
}

.book-switch-notice-enter-active,
.book-switch-notice-leave-active {
  transition: opacity 0.22s ease;
}

.book-switch-notice-enter-from,
.book-switch-notice-leave-to {
  opacity: 0;
}

/* 淡入淡出动画（用于阅读模式切换等） */
.fade-enter-active,
.fade-leave-active {
  transition: all 0.25s ease-in-out;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: scale(0.95);
}

/* 按钮悬停效果优化 */
button {
  position: relative;
  overflow: hidden;
}

button::before {
  content: "";
  position: absolute;
  top: 50%;
  left: 50%;
  width: 0;
  height: 0;
  background: rgba(255, 255, 255, 0.1);
  border-radius: 50%;
  transform: translate(-50%, -50%);
  transition:
    width 0.3s ease-out,
    height 0.3s ease-out;
  pointer-events: none;
}

button:hover::before {
  width: 200%;
  height: 200%;
}

/* 工具栏按钮特殊效果 */
.toolbar-button {
  backdrop-filter: blur(8px);
  border: 1px solid rgba(255, 255, 255, 0.1);
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.toolbar-button:hover {
  background: rgba(255, 255, 255, 0.15);
  border-color: rgba(255, 255, 255, 0.2);
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
}

.reader-toolbar-actions {
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.reader-toolbar-actions::-webkit-scrollbar {
  display: none;
}
</style>
