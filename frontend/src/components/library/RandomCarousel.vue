<template>
  <div class="carousel-section border-b border-[var(--border)]">
    <!-- 标题行 + 收起按钮 -->
    <div class="flex items-center justify-between px-3 py-1.5 bg-[var(--bg-primary)]">
      <div class="flex items-center gap-2">
        <button
          class="flex items-center text-xs text-[var(--text-secondary)] hover:text-[var(--text-primary)] transition-colors"
          @click="toggleCollapse"
        >
          <svg
            class="w-3.5 h-3.5 mr-1 transition-transform duration-200"
            :class="{ '-rotate-90': isCollapsed }"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
          </svg>
          随机精选
        </button>
      </div>

      <div class="flex items-center gap-1">
        <button
          v-if="!isCollapsed"
          class="p-1 rounded text-[var(--text-tertiary)] hover:text-[var(--accent)] transition-colors"
          title="刷新"
          @click="handleRefresh"
        >
          <svg
            class="w-3.5 h-3.5 transition-transform duration-700"
            :class="{ 'animate-spin': isRefreshing }"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
            />
          </svg>
        </button>
      </div>
    </div>

    <!-- 展开内容 -->
    <div v-if="!isCollapsed" class="px-3 pb-2">
      <div ref="carouselScroll" class="carousel-scroll pb-0.5">
        <!-- 加载骨架 -->
        <template v-if="isLoading">
          <div
            v-for="i in 8"
            :key="'skeleton-' + i"
            class="carousel-item h-[245px] rounded bg-[var(--bg-tertiary)] animate-pulse"
          />
        </template>

        <!-- 卡片列表 -->
        <template v-else>
          <div
            v-for="archive in archives"
            :key="archive.id"
            class="carousel-item"
          >
            <ArchiveThumbnailCard
              :archive="archive"
              @click="$emit('open-archive', archive.id, data?.sessionId, archives.indexOf(archive))"
              @contextmenu="$emit('archive-contextmenu', $event, archive, data?.sessionId, archives.indexOf(archive))"
            />
          </div>
        </template>

        <div
          v-if="!isLoading && archives.length === 0"
          class="text-xs text-[var(--text-tertiary)] py-8"
        >
          暂无随机漫画
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, nextTick } from 'vue'
import { useQuery, useQueryClient } from '@tanstack/vue-query'
import { getRandomArchiveSession } from '@/utils/api'
import { useLibraryStore } from '@/stores/library'
import ArchiveThumbnailCard from '@/components/ArchiveThumbnailCard.vue'
import type { Archive } from '@/types/api'

interface Props {
  categoryId?: string
  searchQuery?: string
  tags?: string[]
  minPages?: number
  maxPages?: number
  minFileSize?: number
  maxFileSize?: number
  createdAfter?: string
  createdBefore?: string
}

const props = withDefaults(defineProps<Props>(), {
  categoryId: '',
  searchQuery: ''
})

defineEmits<{
  'open-archive': [archiveId: string, recommendationSessionId?: string, recommendationPosition?: number]
  'archive-contextmenu': [event: MouseEvent, archive: Archive, recommendationSessionId?: string, recommendationPosition?: number]
}>()

const queryClient = useQueryClient()
const libraryStore = useLibraryStore()

const isRefreshing = ref(false)
const isCollapsed = ref(!libraryStore.showCarousel)
const carouselScroll = ref<HTMLElement | null>(null)

const { data, isLoading } = useQuery({
  queryKey: computed(() => [
    'randomArchives',
    props.categoryId,
    props.searchQuery,
    props.tags,
    props.minPages,
    props.maxPages,
    props.minFileSize,
    props.maxFileSize,
    props.createdAfter,
    props.createdBefore,
  ]),
  queryFn: () => getRandomArchiveSession({
    count: 12,
    categoryId: props.categoryId || undefined,
    query: props.searchQuery || undefined,
    tags: props.tags && props.tags.length > 0 ? props.tags : undefined,
    minPages: props.minPages,
    maxPages: props.maxPages,
    minFileSize: props.minFileSize,
    maxFileSize: props.maxFileSize,
    createdAfter: props.createdAfter,
    createdBefore: props.createdBefore,
  }),
  retry: 1,
  staleTime: 5 * 60 * 1000,
})

const archives = computed(() => data.value?.archives || [])

const toggleCollapse = () => {
  isCollapsed.value = !isCollapsed.value
  libraryStore.setShowCarousel(!isCollapsed.value)
}

const resetCarouselPosition = () => {
  carouselScroll.value?.scrollTo({ left: 0, behavior: 'auto' })
}

const handleRefresh = async () => {
  isRefreshing.value = true
  resetCarouselPosition()

  try {
    await queryClient.invalidateQueries({ queryKey: ['randomArchives'] })
    await nextTick()
    resetCarouselPosition()
  } finally {
    isRefreshing.value = false
  }
}
</script>

<style scoped>
.carousel-scroll {
  --cols: 3;
  --gap: 0.5rem;
  display: grid;
  grid-auto-flow: column;
  gap: var(--gap);
  grid-auto-columns: calc((100% - (var(--cols) - 1) * var(--gap)) / var(--cols));
  overflow-x: auto;
  scroll-snap-type: x mandatory;
  -webkit-overflow-scrolling: touch;
  scrollbar-width: none;
  -ms-overflow-style: none;
}
.carousel-scroll::-webkit-scrollbar {
  display: none;
}

.carousel-item {
  scroll-snap-align: start;
}

@media (min-width: 640px) {
  .carousel-scroll {
    --cols: 4;
  }
}

@media (min-width: 768px) {
  .carousel-scroll {
    --cols: 5;
  }
}

@media (min-width: 1024px) {
  .carousel-scroll {
    --cols: 6;
  }
}

@media (min-width: 1280px) {
  .carousel-scroll {
    --cols: 7;
  }
}

@media (min-width: 1536px) {
  .carousel-scroll {
    --cols: 8;
  }
}
</style>
