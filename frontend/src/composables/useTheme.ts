import { ref, watch, onMounted } from 'vue'

type Theme = 'light' | 'dark' | 'system'

const STORAGE_KEY = 'theme'

export function useTheme() {
  const theme = ref<Theme>('system')

  // 获取实际应用的主题（解析 system）
  const getEffectiveTheme = (): 'light' | 'dark' => {
    if (theme.value === 'system') {
      return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
    }
    return theme.value
  }

  // 应用主题到 DOM
  const applyTheme = () => {
    const effectiveTheme = getEffectiveTheme()
    document.documentElement.dataset.theme = effectiveTheme
  }

  // 设置主题
  const setTheme = (newTheme: Theme) => {
    theme.value = newTheme
    localStorage.setItem(STORAGE_KEY, newTheme)
    applyTheme()
  }

  // 初始化
  onMounted(() => {
    // 从 localStorage 读取
    const saved = localStorage.getItem(STORAGE_KEY) as Theme | null
    if (saved && ['light', 'dark', 'system'].includes(saved)) {
      theme.value = saved
    }

    // 应用主题
    applyTheme()

    // 监听系统主题变化
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
    const handleChange = () => {
      if (theme.value === 'system') {
        applyTheme()
      }
    }
    mediaQuery.addEventListener('change', handleChange)

    // 清理
    return () => mediaQuery.removeEventListener('change', handleChange)
  })

  // 监听主题变化
  watch(theme, applyTheme)

  return {
    theme,
    setTheme,
    effectiveTheme: getEffectiveTheme
  }
}
