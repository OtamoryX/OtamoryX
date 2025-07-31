import { ref, onMounted, onUnmounted } from 'vue'

export const useInfiniteScroll = (callback: () => void, threshold = 100) => {
  const isLoading = ref(false)
  const element = ref<HTMLElement>()

  const handleScroll = () => {
    if (isLoading.value || !element.value) return

    const { scrollTop, scrollHeight, clientHeight } = element.value
    
    if (scrollHeight - scrollTop - clientHeight < threshold) {
      isLoading.value = true
      callback()
    }
  }

  const finishLoading = () => {
    isLoading.value = false
  }

  onMounted(() => {
    element.value?.addEventListener('scroll', handleScroll)
  })

  onUnmounted(() => {
    element.value?.removeEventListener('scroll', handleScroll)
  })

  return {
    element,
    isLoading,
    finishLoading
  }
}