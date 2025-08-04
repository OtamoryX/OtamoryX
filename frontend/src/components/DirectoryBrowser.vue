<template>
  <div class="directory-browser">
    <!-- 模态框遮罩 -->
    <div 
      v-if="isOpen" 
      class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50"
      @click="closeModal"
    >
      <!-- 模态框内容 -->
      <div 
        class="bg-white rounded-lg shadow-xl w-full max-w-2xl max-h-[80vh] flex flex-col"
        @click.stop
      >
        <!-- 标题栏 -->
        <div class="flex items-center justify-between p-4 border-b">
          <h2 class="text-lg font-semibold text-gray-900">选择目录</h2>
          <button
            @click="closeModal"
            class="text-gray-400 hover:text-gray-600 transition-colors"
          >
            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <!-- 当前路径显示 -->
        <div class="px-4 py-2 bg-gray-50 border-b">
          <div class="flex items-center space-x-2 text-sm text-gray-600">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2H5a2 2 0 00-2-2v0" />
            </svg>
            <span class="font-medium">当前路径:</span>
            <span class="font-mono bg-white px-2 py-1 rounded border">{{ currentPath || '/' }}</span>
          </div>
        </div>

        <!-- 目录列表 -->
        <div class="flex-1 overflow-y-auto p-4">
          <!-- 加载状态 -->
          <div v-if="loading" class="flex items-center justify-center py-8">
            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
            <span class="ml-2 text-gray-600">加载中...</span>
          </div>

          <!-- 错误状态 -->
          <div v-else-if="error" class="text-center py-8">
            <div class="text-red-600 mb-2">
              <svg class="w-12 h-12 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 15.5c-.77.833.192 2.5 1.732 2.5z" />
              </svg>
            </div>
            <p class="text-red-600 font-medium">加载失败</p>
            <p class="text-gray-500 text-sm mt-1">{{ error }}</p>
            <button
              @click="refreshDirectory"
              class="mt-3 px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
            >
              重试
            </button>
          </div>

          <!-- 目录列表 -->
          <div v-else class="space-y-1">
            <!-- 返回上级目录 -->
            <button
              v-if="parentPath"
              @click="navigateToParent"
              class="w-full flex items-center p-3 text-left hover:bg-gray-100 rounded-lg transition-colors group"
            >
              <svg class="w-5 h-5 text-gray-400 group-hover:text-blue-600 mr-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 19l-7-7 7-7m8 14l-7-7 7-7" />
              </svg>
              <span class="text-gray-600 group-hover:text-blue-600 font-medium">..</span>
              <span class="text-gray-400 ml-2">(返回上级)</span>
            </button>

            <!-- 目录项 -->
            <div
              v-for="directory in directories"
              :key="directory.path"
              class="directory-item"
            >
              <button
                @click="navigateToDirectory(directory.path)"
                :disabled="!directory.is_accessible"
                class="w-full flex items-center p-3 text-left rounded-lg transition-colors"
                :class="{
                  'hover:bg-gray-100 cursor-pointer': directory.is_accessible,
                  'opacity-50 cursor-not-allowed': !directory.is_accessible,
                  'bg-blue-50 border border-blue-200': selectedPath === directory.path
                }"
              >
                <svg 
                  class="w-5 h-5 mr-3"
                  :class="{
                    'text-blue-600': directory.is_accessible,
                    'text-gray-400': !directory.is_accessible
                  }"
                  fill="none" 
                  stroke="currentColor" 
                  viewBox="0 0 24 24"
                >
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2H5a2 2 0 00-2-2v0" />
                </svg>
                <span 
                  :class="{
                    'text-gray-900': directory.is_accessible,
                    'text-gray-400': !directory.is_accessible
                  }"
                >
                  {{ directory.name }}
                </span>
                <span 
                  v-if="!directory.is_accessible"
                  class="ml-auto text-xs text-gray-400"
                >
                  无权限
                </span>
              </button>
            </div>

            <!-- 空目录提示 -->
            <div v-if="directories.length === 0" class="text-center py-8 text-gray-500">
              <svg class="w-12 h-12 mx-auto mb-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2H5a2 2 0 00-2-2v0" />
              </svg>
              <p>此目录中没有子目录</p>
            </div>
          </div>
        </div>

        <!-- 底部操作栏 -->
        <div class="flex items-center justify-between p-4 border-t bg-gray-50">
          <div class="flex items-center space-x-2 text-sm text-gray-600">
            <span>已选择:</span>
            <span class="font-mono bg-white px-2 py-1 rounded border max-w-md truncate">
              {{ selectedPath || '未选择' }}
            </span>
          </div>
          <div class="flex space-x-2">
            <button
              @click="closeModal"
              class="px-4 py-2 text-gray-600 hover:text-gray-800 transition-colors"
            >
              取消
            </button>
            <button
              @click="selectCurrentPath"
              :disabled="!currentPath"
              class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              选择当前目录
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { getDirectories } from '@/utils/api'

interface DirectoryInfo {
  name: string
  path: string
  is_accessible: boolean
}

interface DirectoryListResponse {
  current_path: string
  parent_path: string | null
  directories: DirectoryInfo[]
}

interface Props {
  isOpen: boolean
  initialPath?: string
}

interface Emits {
  (e: 'close'): void
  (e: 'select', path: string): void
}

const props = withDefaults(defineProps<Props>(), {
  initialPath: ''
})

const emit = defineEmits<Emits>()

// 响应式数据
const loading = ref(false)
const error = ref<string | null>(null)
const currentPath = ref('')
const parentPath = ref<string | null>(null)
const directories = ref<DirectoryInfo[]>([])
const selectedPath = ref('')

// 监听模态框打开状态
watch(() => props.isOpen, (isOpen) => {
  if (isOpen) {
    // 只有绝对路径才传递给后端，相对路径或空路径使用默认行为
    const pathToLoad = props.initialPath && props.initialPath.startsWith('/') ? props.initialPath : ''
    loadDirectory(pathToLoad)
    selectedPath.value = props.initialPath
  }
})

// 加载目录
const loadDirectory = async (path: string = '') => {
  loading.value = true
  error.value = null
  
  try {
    const response = await getDirectories(path || undefined)
    
    currentPath.value = response.current_path
    parentPath.value = response.parent_path
    directories.value = response.directories
  } catch (err: any) {
    console.error('Failed to load directory:', err)
    error.value = err.response?.data?.message || err.message || '加载目录失败'
  } finally {
    loading.value = false
  }
}

// 导航到父目录
const navigateToParent = () => {
  if (parentPath.value) {
    loadDirectory(parentPath.value)
  }
}

// 导航到指定目录
const navigateToDirectory = (path: string) => {
  loadDirectory(path)
}

// 刷新当前目录
const refreshDirectory = () => {
  loadDirectory(currentPath.value)
}

// 选择当前路径
const selectCurrentPath = () => {
  if (currentPath.value) {
    emit('select', currentPath.value)
    closeModal()
  }
}

// 关闭模态框
const closeModal = () => {
  emit('close')
}
</script>

<style scoped>
.directory-browser {
  /* 确保模态框在最顶层 */
  z-index: 1000;
}

.directory-item:hover {
  transform: translateX(2px);
}

/* 滚动条样式 */
.overflow-y-auto::-webkit-scrollbar {
  width: 6px;
}

.overflow-y-auto::-webkit-scrollbar-track {
  background: #f1f1f1;
  border-radius: 3px;
}

.overflow-y-auto::-webkit-scrollbar-thumb {
  background: #c1c1c1;
  border-radius: 3px;
}

.overflow-y-auto::-webkit-scrollbar-thumb:hover {
  background: #a8a8a8;
}
</style>