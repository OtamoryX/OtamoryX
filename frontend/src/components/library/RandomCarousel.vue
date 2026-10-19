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
            v-for="(archive, position) in archives"
            :key="archive.id"
            class="carousel-item"
            :data-position="position"
          >
            <ArchiveThumbnailCard
              :archive="archive"
              @click="$emit('open-archive', archive.id, data?.sessionId, position)"
              @contextmenu="$emit('archive-contextmenu', $event, archive, data?.sessionId, position)"
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
import { ref, computed, nextTick, onBeforeUnmount, watch } from 'vue'
import { useQuery, useQueryClient } from '@tanstack/vue-query'
import {
  getRandomArchiveSession,
  reportRandomRecommendationImpressions,
} from '@/utils/api'
import { useLibraryStore } from '@/stores/library'
import ArchiveThumbnailCard from '@/components/ArchiveThumbnailCard.vue'
import type { Archive } from '@/types/api'

interface Props {
  categoryId?: string
  searchQuery?: string
  tags?: string[]
  themeIds?: string[]
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
    props.themeIds,
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
    themeIds: props.themeIds && props.themeIds.length > 0 ? props.themeIds : undefined,
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

// Report the cards that actually entered the carousel viewport so the backend can downweight
// exactly what the user saw on the next refresh. The first screen is assumed seen by the server;
// cards further right are only counted once this observer has reported them.
const IMPRESSION_DEBOUNCE_MS = 400
const reportedPositions = new Set<number>()
const pendingPositions = new Set<number>()
let impressionTimer: ReturnType<typeof setTimeout> | null = null
let cardObserver: IntersectionObserver | null = null

const flushImpressions = async () => {
  const sessionId = data.value?.sessionId
  if (!sessionId || pendingPositions.size === 0) return
  const positions = [...pendingPositions]
    .filter((position) => !reportedPositions.has(position))
    .sort((left, right) => left - right)
  pendingPositions.clear()
  if (positions.length === 0) return
  for (const position of positions) reportedPositions.add(position)
  try {
    await reportRandomRecommendationImpressions(sessionId, positions)
  } catch (error) {
    if (data.value?.sessionId === sessionId) {
      for (const position of positions) reportedPositions.delete(position)
    }
    console.debug('recommendation impressions were not recorded', error)
  }
}

const scheduleImpressionFlush = () => {
  if (impressionTimer !== null) return
  impressionTimer = setTimeout(() => {
    impressionTimer = null
    void flushImpressions()
  }, IMPRESSION_DEBOUNCE_MS)
}

const disconnectObserver = () => {
  cardObserver?.disconnect()
  cardObserver = null
}

const observeCards = async () => {
  disconnectObserver()
  await nextTick()
  const root = carouselScroll.value
  if (!root || typeof IntersectionObserver === 'undefined') return
  cardObserver = new IntersectionObserver(
    (entries) => {
      let discovered = false
      for (const entry of entries) {
        if (!entry.isIntersecting || entry.intersectionRatio < 0.5) continue
        const position = Number((entry.target as HTMLElement).dataset.position)
        if (!Number.isInteger(position) || reportedPositions.has(position)) continue
        pendingPositions.add(position)
        discovered = true
      }
      if (discovered) scheduleImpressionFlush()
    },
    { root, threshold: 0.5 },
  )
  root.querySelectorAll<HTMLElement>('.carousel-item').forEach((card) => {
    cardObserver?.observe(card)
  })
}

watch(
  () => data.value?.sessionId,
  () => {
    reportedPositions.clear()
    pendingPositions.clear()
    void observeCards()
  },
  { immediate: true },
)

watch(
  () => isCollapsed.value,
  (collapsed) => {
    if (collapsed) disconnectObserver()
    else void observeCards()
  },
)

onBeforeUnmount(() => {
  void flushImpressions()
  disconnectObserver()
  if (impressionTimer !== null) clearTimeout(impressionTimer)
})

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
  // Send what was already on screen for the outgoing session before the new one replaces it.
  void flushImpressions()

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
