<template>
  <div :class="[
    'category-sidebar bg-white border-r border-gray-200 h-full overflow-y-auto transition-all duration-300',
    isCollapsed ? 'w-16' : 'w-64'
  ]">
    <!-- 分类标题 -->
    <div class="p-4 border-b border-gray-200">
      <div v-if="!isCollapsed" class="flex items-center justify-between">
        <h2 class="text-lg font-semibold text-gray-900">分类</h2>
        <div class="flex items-center space-x-2">
          <button
            @click="$emit('create-category')"
            class="p-1 text-gray-400 hover:text-blue-600"
            title="创建分类"
          >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
            </svg>
          </button>
          <button
            @click="toggleCollapse"
            class="p-1 text-gray-400 hover:text-blue-600"
            title="折叠侧边栏"
          >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
            </svg>
          </button>
        </div>
      </div>
      <!-- 折叠状态下的按钮 -->
      <div v-else class="flex flex-col items-center space-y-3">
        <button
          @click="$emit('create-category')"
          class="p-2 w-10 h-10 text-gray-400 hover:text-blue-600 hover:bg-blue-50 rounded-lg transition-colors"
          title="创建分类"
        >
          <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
          </svg>
        </button>
        <button
          @click="toggleCollapse"
          class="p-2 w-10 h-10 text-gray-400 hover:text-blue-600 hover:bg-blue-50 rounded-lg transition-colors"
          title="展开侧边栏"
        >
          <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
          </svg>
        </button>
      </div>
    </div>

    <!-- 全部漫画 -->
    <div class="p-2">
      <button
        @click="selectCategory(null)"
        :class="[
          'w-full text-left px-3 py-2 rounded-lg transition-colors',
          !selectedCategoryId ? 'bg-blue-50 text-blue-700 border border-blue-200' : 'hover:bg-gray-50'
        ]"
        :title="isCollapsed ? `全部漫画 (${totalArchives})` : ''"
      >
        <div v-if="!isCollapsed" class="flex items-center justify-between">
          <span class="font-medium">全部漫画</span>
          <span class="text-sm text-gray-500">{{ totalArchives }}</span>
        </div>
        <div v-else class="flex justify-center">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
          </svg>
        </div>
      </button>
    </div>

    <!-- 加载状态 -->
    <div v-if="isLoading" class="p-4 text-center">
      <div class="text-gray-500 text-sm">加载中...</div>
    </div>

    <!-- 错误状态 -->
    <div v-else-if="error" class="p-4">
      <div class="text-red-600 text-sm">加载失败</div>
    </div>

    <!-- 分类列表 -->
    <div v-else class="p-2 space-y-1">
      <!-- 静态分类 -->
      <div v-if="staticCategories.length > 0">
        <div v-if="!isCollapsed" class="px-3 py-2 text-xs font-semibold text-gray-500 uppercase tracking-wider">
          静态分类
        </div>
        <div v-for="category in staticCategories" :key="category.id" class="space-y-1">
          <button
            @click="selectCategory(category.id)"
            :class="[
              'w-full text-left px-3 py-2 rounded-lg transition-colors group',
              selectedCategoryId === category.id ? 'bg-blue-50 text-blue-700 border border-blue-200' : 'hover:bg-gray-50'
            ]"
            :title="isCollapsed ? `${category.name} (${category.archiveCount})` : ''"
          >
            <div v-if="!isCollapsed">
              <div class="flex items-center justify-between">
                <div class="flex items-center space-x-2">
                  <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
                  </svg>
                  <span class="text-sm font-medium">{{ category.name }}</span>
                </div>
                <div class="flex items-center space-x-1">
                  <span class="text-xs text-gray-500">{{ category.archiveCount }}</span>
                  <button
                    @click.stop="$emit('edit-category', category)"
                    class="opacity-0 group-hover:opacity-100 p-1 text-gray-400 hover:text-blue-600"
                  >
                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                    </svg>
                  </button>
                </div>
              </div>
              <div v-if="category.description" class="text-xs text-gray-500 mt-1 pl-6">
                {{ category.description }}
              </div>
            </div>
            <div v-else class="flex justify-center">
              <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
              </svg>
            </div>
          </button>
        </div>
      </div>

      <!-- 动态分类 -->
      <div v-if="dynamicCategories.length > 0" class="mt-4">
        <div v-if="!isCollapsed" class="px-3 py-2 text-xs font-semibold text-gray-500 uppercase tracking-wider">
          动态分类
        </div>
        <div v-for="category in dynamicCategories" :key="category.id" class="space-y-1">
          <button
            @click="selectCategory(category.id)"
            :class="[
              'w-full text-left px-3 py-2 rounded-lg transition-colors group',
              selectedCategoryId === category.id ? 'bg-blue-50 text-blue-700 border border-blue-200' : 'hover:bg-gray-50'
            ]"
            :title="isCollapsed ? category.name : ''"
          >
            <div v-if="!isCollapsed">
              <div class="flex items-center justify-between">
                <div class="flex items-center space-x-2">
                  <svg class="w-4 h-4 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
                  </svg>
                  <span class="text-sm font-medium">{{ category.name }}</span>
                </div>
                <button
                  @click.stop="$emit('edit-category', category)"
                  class="opacity-0 group-hover:opacity-100 p-1 text-gray-400 hover:text-blue-600"
                >
                  <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                  </svg>
                </button>
              </div>
              <div v-if="category.description" class="text-xs text-gray-500 mt-1 pl-6">
                {{ category.description }}
              </div>
            </div>
            <div v-else class="flex justify-center">
              <svg class="w-4 h-4 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
              </svg>
            </div>
          </button>
        </div>
      </div>

      <!-- 空状态 -->
      <div v-if="staticCategories.length === 0 && dynamicCategories.length === 0" class="p-4 text-center">
        <div v-if="!isCollapsed" class="text-gray-400 text-sm">暂无分类</div>
        <button
          v-if="!isCollapsed"
          @click="$emit('create-category')"
          class="mt-2 text-blue-600 hover:text-blue-700 text-sm"
        >
          创建第一个分类
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useQuery } from '@tanstack/vue-query'
import { getCategories } from '@/utils/api'
import type { Category, DynamicCategory } from '@/types/api'

interface Props {
  selectedCategoryId?: string | null
  totalArchives?: number
  collapsed?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  totalArchives: 0,
  collapsed: false
})

// 折叠状态管理 - 现在使用外部传入的状态，并支持本地切换
const isCollapsed = ref(props.collapsed)

// 获取分类数据
const { data: categories, isLoading, error } = useQuery({
  queryKey: ['categories'],
  queryFn: getCategories,
})

// 分离静态和动态分类
const staticCategories = computed(() => {
  return categories.value?.filter(cat => cat.isStatic) || []
})

const dynamicCategories = computed(() => {
  // 动态分类需要特殊处理，因为后端返回的数据结构可能不同
  return categories.value?.filter(cat => !cat.isStatic) || []
})

const emit = defineEmits<{
  'select-category': [categoryId: string | null]
  'create-category': []
  'edit-category': [category: Category | DynamicCategory]
  'toggle-collapse': [collapsed: boolean]
}>()

const selectCategory = (categoryId: string | null) => {
  emit('select-category', categoryId)
}

const toggleCollapse = () => {
  isCollapsed.value = !isCollapsed.value
  emit('toggle-collapse', isCollapsed.value)
}

// 监听外部传入的 collapsed 属性变化
watch(() => props.collapsed, (newCollapsed) => {
  isCollapsed.value = newCollapsed
})
</script>

<style scoped>
.category-sidebar {
  height: 100%;
}
</style>