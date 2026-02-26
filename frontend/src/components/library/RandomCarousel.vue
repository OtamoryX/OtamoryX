<template>
  <div v-if="!isLoaded || (archives && archives.length > 0)" class="px-4 py-3">
    <!-- 标题行 -->
    <div class="flex items-center justify-between mb-3">
      <h2 class="text-gray-900 dark:text-gray-100 font-semibold text-base">随机精选</h2>
      <button
        class="p-1.5 rounded-md text-gray-500 hover:text-blue-500 transition-colors"
        aria-label="刷新随机精选"
        @click="handleRefresh"
      >
        <svg
          class="w-5 h-5 transition-transform duration-1000"
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

    <!-- 横向滚动卡片列表 -->
    <div class="carousel-scroll flex gap-3 overflow-x-auto pb-2" style="scroll-snap-type: x mandatory;">
      <!-- 加载骨架屏 -->
      <template v-if="isLoading">
        <div
          v-for="i in 5"
          :key="'skeleton-' + i"
          class="flex-shrink-0 w-[120px] h-[160px] md:w-[150px] md:h-[200px] rounded-lg bg-gray-100 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 animate-pulse"
          style="scroll-snap-align: start;"
        />
      </template>

      <!-- 卡片列表 -->
      <template v-else>
        <div
          v-for="archive in archives"
          :key="archive.id"
          class="flex-shrink-0 w-[120px] md:w-[150px] cursor-pointer group"
          style="scroll-snap-align: start;"
          @click="openReader(archive.id)"
        >
          <div class="relative w-[120px] h-[160px] md:w-[150px] md:h-[200px] rounded-lg overflow-hidden bg-gray-100 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 hover:shadow-md transition-shadow">
            <!-- 封面图片 -->
            <img
              v-if="thumbnails[archive.id]"
              :src="thumbnails[archive.id]"
              :alt="archive.title"
              class="w-full h-full object-cover"
            />
            <!-- 图片加载中 / 失败时的占位 -->
            <div
              v-else
              class="w-full h-full flex items-center justify-center text-gray-400 dark:text-gray-500"
            >
              <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="1.5"
                  d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"
                />
              </svg>
            </div>
            <!-- 底部标题 -->
            <div class="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/70 to-transparent px-2 py-1.5">
              <p class="text-white text-xs truncate">{{ archive.title }}</p>
            </div>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, computed, onBeforeUnmount } from 'vue'
import { useRouter } from 'vue-router'
import { useQuery, useQueryClient } from '@tanstack/vue-query'
import { getRandomArchives, getArchiveThumbnail } from '@/utils/api'

const router = useRouter()
const queryClient = useQueryClient()
const isRefreshing = ref(false)
const thumbnails = ref<Record<string, string>>({})

// 获取随机档案
const { data, isLoading } = useQuery({
  queryKey: ['randomArchives'],
  queryFn: () => getRandomArchives(10),
  retry: 1,
  staleTime: 5 * 60 * 1000,
})

const archives = computed(() => data.value || [])
const isLoaded = computed(() => !isLoading.value)

// 加载缩略图
const loadThumbnails = async (ids: string[]) => {
  for (const id of ids) {
    if (thumbnails.value[id]) continue
    try {
      const url = await getArchiveThumbnail(id)
      thumbnails.value[id] = url
    } catch {
      // 加载失败时保持占位图标
    }
  }
}

// 监听档案数据变化，加载缩略图
watch(archives, (newArchives) => {
  if (newArchives.length > 0) {
    loadThumbnails(newArchives.map(a => a.id))
  }
}, { immediate: true })

// 刷新
const handleRefresh = () => {
  isRefreshing.value = true
  // 清除旧缩略图的 blob URL
  Object.values(thumbnails.value).forEach(url => URL.revokeObjectURL(url))
  thumbnails.value = {}
  queryClient.invalidateQueries({ queryKey: ['randomArchives'] })
  setTimeout(() => {
    isRefreshing.value = false
  }, 1000)
}

// 导航到阅读器
const openReader = (archiveId: string) => {
  router.push(`/reader/${archiveId}`)
}

// 组件卸载时清理 blob URL
onBeforeUnmount(() => {
  Object.values(thumbnails.value).forEach(url => URL.revokeObjectURL(url))
})
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
