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
            fill="none" stroke="currentColor" viewBox="0 0 24 24"
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
            fill="none" stroke="currentColor" viewBox="0 0 24 24"
          >
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
        </button>
      </div>
    </div>

    <!-- 展开内容 -->
    <div v-if="!isCollapsed" class="px-3 pb-2">
      <!-- 横向滚动卡片列表 -->
      <div class="carousel-scroll flex gap-2 overflow-x-auto" style="scroll-snap-type: x mandatory;">
        <!-- 加载骨架 -->
        <template v-if="isLoading">
          <div
            v-for="i in 8"
            :key="'skeleton-' + i"
            class="flex-shrink-0 w-[90px] h-[135px] rounded bg-[var(--bg-tertiary)] animate-pulse"
            style="scroll-snap-align: start;"
          />
        </template>

        <!-- 卡片列表 -->
        <template v-else>
          <div
            v-for="archive in archives"
            :key="archive.id"
            class="flex-shrink-0 cursor-pointer group"
            style="scroll-snap-align: start; width: 90px;"
            @click="openReader(archive.id)"
          >
            <!-- 图片 -->
            <div class="relative w-full rounded overflow-hidden bg-[var(--bg-tertiary)] border border-[var(--border)] group-hover:border-[var(--accent)] transition-colors"
              style="height: 120px;">
              <img
                v-if="thumbnails[archive.id]"
                :src="thumbnails[archive.id]"
                :alt="archive.title"
                class="w-full h-full object-cover"
              />
              <div v-else class="w-full h-full flex items-center justify-center text-[var(--text-tertiary)]">
                <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
                    d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
                </svg>
              </div>
            </div>
            <!-- 标题 -->
            <p class="text-[10px] text-[var(--text-secondary)] mt-1 leading-tight truncate px-0.5">{{ archive.title }}</p>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useQuery, useQueryClient } from '@tanstack/vue-query'
import { getRandomArchives, getArchiveThumbnail } from '@/utils/api'
import { useLibraryStore } from '@/stores/library'

interface Props {
  categoryId?: string
  searchQuery?: string
  tags?: string[]
  minPages?: number
  maxPages?: number
  createdAfter?: string
  createdBefore?: string
}

const props = withDefaults(defineProps<Props>(), {
  categoryId: '',
  searchQuery: ''
})

const router = useRouter()
const queryClient = useQueryClient()
const libraryStore = useLibraryStore()

const isRefreshing = ref(false)
const isCollapsed = ref(!libraryStore.showCarousel)
const thumbnails = ref<Record<string, string>>({})

const { data, isLoading } = useQuery({
  queryKey: computed(() => ['randomArchives', props.categoryId, props.searchQuery, props.tags, props.minPages, props.maxPages, props.createdAfter, props.createdBefore]),
  queryFn: () => getRandomArchives({
    count: 12,
    categoryId: props.categoryId || undefined,
    query: props.searchQuery || undefined,
    tags: props.tags && props.tags.length > 0 ? props.tags : undefined,
    minPages: props.minPages,
    maxPages: props.maxPages,
    createdAfter: props.createdAfter,
    createdBefore: props.createdBefore,
  }),
  retry: 1,
  staleTime: 5 * 60 * 1000,
})

const archives = computed(() => data.value || [])

const loadThumbnails = async (ids: string[]) => {
  for (const id of ids) {
    if (thumbnails.value[id]) continue
    try {
      const url = await getArchiveThumbnail(id)
      thumbnails.value[id] = url
    } catch { /* ignore */ }
  }
}

watch(archives, (newArchives) => {
  if (newArchives.length > 0) {
    loadThumbnails(newArchives.map(a => a.id))
  }
}, { immediate: true })

const toggleCollapse = () => {
  isCollapsed.value = !isCollapsed.value
  libraryStore.setShowCarousel(!isCollapsed.value)
}

const handleRefresh = () => {
  isRefreshing.value = true
  Object.values(thumbnails.value).forEach(url => URL.revokeObjectURL(url))
  thumbnails.value = {}
  queryClient.invalidateQueries({ queryKey: ['randomArchives'] })
  setTimeout(() => { isRefreshing.value = false }, 1000)
}

// 当分类、搜索或筛选变化时清除旧缩略图
watch(() => [props.categoryId, props.searchQuery, props.tags, props.minPages, props.maxPages, props.createdAfter, props.createdBefore], () => {
  Object.values(thumbnails.value).forEach(url => URL.revokeObjectURL(url))
  thumbnails.value = {}
})

const openReader = (archiveId: string) => {
  router.push(`/reader/${archiveId}`)
}
</script>

<style scoped>
.carousel-scroll {
  -webkit-overflow-scrolling: touch;
  scrollbar-width: none;
  -ms-overflow-style: none;
}
.carousel-scroll::-webkit-scrollbar {
  display: none;
}
</style>
