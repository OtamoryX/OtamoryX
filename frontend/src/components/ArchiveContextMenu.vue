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
      class="absolute bg-[var(--bg-card)]/95 backdrop-blur-xl border border-[var(--border)] rounded-xl shadow-2xl py-2 w-[320px] max-w-[calc(100vw-20px)] max-h-[85vh] overflow-y-auto z-50 touch-manipulation select-none"
      @click.stop
    >
      <!-- 漫画信息 -->
      <div
        v-if="archive"
        class="px-4 py-3 border-b border-[var(--border)] bg-[var(--bg-primary)]/55"
      >
        <div
          class="text-[var(--text-primary)] text-sm font-medium leading-snug whitespace-normal break-words [overflow-wrap:anywhere]"
        >
          {{ archive.title }}
        </div>
        <div class="text-[var(--text-secondary)] text-xs mt-1">
          {{ archive.pageCount }} 页
        </div>
      </div>

      <!-- 菜单项 -->
      <div class="py-1">
        <!-- 阅读 -->
        <button
          class="w-full px-4 py-2.5 text-left text-[var(--text-primary)] hover:bg-[var(--bg-tertiary)] active:bg-[var(--bg-tertiary)]/80 transition-colors flex items-center touch-manipulation"
          @click="$emit('open-reader-new-tab')"
        >
          <svg
            class="w-4 h-4 mr-3 text-[var(--accent)]"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M14 3h7m0 0v7m0-7L10 14"
            />
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M5 5h6M5 5v14h14v-6"
            />
          </svg>
          阅读
        </button>

        <!-- 编辑元信息 -->
        <button
          class="w-full px-4 py-2.5 text-left text-[var(--text-primary)] hover:bg-[var(--bg-tertiary)] active:bg-[var(--bg-tertiary)]/80 transition-colors flex items-center touch-manipulation"
          @click="$emit('edit-metadata')"
        >
          <svg
            class="w-4 h-4 mr-3 text-[var(--accent)]"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
            />
          </svg>
          编辑元信息
        </button>

        <!-- 分隔线 -->
        <div class="my-1 border-t border-[var(--border)]"></div>

        <!-- 添加标签 -->
        <button
          class="w-full px-4 py-2.5 text-left text-[var(--text-primary)] hover:bg-[var(--bg-tertiary)] active:bg-[var(--bg-tertiary)]/80 transition-colors flex items-center touch-manipulation"
          @click="$emit('add-tag')"
        >
          <svg
            class="w-4 h-4 mr-3 text-[var(--accent)]"
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
          class="w-full px-4 py-2.5 text-left text-[var(--text-primary)] hover:bg-[var(--bg-tertiary)] active:bg-[var(--bg-tertiary)]/80 transition-colors flex items-center touch-manipulation"
          @click="toggleCategorySubmenu"
        >
          <svg
            class="w-4 h-4 mr-3 text-[var(--accent)]"
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
            :class="[
              'w-4 h-4 ml-auto transition-transform text-[var(--text-secondary)]',
              showCategorySubmenu ? 'rotate-90' : ''
            ]"
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
          class="mx-2 mt-1 rounded-lg border border-[var(--border)] bg-[var(--bg-primary)]/60 p-1"
        >
          <div v-if="isLoadingCategories" class="px-3 py-2 text-[var(--text-tertiary)] text-sm">
            加载中...
          </div>
          <div
            v-else-if="staticCategories.length === 0"
            class="px-3 py-2 text-[var(--text-tertiary)] text-sm"
          >
            暂无静态分类
          </div>
          <div v-else>
            <button
              v-for="category in staticCategories"
              :key="category.id"
              class="w-full px-3 py-2 text-left text-sm transition-colors touch-manipulation flex items-center justify-between rounded-md"
              :class="isArchiveInCategory(category.id) 
                ? 'text-emerald-300 hover:bg-emerald-500/12 active:bg-emerald-500/20' 
                : 'text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] active:bg-[var(--bg-tertiary)]/80'"
              @click="handleCategoryAction(category.id)"
            >
              <span class="flex items-center">
                <svg
                  v-if="isArchiveInCategory(category.id)"
                  class="w-3 h-3 mr-2 text-emerald-400"
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
              <span class="text-xs opacity-70">
                {{ isArchiveInCategory(category.id) ? '移出' : '添加' }}
              </span>
            </button>
          </div>
        </div>

        <!-- 分隔线 -->
        <div class="my-1 border-t border-[var(--border)]"></div>

        <!-- 删除漫画 -->
        <button
          class="w-full px-4 py-2.5 text-left text-red-400 hover:bg-red-500/10 hover:text-red-300 active:bg-red-500/20 transition-colors flex items-center touch-manipulation"
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
  'open-reader-new-tab': []
  'edit-metadata': []
  'add-tag': []
  'add-to-category': [categoryId: string]
  'remove-from-category': [categoryId: string]
  'delete-archive': []
}>()

const menuRef = ref<HTMLElement>()
const showCategorySubmenu = ref(false)

// 获取分类数据
const { data: categories, isLoading: isLoadingCategories } = useQuery({
  queryKey: ['categories'],
  queryFn: getCategories,
})

// 获取档案所属分类数据
const { data: archiveCategories } = useQuery({
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
  await nextTick()
  // 等待菜单 DOM 完整渲染后再读取尺寸，否则首次打开时可能拿不到宽高
  await new Promise((resolve) => requestAnimationFrame(() => resolve(null)))

  const menu = menuRef.value
  if (!menu) return
  const rect = menu.getBoundingClientRect()
  const windowWidth = window.innerWidth
  const windowHeight = window.innerHeight

  const margin = 10
  let { x, y } = props.position

  // 将菜单坐标夹紧在可视区域内
  const maxX = Math.max(margin, windowWidth - rect.width - margin)
  const maxY = Math.max(margin, windowHeight - rect.height - margin)
  x = Math.min(Math.max(margin, x), maxX)
  y = Math.min(Math.max(margin, y), maxY)

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

watch(
  () => props.position,
  () => {
    if (props.show) {
      adjustMenuPosition()
    }
  },
  { deep: true }
)

onMounted(() => {
  if (props.show) {
    adjustMenuPosition()
  }
})
</script>
