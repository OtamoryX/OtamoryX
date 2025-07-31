import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { Archive } from '@/types/api'

export const useReaderStore = defineStore('reader', () => {
  const currentArchive = ref<Archive | null>(null)
  const currentPage = ref(0)
  const readingMode = ref<'single' | 'double'>('single')
  const zoomLevel = ref(1)
  const isFullscreen = ref(false)

  const setArchive = (archive: Archive) => {
    currentArchive.value = archive
    currentPage.value = 1 // 从第1页开始
  }

  const nextPage = () => {
    if (currentPage.value < (currentArchive.value?.pageCount ?? 0)) {
      currentPage.value++
    }
  }

  const prevPage = () => {
    if (currentPage.value > 1) {
      currentPage.value--
    }
  }

  const goToPage = (page: number) => {
    if (page >= 1 && page <= (currentArchive.value?.pageCount ?? 0)) {
      currentPage.value = page
    }
  }

  const setReadingMode = (mode: 'single' | 'double') => {
    readingMode.value = mode
  }

  const setZoomLevel = (level: number) => {
    zoomLevel.value = Math.max(0.1, Math.min(3, level)) // 限制缩放范围
  }

  const toggleFullscreen = () => {
    isFullscreen.value = !isFullscreen.value
  }

  const resetReader = () => {
    currentArchive.value = null
    currentPage.value = 0
    zoomLevel.value = 1
    isFullscreen.value = false
  }

  // 计算属性
  const progress = computed(() => {
    if (!currentArchive.value) return 0
    return (currentPage.value / currentArchive.value.pageCount) * 100
  })

  const canGoNext = computed(() => {
    return currentPage.value < (currentArchive.value?.pageCount ?? 0)
  })

  const canGoPrev = computed(() => {
    return currentPage.value > 1
  })

  return {
    currentArchive,
    currentPage,
    readingMode,
    zoomLevel,
    isFullscreen,
    progress,
    canGoNext,
    canGoPrev,
    setArchive,
    nextPage,
    prevPage,
    goToPage,
    setReadingMode,
    setZoomLevel,
    toggleFullscreen,
    resetReader
  }
})