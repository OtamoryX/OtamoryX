<template>
  <div
    v-if="show"
    class="fixed inset-0 z-50"
    @click="$emit('close')"
    @contextmenu.prevent="$emit('close')"
    @touchstart="$emit('close')"
  >
    <!-- 右键菜单 -->
    <div
      ref="menuRef"
      :style="{ left: `${position.x}px`, top: `${position.y}px` }"
      class="absolute bg-black/90 backdrop-blur-md border border-white/20 rounded-lg shadow-2xl py-2 min-w-48 z-50 touch-manipulation select-none"
      @click.stop
    >
      <!-- 漫画信息 -->
      <div v-if="archive" class="px-4 py-2 border-b border-white/20">
        <div class="text-white font-medium truncate">{{ archive.title }}</div>
        <div class="text-white/60 text-sm">{{ archive.pageCount }} 页</div>
      </div>

      <!-- 菜单项 -->
      <div class="py-1">
        <!-- 添加标签 -->
        <button
          class="w-full px-4 py-3 text-left text-white hover:bg-white/10 active:bg-white/20 transition-colors flex items-center touch-manipulation"
          @click="$emit('add-tag')"
        >
          <svg
            class="w-4 h-4 mr-3 text-blue-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z"
            />
          </svg>
          添加标签
        </button>

        <!-- 添加到分类 -->
        <button
          class="w-full px-4 py-3 text-left text-white hover:bg-white/10 active:bg-white/20 transition-colors flex items-center touch-manipulation"
          @click="toggleCategorySubmenu"
        >
          <svg
            class="w-4 h-4 mr-3 text-purple-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
            />
          </svg>
          添加到分类
          <svg
            :class="['w-4 h-4 ml-auto transition-transform', showCategorySubmenu ? 'rotate-90' : '']"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M9 5l7 7-7 7"
            />
          </svg>
        </button>

        <!-- 分类子菜单 -->
        <div
          v-if="showCategorySubmenu"
          class="ml-4 border-l border-white/20 pl-2"
        >
          <div v-if="isLoadingCategories" class="px-4 py-2 text-white/60 text-sm">
            加载中...
          </div>
          <div v-else-if="staticCategories.length === 0" class="px-4 py-2 text-white/60 text-sm">
            暂无静态分类
          </div>
          <div v-else>
            <button
              v-for="category in staticCategories"
              :key="category.id"
              class="w-full px-4 py-2 text-left text-sm transition-colors touch-manipulation flex items-center justify-between"
              :class="isArchiveInCategory(category.id) 
                ? 'text-green-300 hover:bg-green-500/10 active:bg-green-500/20' 
                : 'text-white/80 hover:bg-white/5 active:bg-white/10'"
              @click="handleCategoryAction(category.id)"
            >
              <span class="flex items-center">
                <svg
                  v-if="isArchiveInCategory(category.id)"
                  class="w-3 h-3 mr-2 text-green-400"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M5 13l4 4L19 7"
                  />
                </svg>
                {{ category.name }}
              </span>
              <span class="text-xs opacity-60">
                {{ isArchiveInCategory(category.id) ? '移出' : '添加' }}
              </span>
            </button>
          </div>
        </div>

        <!-- 分隔线 -->
        <div class="my-1 border-t border-white/20"></div>

        <!-- 删除漫画 -->
        <button
          class="w-full px-4 py-3 text-left text-red-400 hover:bg-red-500/10 hover:text-red-300 active:bg-red-500/20 transition-colors flex items-center touch-manipulation"
          @click="$emit('delete-archive')"
        >
          <svg
            class="w-4 h-4 mr-3"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
            />
          </svg>
          删除漫画
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, nextTick, watch } from 'vue'
import { useQuery } from '@tanstack/vue-query'
import { getCategories, getArchiveCategories } from '@/utils/api'
import type { Archive, Category } from '@/types/api'

interface Props {
  show: boolean
  archive: Archive | null
  position: { x: number; y: number }
}

const props = defineProps<Props>()

const emit = defineEmits<{
  close: []
  'add-tag': []
  'add-to-category': [categoryId: string]
  'remove-from-category': [categoryId: string]
  'delete-archive': []
}>()

const menuRef = ref<HTMLElement>()
const showCategorySubmenu = ref(false)
const archiveCategoryIds = ref<string[]>([])

// 获取分类数据
const { data: categories, isLoading: isLoadingCategories } = useQuery({
  queryKey: ['categories'],
  queryFn: getCategories,
})

// 获取档案所属分类数据
const { data: archiveCategories, isLoading: isLoadingArchiveCategories } = useQuery({
  queryKey: ['archiveCategories', computed(() => props.archive?.id)],
  queryFn: () => getArchiveCategories(props.archive!.id),
  enabled: computed(() => props.show && !!props.archive?.id),
})

// 静态分类
const staticCategories = computed(() => {
  return categories.value?.filter(cat => cat.isStatic) || []
})

const toggleCategorySubmenu = () => {
  showCategorySubmenu.value = !showCategorySubmenu.value
}

// 检查档案是否在指定分类中
const isArchiveInCategory = (categoryId: string): boolean => {
  return archiveCategories.value?.includes(categoryId) ?? false
}

// 处理分类操作（添加或移出）
const handleCategoryAction = (categoryId: string) => {
  if (isArchiveInCategory(categoryId)) {
    emit('remove-from-category', categoryId)
  } else {
    emit('add-to-category', categoryId)
  }
}

// 调整菜单位置，确保不超出屏幕边界
const adjustMenuPosition = async () => {
  if (!menuRef.value) return

  await nextTick()

  const menu = menuRef.value
  const rect = menu.getBoundingClientRect()
  const windowWidth = window.innerWidth
  const windowHeight = window.innerHeight

  let { x, y } = props.position

  // 确保菜单不超出右边界
  if (x + rect.width > windowWidth) {
    x = windowWidth - rect.width - 10
  }

  // 确保菜单不超出下边界
  if (y + rect.height > windowHeight) {
    y = windowHeight - rect.height - 10
  }

  // 确保菜单不超出上边界和左边界
  x = Math.max(10, x)
  y = Math.max(10, y)

  menu.style.left = `${x}px`
  menu.style.top = `${y}px`
}

// 监听显示状态变化，调整位置
watch(() => props.show, (show) => {
  if (show) {
    showCategorySubmenu.value = false
    adjustMenuPosition()
  }
})

// 监听分类子菜单展开，重新调整位置
watch(showCategorySubmenu, () => {
  adjustMenuPosition()
})

onMounted(() => {
  if (props.show) {
    adjustMenuPosition()
  }
})
</script>