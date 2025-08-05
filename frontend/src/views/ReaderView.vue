<template>
  <div class="reader-view fixed inset-0 z-50 bg-black flex flex-col">
    <!-- 主要阅读区域 -->
    <div class="reader-content flex-1 relative overflow-hidden">
      <!-- 点击区域 -->
      <div class="absolute inset-0 flex">
        <!-- 左侧点击区域（上一页）-->
        <div 
          class="w-1/3 h-full cursor-pointer z-10"
          @click="prevPage"
          @mouseenter="showNavHint('prev')"
          @mouseleave="hideNavHint"
        >
          <!-- 导航提示 -->
          <div 
            v-if="navHint === 'prev'" 
            class="absolute left-4 top-1/2 transform -translate-y-1/2 bg-black bg-opacity-60 text-white px-3 py-2 rounded-lg text-sm"
          >
            上一页
          </div>
        </div>
        
        <!-- 中间点击区域（显示信息面板）-->
        <div 
          class="w-1/3 h-full cursor-pointer z-10"
          @click="toggleInfoPanel"
          @mouseenter="showNavHint('info')"
          @mouseleave="hideNavHint"
        >
          <!-- 导航提示 -->
          <div 
            v-if="navHint === 'info'" 
            class="absolute left-1/2 top-1/2 transform -translate-x-1/2 -translate-y-1/2 bg-black bg-opacity-60 text-white px-3 py-2 rounded-lg text-sm"
          >
            显示详情
          </div>
        </div>
        
        <!-- 右侧点击区域（下一页）-->
        <div 
          class="w-1/3 h-full cursor-pointer z-10"
          @click="nextPage"
          @mouseenter="showNavHint('next')"
          @mouseleave="hideNavHint"
        >
          <!-- 导航提示 -->
          <div 
            v-if="navHint === 'next'" 
            class="absolute right-4 top-1/2 transform -translate-y-1/2 bg-black bg-opacity-60 text-white px-3 py-2 rounded-lg text-sm"
          >
            下一页
          </div>
        </div>
      </div>

      <!-- 漫画图片 -->
      <div class="absolute inset-0 flex justify-center items-center p-2">
        <div v-if="isLoading" class="text-white text-xl">加载中...</div>
        <div v-else-if="error" class="text-red-400 text-xl">加载失败: {{ error }}</div>
        <img
          v-else
          :src="currentPageUrl"
          :alt="`第 ${currentPage} 页`"
          class="max-w-full max-h-full object-contain"
          @load="handleImageLoad"
          @error="handleImageError"
        />
      </div>
    </div>

    <!-- 透明信息面板 -->
    <transition
      enter-active-class="transition-all duration-300 ease-out"
      enter-from-class="opacity-0 translate-y-full"
      enter-to-class="opacity-100 translate-y-0"
      leave-active-class="transition-all duration-300 ease-in"
      leave-from-class="opacity-100 translate-y-0"
      leave-to-class="opacity-0 translate-y-full"
    >
      <div
        v-if="showInfoPanel"
        class="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/80 via-black/60 to-transparent text-white px-0 py-6 z-20"
        @click.stop
      >
        <div class="px-6">
          <!-- 基本信息 -->
          <div class="flex items-start justify-between mb-4">
            <div class="flex-1">
              <h2 class="text-2xl font-bold mb-2">{{ archiveInfo?.title || '加载中...' }}</h2>
              <div class="flex items-center space-x-4 text-sm text-gray-300 mb-2">
                <span>第 {{ currentPage }} 页 / 共 {{ totalPages }} 页</span>
                <span v-if="progressData">进度: {{ (progressData.progressPercentage*100).toFixed(1) }}%</span>
                <span>{{ archiveInfo?.pageCount }} 页</span>
                <span>{{ formatFileSize(archiveInfo?.fileSize) }}</span>
              </div>
              <div class="text-sm text-gray-400">
                路径: {{ archiveInfo?.path }}
              </div>
            </div>
            
            <!-- 关闭按钮 -->
            <button
              @click="hideInfoPanel"
              class="p-2 hover:bg-white/20 rounded-lg transition-colors"
            >
              <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <!-- 标签 -->
          <div class="mb-4">
            <div class="flex items-center space-x-2 mb-2">
              <span class="text-sm font-medium">标签:</span>
              <button
                @click="showAddTagModal = true"
                class="px-2 py-1 bg-blue-600 hover:bg-blue-700 rounded text-xs transition-colors"
              >
                添加标签
              </button>
            </div>
            <div class="flex flex-wrap gap-2">
              <span
                v-for="tag in archiveInfo?.tags || []"
                :key="tag.id"
                class="px-2 py-1 bg-gray-700 hover:bg-gray-600 rounded-full text-xs cursor-pointer"
                @click="removeTag(tag.id)"
              >
                {{ tag.namespace }}:{{ tag.name }}
                <span class="ml-1 text-gray-400">×</span>
              </span>
              <span v-if="!archiveInfo?.tags?.length" class="text-gray-500 text-sm">暂无标签</span>
            </div>
          </div>

          <!-- 操作按钮 -->
          <div class="flex items-center justify-between">
            <div class="flex items-center space-x-3">
              <button
                @click="goBack"
                class="px-4 py-2 bg-gray-700 hover:bg-gray-600 rounded-lg transition-colors"
              >
                返回书库
              </button>
              <button
                @click="prevPage"
                :disabled="currentPage <= 1"
                class="px-4 py-2 bg-gray-700 hover:bg-gray-600 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg transition-colors"
              >
                上一页
              </button>
              <button
                @click="nextPage"
                :disabled="currentPage >= totalPages"
                class="px-4 py-2 bg-gray-700 hover:bg-gray-600 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg transition-colors"
              >
                下一页
              </button>
            </div>
            
            <div class="flex items-center space-x-3">
              <button
                @click="showDeleteConfirm = true"
                class="px-4 py-2 bg-red-600 hover:bg-red-700 rounded-lg transition-colors"
              >
                删除漫画
              </button>
            </div>
          </div>
        </div>
      </div>
    </transition>

    <!-- 添加标签模态框 -->
    <div
      v-if="showAddTagModal"
      class="fixed inset-0 bg-black/50 flex items-center justify-center z-30"
      @click="showAddTagModal = false"
    >
      <div
        class="bg-white rounded-lg p-6 max-w-md w-full mx-4"
        @click.stop
      >
        <h3 class="text-lg font-bold mb-4 text-gray-900">添加标签</h3>
        <div class="space-y-4">
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">命名空间</label>
            <input
              v-model="newTag.namespace"
              type="text"
              class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
              placeholder="例如: 作者, 分类, 语言"
            />
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">标签值</label>
            <input
              v-model="newTag.name"
              type="text"
              class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
              placeholder="例如: 尾田荣一郎, 少年漫画, 中文"
            />
          </div>
        </div>
        <div class="flex justify-end space-x-3 mt-6">
          <button
            @click="showAddTagModal = false"
            class="px-4 py-2 text-gray-600 hover:text-gray-800"
          >
            取消
          </button>
          <button
            @click="addTag"
            :disabled="!newTag.namespace || !newTag.name"
            class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            添加
          </button>
        </div>
      </div>
    </div>

    <!-- 删除确认模态框 -->
    <div
      v-if="showDeleteConfirm"
      class="fixed inset-0 bg-black/50 flex items-center justify-center z-30"
      @click="showDeleteConfirm = false"
    >
      <div
        class="bg-white rounded-lg p-6 max-w-md w-full mx-4"
        @click.stop
      >
        <h3 class="text-lg font-bold mb-4 text-gray-900">确认删除</h3>
        <p class="text-gray-600 mb-6">
          确定要删除这部漫画吗？此操作不可撤销。
        </p>
        <div class="flex justify-end space-x-3">
          <button
            @click="showDeleteConfirm = false"
            class="px-4 py-2 text-gray-600 hover:text-gray-800"
          >
            取消
          </button>
          <button
            @click="deleteArchive"
            class="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700"
          >
            删除
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useQuery, useMutation, useQueryClient } from '@tanstack/vue-query'
import { getArchive, getProgress, updateProgress, removeTagFromArchive, getArchivePage } from '@/utils/api'
import type { Archive, Tag, ReadingProgress } from '@/types/api'

const route = useRoute()
const router = useRouter()
const queryClient = useQueryClient()

const archiveId = computed(() => route.params.id as string)
const currentPage = ref(1)
const totalPages = ref(1)
const isLoading = ref(false)
const error = ref<string | null>(null)
const currentPageUrl = ref<string | null>(null)

// 信息面板相关状态
const showInfoPanel = ref(false)
const navHint = ref<string | null>(null)
const autoHideTimeout = ref<NodeJS.Timeout | null>(null)

// 模态框状态
const showAddTagModal = ref(false)
const showDeleteConfirm = ref(false)

// 新标签数据
const newTag = ref({
  namespace: '',
  name: ''
})

// 获取漫画信息
const { data: archiveInfo, isLoading: isArchiveLoading } = useQuery({
  queryKey: computed(() => ['archive', archiveId.value]),
  queryFn: () => getArchive(archiveId.value),
  enabled: computed(() => !!archiveId.value)
})

// 获取阅读进度
const { data: progressData, isLoading: isProgressLoading } = useQuery({
  queryKey: computed(() => ['progress', archiveId.value]),
  queryFn: () => getProgress(archiveId.value),
  enabled: computed(() => !!archiveId.value),
  retry: false // 如果没有进度记录，不重试
})

// 更新进度的mutation
const updateProgressMutation = useMutation({
  mutationFn: ({ archiveId, currentPage }: { archiveId: string, currentPage: number }) =>
    updateProgress(archiveId, { currentPage }),
  onSuccess: () => {
    // 刷新进度数据
    queryClient.invalidateQueries({ queryKey: ['progress', archiveId.value] })
  },
  onError: (error) => {
    console.error('Failed to update progress:', error)
  }
})

// 监听漫画信息变化，更新总页数
watch(archiveInfo, (newInfo) => {
  console.log('archiveInfo watch triggered:', {
    newInfo: newInfo ? { id: newInfo.id, title: newInfo.title, pageCount: newInfo.pageCount } : null,
    totalPagesBefore: totalPages.value,
    currentPage: currentPage.value
  })
  if (newInfo) {
    totalPages.value = newInfo.pageCount
    console.log('Updated totalPages to:', newInfo.pageCount)
  }
}, { immediate: true })

// 加载当前页面图片
const loadCurrentPage = async () => {
  console.log('loadCurrentPage called:', {
    archiveId: archiveId.value,
    currentPage: currentPage.value,
    totalPages: totalPages.value
  })
  
  if (!archiveId.value) {
    console.log('No archiveId, returning')
    return
  }
  
  try {
    isLoading.value = true
    error.value = null
    
    console.log('Calling getArchivePage with:', archiveId.value, currentPage.value)
    const pageUrl = await getArchivePage(archiveId.value, currentPage.value)
    console.log('Got page URL:', pageUrl)
    currentPageUrl.value = pageUrl
  } catch (err: any) {
    console.error('Failed to load page:', err)
    error.value = err.response?.data?.message || err.message || '加载页面失败'
    currentPageUrl.value = null
  } finally {
    isLoading.value = false
  }
}

// 监听进度数据变化，恢复阅读位置
watch(progressData, (newProgress) => {
  console.log('progressData watch triggered:', {
    newProgress,
    currentPageBefore: currentPage.value,
    archiveId: archiveId.value
  })
  if (newProgress && newProgress.currentPage > 0) {
    const newPage = newProgress.currentPage
    currentPage.value = newPage
    console.log('Set currentPage from progress:', newPage)
  } else if (newProgress && newProgress.currentPage === 0) {
    // 进度为0的书籍，从第1页开始
    console.log('Progress is 0, setting currentPage to 1')
    currentPage.value = 1
  } else if (!newProgress && currentPage.value <= 0) {
    // 如果没有进度数据且当前页面未设置，默认从第1页开始
    currentPage.value = 1
    console.log('No progress data, setting currentPage to 1')
  }
}, { immediate: true })

// 监听当前页码变化，自动加载页面
watch(currentPage, (newPage, oldPage) => {
  console.log('currentPage watch triggered:', {
    newPage,
    oldPage,
    archiveId: archiveId.value,
    totalPages: totalPages.value
  })
  if (currentPage.value > 0) {
    loadCurrentPage()
  } else {
    console.log('currentPage <= 0, not loading page')
  }
}, { immediate: true }) // 添加immediate: true确保初始值也会触发

// 导航方法
const goBack = () => {
  router.push('/library')
}

const prevPage = () => {
  if (currentPage.value > 1) {
    currentPage.value--
    hideInfoPanel()
    // 自动保存进度
    saveProgress()
  }
}

const nextPage = () => {
  if (currentPage.value < totalPages.value) {
    currentPage.value++
    hideInfoPanel()
    // 自动保存进度
    saveProgress()
  }
}

// 保存阅读进度
const saveProgress = () => {
  if (archiveId.value && currentPage.value > 0) {
    updateProgressMutation.mutate({
      archiveId: archiveId.value,
      currentPage: currentPage.value
    })
    
    // 如果阅读超过第一页，自动移除"new"标签
    removeNewTagIfNeeded()
  }
}

// 移除"new"标签（如果存在且已读超过第一页）
const removeNewTagIfNeeded = async () => {
  if (currentPage.value > 1 && archiveInfo.value) {
    const newTag = archiveInfo.value.tags?.find(tag => 
      tag.name === 'new' && tag.namespace === 'system'
    )
    
    if (newTag) {
      try {
        await removeTagFromArchive(archiveId.value, newTag.id)
        // 刷新漫画信息以更新标签列表
        queryClient.invalidateQueries({ queryKey: ['archive', archiveId.value] })
      } catch (error) {
        console.error('Failed to remove new tag:', error)
      }
    }
  }
}

// 信息面板控制
const toggleInfoPanel = () => {
  if (showInfoPanel.value) {
    hideInfoPanel()
  } else {
    showInfoPanelWithAutoHide()
  }
}

const showInfoPanelWithAutoHide = () => {
  showInfoPanel.value = true
  setAutoHideTimer()
}

const hideInfoPanel = () => {
  showInfoPanel.value = false
  clearAutoHideTimer()
}

const setAutoHideTimer = () => {
  clearAutoHideTimer()
  autoHideTimeout.value = setTimeout(() => {
    hideInfoPanel()
  }, 5000) // 5秒后自动隐藏
}

const clearAutoHideTimer = () => {
  if (autoHideTimeout.value) {
    clearTimeout(autoHideTimeout.value)
    autoHideTimeout.value = null
  }
}

// 导航提示
const showNavHint = (type: string) => {
  navHint.value = type
}

const hideNavHint = () => {
  navHint.value = null
}

// 标签管理
const addTag = async () => {
  if (!newTag.value.namespace || !newTag.value.name) return
  
  try {
    // TODO: 调用API添加标签
    console.log('Adding tag:', newTag.value)
    
    // 重置表单
    newTag.value = { namespace: '', name: '' }
    showAddTagModal.value = false
    
    // 刷新漫画信息
    queryClient.invalidateQueries({ queryKey: ['archive', archiveId.value] })
  } catch (error) {
    console.error('Failed to add tag:', error)
  }
}

const removeTag = async (tagId: number) => {
  try {
    // TODO: 调用API删除标签
    console.log('Removing tag:', tagId)
    
    // 刷新漫画信息
    queryClient.invalidateQueries({ queryKey: ['archive', archiveId.value] })
  } catch (error) {
    console.error('Failed to remove tag:', error)
  }
}

// 删除漫画
const deleteArchive = async () => {
  try {
    // TODO: 调用API删除漫画
    console.log('Deleting archive:', archiveId.value)
    
    showDeleteConfirm.value = false
    router.push('/library')
  } catch (error) {
    console.error('Failed to delete archive:', error)
  }
}

// 工具方法
const formatFileSize = (bytes?: number): string => {
  if (!bytes) return '未知'
  
  const units = ['B', 'KB', 'MB', 'GB']
  let size = bytes
  let unitIndex = 0
  
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024
    unitIndex++
  }
  
  return `${size.toFixed(1)} ${units[unitIndex]}`
}

// 图片加载处理
const handleImageLoad = () => {
  isLoading.value = false
  error.value = null
}

const handleImageError = () => {
  isLoading.value = false
  error.value = '图片加载失败'
}

// 键盘事件处理
const handleKeydown = (event: KeyboardEvent) => {
  switch (event.key) {
    case 'ArrowLeft':
      prevPage()
      break
    case 'ArrowRight':
      nextPage()
      break
    case 'Escape':
      if (showInfoPanel.value) {
        hideInfoPanel()
      } else {
        goBack()
      }
      break
    case ' ':
      event.preventDefault()
      toggleInfoPanel()
      break
  }
}

// 初始化阅读器状态
const initializeReader = async () => {
  if (!archiveId.value) return
  
  console.log('initializeReader called:', {
    archiveId: archiveId.value,
    progressData: progressData.value,
    currentPage: currentPage.value,
    isProgressLoading: isProgressLoading.value
  })
  
  // 只重置加载状态，不重置currentPage
  // currentPage应该由progressData watch来管理
  isLoading.value = true
  error.value = null
  
  console.log('Initialized reader without resetting currentPage')
}

// 监听archiveId变化
watch(archiveId, (newArchiveId, oldArchiveId) => {
  if (newArchiveId) {
    console.log('archiveId changed:', { newArchiveId, oldArchiveId })
    // 只有当真正切换书籍时才重置currentPage（排除首次加载的情况）
    if (oldArchiveId !== undefined && newArchiveId !== oldArchiveId) {
      currentPage.value = 0
      console.log('Reset currentPage to 0 due to archiveId change')
    }
    initializeReader()
  }
}, { immediate: true })

onMounted(() => {
  // 添加键盘事件监听
  document.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  // 保存当前进度
  saveProgress()
  // 清理事件监听和定时器
  document.removeEventListener('keydown', handleKeydown)
  clearAutoHideTimer()
})
</script>