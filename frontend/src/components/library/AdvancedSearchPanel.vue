<template>
  <Transition
    enter-active-class="transition-all duration-200 ease-out"
    enter-from-class="opacity-0 -translate-y-2"
    enter-to-class="opacity-100 translate-y-0"
    leave-active-class="transition-all duration-150 ease-in"
    leave-from-class="opacity-100 translate-y-0"
    leave-to-class="opacity-0 -translate-y-2"
  >
    <div v-if="show" class="fixed top-14 left-0 right-0 z-40 bg-[#1b1b2f] border-b border-[#2d2d44] shadow-lg px-4 py-3">
      <div class="max-w-6xl mx-auto">
        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">

          <!-- 标签筛选 -->
          <div>
            <label class="block text-xs text-[#808090] mb-1">标签</label>
            <div class="relative">
              <input
                v-model="tagInput"
                type="text"
                placeholder="输入标签筛选..."
                class="w-full px-3 py-1.5 text-sm bg-[#2d2d44] border border-[#3d3d5c] rounded text-[#e0e0e0] placeholder-[#707090] focus:outline-none focus:border-[#7b68ee] transition-colors"
                @focus="showTagDropdown = true"
                @blur="handleTagBlur"
                @input="tagInput = ($event.target as HTMLInputElement).value"
              />
              <!-- 标签下拉建议 -->
              <div
                v-if="showTagDropdown && filteredTagSuggestions.length > 0"
                class="absolute left-0 right-0 z-50 mt-1 max-h-[200px] overflow-y-auto bg-[#1b1b2f] border border-[#3d3d5c] rounded shadow-lg"
              >
                <button
                  v-for="tag in filteredTagSuggestions"
                  :key="`${tag.namespace}:${tag.name}`"
                  class="flex items-center w-full px-3 py-1.5 text-left text-sm hover:bg-[#2d2d44] transition-colors"
                  @mousedown.prevent="addTag(tag)"
                >
                  <span class="text-[#7b68ee] mr-1">{{ tag.namespace }}:</span>
                  <span class="text-[#e0e0e0]">{{ tag.name }}</span>
                </button>
              </div>
            </div>
            <!-- 已选标签 chips -->
            <div v-if="selectedTags.length > 0" class="flex flex-wrap gap-1 mt-1.5">
              <span
                v-for="tag in selectedTags"
                :key="tag"
                class="inline-flex items-center px-2 py-0.5 rounded text-xs bg-[#7b68ee]/20 text-[#7b68ee] border border-[#7b68ee]/30"
              >
                {{ tag }}
                <button class="ml-1 hover:text-white" @click="removeTag(tag)">×</button>
              </span>
            </div>
          </div>

          <!-- 页数范围 -->
          <div>
            <label class="block text-xs text-[#808090] mb-1">页数范围</label>
            <div class="flex items-center gap-2">
              <input
                v-model.number="localFilters.minPages"
                type="number"
                min="0"
                placeholder="最少"
                class="w-full px-3 py-1.5 text-sm bg-[#2d2d44] border border-[#3d3d5c] rounded text-[#e0e0e0] placeholder-[#707090] focus:outline-none focus:border-[#7b68ee] transition-colors"
              />
              <span class="text-[#707090] text-xs">~</span>
              <input
                v-model.number="localFilters.maxPages"
                type="number"
                min="0"
                placeholder="最多"
                class="w-full px-3 py-1.5 text-sm bg-[#2d2d44] border border-[#3d3d5c] rounded text-[#e0e0e0] placeholder-[#707090] focus:outline-none focus:border-[#7b68ee] transition-colors"
              />
            </div>
          </div>

          <!-- 添加时间范围 -->
          <div>
            <label class="block text-xs text-[#808090] mb-1">添加时间</label>
            <div class="flex items-center gap-2">
              <input
                v-model="localFilters.createdAfter"
                type="date"
                class="w-full px-2 py-1.5 text-sm bg-[#2d2d44] border border-[#3d3d5c] rounded text-[#e0e0e0] focus:outline-none focus:border-[#7b68ee] transition-colors"
              />
              <span class="text-[#707090] text-xs">~</span>
              <input
                v-model="localFilters.createdBefore"
                type="date"
                class="w-full px-2 py-1.5 text-sm bg-[#2d2d44] border border-[#3d3d5c] rounded text-[#e0e0e0] focus:outline-none focus:border-[#7b68ee] transition-colors"
              />
            </div>
          </div>

          <!-- 排序 -->
          <div>
            <label class="block text-xs text-[#808090] mb-1">排序</label>
            <div class="flex items-center gap-2">
              <select
                v-model="localFilters.sortBy"
                class="flex-1 px-2 py-1.5 text-sm bg-[#2d2d44] border border-[#3d3d5c] rounded text-[#e0e0e0] focus:outline-none focus:border-[#7b68ee] transition-colors"
              >
                <option value="createdAt">添加时间</option>
                <option value="title">标题</option>
                <option value="fileSize">文件大小</option>
                <option value="pageCount">页数</option>
                <option value="updatedAt">更新时间</option>
              </select>
              <button
                class="p-1.5 bg-[#2d2d44] border border-[#3d3d5c] rounded text-[#a0a0c0] hover:text-white hover:border-[#7b68ee] transition-colors"
                :title="localFilters.sortOrder === 'asc' ? '升序' : '降序'"
                @click="toggleSortOrder"
              >
                <svg v-if="localFilters.sortOrder === 'asc'" class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 4h13M3 8h9m-9 4h6m4 0l4-4m0 0l4 4m-4-4v12" />
                </svg>
                <svg v-else class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 4h13M3 8h9m-9 4h9m5-4v12m0 0l-4-4m4 4l4-4" />
                </svg>
              </button>
            </div>
          </div>

        </div>

        <!-- 操作按钮 -->
        <div class="flex items-center justify-between mt-3 pt-2 border-t border-[#2d2d44]">
          <span v-if="activeFilterCount > 0" class="text-xs text-[#7b68ee]">
            已启用 {{ activeFilterCount }} 项筛选
          </span>
          <span v-else class="text-xs text-[#808090]">暂无筛选条件</span>
          <div class="flex items-center gap-2">
            <button
              class="px-3 py-1 text-xs text-[#a0a0c0] hover:text-white transition-colors"
              @click="handleReset"
            >
              重置
            </button>
            <button
              class="px-4 py-1 text-xs bg-[#7b68ee] text-white rounded hover:bg-[#6a5fd6] transition-colors"
              @click="handleApply"
            >
              应用筛选
            </button>
          </div>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useQuery } from '@tanstack/vue-query'
import { getTags } from '@/utils/api'
import type { SearchParams, Tag } from '@/types/api'

interface Props {
  show: boolean
  currentFilters?: Partial<SearchParams>
}

const props = withDefaults(defineProps<Props>(), {
  show: false,
})

const emit = defineEmits<{
  'apply-filters': [filters: Partial<SearchParams>]
  'reset-filters': []
}>()

const tagInput = ref('')
const showTagDropdown = ref(false)
const selectedTags = ref<string[]>([])

interface LocalFilters {
  minPages?: number
  maxPages?: number
  createdAfter?: string
  createdBefore?: string
  sortBy: string
  sortOrder: string
}

const localFilters = ref<LocalFilters>({
  sortBy: 'createdAt',
  sortOrder: 'asc',
})

// 获取所有标签用于自动完成
const { data: allTags } = useQuery({
  queryKey: ['tags'],
  queryFn: getTags,
  staleTime: 5 * 60 * 1000,
})

const filteredTagSuggestions = computed<Tag[]>(() => {
  const input = tagInput.value.trim().toLowerCase()
  if (!input || !allTags.value) return []
  return allTags.value
    .filter(tag => {
      const full = `${tag.namespace}:${tag.name}`.toLowerCase()
      const alreadySelected = selectedTags.value.includes(`${tag.namespace}:${tag.name}`)
      return !alreadySelected && (full.includes(input) || tag.name.toLowerCase().includes(input))
    })
    .slice(0, 8)
})

const activeFilterCount = computed(() => {
  let count = 0
  if (selectedTags.value.length > 0) count++
  if (localFilters.value.minPages != null || localFilters.value.maxPages != null) count++
  if (localFilters.value.createdAfter || localFilters.value.createdBefore) count++
  if (localFilters.value.sortBy !== 'createdAt' || localFilters.value.sortOrder !== 'asc') count++
  return count
})

const addTag = (tag: Tag) => {
  const tagStr = `${tag.namespace}:${tag.name}`
  if (!selectedTags.value.includes(tagStr)) {
    selectedTags.value.push(tagStr)
  }
  tagInput.value = ''
  showTagDropdown.value = false
}

const removeTag = (tag: string) => {
  selectedTags.value = selectedTags.value.filter(t => t !== tag)
}

const handleTagBlur = () => {
  setTimeout(() => { showTagDropdown.value = false }, 150)
}

const toggleSortOrder = () => {
  localFilters.value.sortOrder = localFilters.value.sortOrder === 'asc' ? 'desc' : 'asc'
}

const handleApply = () => {
  const filters: Partial<SearchParams> = {
    sortBy: localFilters.value.sortBy,
    sortOrder: localFilters.value.sortOrder,
  }
  if (selectedTags.value.length > 0) {
    filters.tags = selectedTags.value
  }
  if (localFilters.value.minPages != null) {
    filters.minPages = localFilters.value.minPages
  }
  if (localFilters.value.maxPages != null) {
    filters.maxPages = localFilters.value.maxPages
  }
  if (localFilters.value.createdAfter) {
    filters.createdAfter = localFilters.value.createdAfter
  }
  if (localFilters.value.createdBefore) {
    filters.createdBefore = localFilters.value.createdBefore
  }
  emit('apply-filters', filters)
}

const handleReset = () => {
  selectedTags.value = []
  tagInput.value = ''
  localFilters.value = { sortBy: 'createdAt', sortOrder: 'asc' }
  emit('reset-filters')
}

// 同步外部传入的当前筛选状态
watch(() => props.currentFilters, (filters) => {
  if (!filters) return
  if (filters.tags) selectedTags.value = [...filters.tags]
  if (filters.minPages != null) localFilters.value.minPages = filters.minPages
  if (filters.maxPages != null) localFilters.value.maxPages = filters.maxPages
  if (filters.createdAfter) localFilters.value.createdAfter = filters.createdAfter
  if (filters.createdBefore) localFilters.value.createdBefore = filters.createdBefore
  if (filters.sortBy) localFilters.value.sortBy = filters.sortBy
  if (filters.sortOrder) localFilters.value.sortOrder = filters.sortOrder
}, { immediate: true })
</script>
