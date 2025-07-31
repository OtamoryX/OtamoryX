import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useReaderStore } from '@/stores/reader'

export const useReader = () => {
  const readerStore = useReaderStore()
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  // 预加载下一页图片
  const preloadNextPage = () => {
    if (readerStore.canGoNext) {
      const nextPageUrl = `/api/v1/archives/${readerStore.currentArchive?.id}/pages/${readerStore.currentPage + 1}`
      const img = new Image()
      img.src = nextPageUrl
    }
  }

  // 当前页面图片URL
  const currentPageUrl = computed(() => {
    if (!readerStore.currentArchive) return ''
    return `/api/v1/archives/${readerStore.currentArchive.id}/pages/${readerStore.currentPage}`
  })

  // 处理图片加载
  const handleImageLoad = () => {
    isLoading.value = false
    error.value = null
    preloadNextPage()
  }

  const handleImageError = () => {
    isLoading.value = false
    error.value = '图片加载失败'
  }

  // 键盘控制
  const handleKeydown = (event: KeyboardEvent) => {
    switch (event.key) {
      case 'ArrowLeft':
      case 'h':
        readerStore.prevPage()
        break
      case 'ArrowRight':
      case 'l':
        readerStore.nextPage()
        break
      case 'ArrowUp':
      case 'k':
        event.preventDefault()
        // 向上滚动或缩放
        break
      case 'ArrowDown':
      case 'j':
        event.preventDefault()
        // 向下滚动或缩放
        break
      case 'f':
        event.preventDefault()
        readerStore.toggleFullscreen()
        break
      case '1':
        readerStore.setReadingMode('single')
        break
      case '2':
        readerStore.setReadingMode('double')
        break
      case '=':
      case '+':
        event.preventDefault()
        readerStore.setZoomLevel(readerStore.zoomLevel + 0.1)
        break
      case '-':
        event.preventDefault()
        readerStore.setZoomLevel(readerStore.zoomLevel - 0.1)
        break
      case '0':
        readerStore.setZoomLevel(1)
        break
    }
  }

  // 鼠标滚轮控制缩放
  const handleWheel = (event: WheelEvent) => {
    if (event.ctrlKey) {
      event.preventDefault()
      const delta = event.deltaY > 0 ? -0.1 : 0.1
      readerStore.setZoomLevel(readerStore.zoomLevel + delta)
    }
  }

  onMounted(() => {
    document.addEventListener('keydown', handleKeydown)
    document.addEventListener('wheel', handleWheel, { passive: false })
  })

  onUnmounted(() => {
    document.removeEventListener('keydown', handleKeydown)
    document.removeEventListener('wheel', handleWheel)
  })

  return {
    isLoading,
    error,
    currentPageUrl,
    handleImageLoad,
    handleImageError,
    preloadNextPage
  }
}