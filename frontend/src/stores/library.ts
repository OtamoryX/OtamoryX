import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

const CATEGORY_STORAGE_KEY = 'selected-category'
const CAROUSEL_STORAGE_KEY = 'show-carousel'

export const useLibraryStore = defineStore('library', () => {
  // 状态
  const selectedCategoryId = ref<string | null>(null)
  const showMobileSearch = ref(false)
  const showCarousel = ref(true)

  // 初始化：从 localStorage 读取
  const initFromStorage = () => {
    try {
      const savedCategory = localStorage.getItem(CATEGORY_STORAGE_KEY)
      if (savedCategory && savedCategory !== 'null') {
        selectedCategoryId.value = savedCategory
      }

      const savedCarousel = localStorage.getItem(CAROUSEL_STORAGE_KEY)
      if (savedCarousel !== null) {
        showCarousel.value = savedCarousel === 'true'
      }
    } catch (error) {
      console.error('Failed to load library state from localStorage:', error)
    }
  }

  // Actions
  const selectCategory = (categoryId: string | null) => {
    selectedCategoryId.value = categoryId
    try {
      localStorage.setItem(CATEGORY_STORAGE_KEY, categoryId || 'null')
    } catch (error) {
      console.error('Failed to save category to localStorage:', error)
    }
  }

  const toggleMobileSearch = () => {
    showMobileSearch.value = !showMobileSearch.value
  }

  const setShowCarousel = (show: boolean) => {
    showCarousel.value = show
    try {
      localStorage.setItem(CAROUSEL_STORAGE_KEY, String(show))
    } catch (error) {
      console.error('Failed to save carousel state to localStorage:', error)
    }
  }

  // Getters
  const hasSelectedCategory = computed(() => selectedCategoryId.value !== null)

  // 初始化
  initFromStorage()

  return {
    // 状态
    selectedCategoryId,
    showMobileSearch,
    showCarousel,
    // Actions
    selectCategory,
    toggleMobileSearch,
    setShowCarousel,
    // Getters
    hasSelectedCategory
  }
})
