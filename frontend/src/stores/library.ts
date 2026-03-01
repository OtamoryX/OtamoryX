import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

const CATEGORY_STORAGE_KEY = 'selected-category'
const CAROUSEL_STORAGE_KEY = 'show-carousel'
const ROWS_PER_PAGE_STORAGE_KEY = 'rows-per-page'

export const useLibraryStore = defineStore('library', () => {
  // 状态
  const selectedCategoryId = ref<string | null>(null)
  const showMobileSearch = ref(false)
  const showCarousel = ref(true)
  const rowsPerPage = ref(5)

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

      const savedRows = localStorage.getItem(ROWS_PER_PAGE_STORAGE_KEY)
      if (savedRows !== null) {
        const parsed = Number(savedRows)
        if (parsed >= 3 && parsed <= 10) {
          rowsPerPage.value = parsed
        }
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

  const setRowsPerPage = (rows: number) => {
    if (rows >= 3 && rows <= 10) {
      rowsPerPage.value = rows
      try {
        localStorage.setItem(ROWS_PER_PAGE_STORAGE_KEY, String(rows))
      } catch (error) {
        console.error('Failed to save rows per page to localStorage:', error)
      }
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
    rowsPerPage,
    // Actions
    selectCategory,
    toggleMobileSearch,
    setShowCarousel,
    setRowsPerPage,
    // Getters
    hasSelectedCategory
  }
})
