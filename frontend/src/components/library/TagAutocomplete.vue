<template>
  <Transition name="dropdown">
    <div v-if="show && filteredTags.length > 0" ref="dropdownRef"
      class="absolute left-0 right-0 z-50 mt-1 max-h-[300px] overflow-y-auto bg-[var(--bg-card)] border border-[var(--border)] rounded-lg shadow-lg"
      role="listbox" aria-label="标签建议">
      <template v-for="(tags, namespace) in groupedTags" :key="namespace">
        <div class="px-3 py-1.5 text-xs font-semibold text-[var(--text-tertiary)] uppercase tracking-wider bg-[var(--bg-secondary)] sticky top-0">
          {{ tagNamespaceLabel(String(namespace)) }}
        </div>
        <button v-for="tag in tags" :key="`${namespace}:${tag.name}`"
          class="flex items-center w-full px-3 py-2 text-left transition-colors"
          :class="flatIndex(namespace, tag.name) === activeIndex
            ? 'bg-[var(--accent)]/10'
            : 'hover:bg-[var(--bg-tertiary)]'"
          role="option"
          :aria-selected="flatIndex(namespace, tag.name) === activeIndex"
          @click="selectTag(namespace as string, tag.name)"
          @mouseenter="activeIndex = flatIndex(namespace, tag.name)">
          <span class="text-[var(--accent)] mr-1">{{ tagNamespaceLabel(String(namespace)) }}:</span>
          <span class="text-[var(--text-primary)]">{{ tagDisplayName(tag) }}</span>
        </button>
      </template>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { useQuery } from '@tanstack/vue-query'
import { getTags } from '@/utils/api'
import type { Tag } from '@/types/api'
import { tagDisplayName, tagNamespaceLabel, tagSearchText } from '@/utils/tagDisplay'

interface Props {
  query: string
  show: boolean
}

const props = defineProps<Props>()

const emit = defineEmits<{
  'select-tag': [tag: string]
  'close': []
}>()

const dropdownRef = ref<HTMLElement | null>(null)
const activeIndex = ref(0)
const debouncedQuery = ref('')
let debounceTimer: ReturnType<typeof setTimeout> | null = null

// 使用 TanStack Query 获取所有标签
const { data: allTags } = useQuery({
  queryKey: ['tags'],
  queryFn: getTags,
  staleTime: 5 * 60 * 1000,
})

// 防抖处理输入
watch(() => props.query, (newVal) => {
  if (debounceTimer) clearTimeout(debounceTimer)
  debounceTimer = setTimeout(() => {
    debouncedQuery.value = newVal
  }, 300)
})

// 从输入中提取当前正在输入的标签片段
const currentFragment = computed(() => {
  const q = debouncedQuery.value.trim()
  if (!q) return ''
  // 取最后一个逗号后的内容
  const parts = q.split(',')
  return parts[parts.length - 1]!.trim()
})

// 过滤匹配的标签，最多 10 个
const filteredTags = computed<Tag[]>(() => {
  const fragment = currentFragment.value.toLowerCase()
  if (!fragment || !allTags?.value) return []

  return allTags.value
    .filter(tag => {
      return tagSearchText(tag).includes(fragment)
    })
    .slice(0, 10)
})

// 按 namespace 分组
const groupedTags = computed(() => {
  const groups: Record<string, Tag[]> = {}
  for (const tag of filteredTags.value) {
    if (!groups[tag.namespace]) {
      groups[tag.namespace] = []
    }
    groups[tag.namespace]!.push(tag)
  }
  return groups
})

// 计算扁平索引（用于键盘导航）
const flatIndex = (namespace: string | number, name: string): number => {
  let idx = 0
  for (const [ns, tags] of Object.entries(groupedTags.value)) {
    for (const tag of tags) {
      if (ns === namespace && tag.name === name) return idx
      idx++
    }
  }
  return -1
}

// 根据扁平索引获取标签
const getTagByIndex = (index: number): { namespace: string; name: string } | null => {
  let idx = 0
  for (const [ns, tags] of Object.entries(groupedTags.value)) {
    for (const tag of tags) {
      if (idx === index) return { namespace: ns, name: tag.name }
      idx++
    }
  }
  return null
}
// 选择标签
const selectTag = (namespace: string, name: string) => {
  emit('select-tag', `${namespace}:${name}`)
  activeIndex.value = 0
}

// 键盘导航
const handleKeydown = (event: KeyboardEvent) => {
  if (!props.show || filteredTags.value.length === 0) return

  const total = filteredTags.value.length

  switch (event.key) {
    case 'ArrowDown':
      event.preventDefault()
      activeIndex.value = (activeIndex.value + 1) % total
      break
    case 'ArrowUp':
      event.preventDefault()
      activeIndex.value = (activeIndex.value - 1 + total) % total
      break
    case 'Enter': {
      event.preventDefault()
      const tag = getTagByIndex(activeIndex.value)
      if (tag) selectTag(tag.namespace, tag.name)
      break
    }
    case 'Escape':
      event.preventDefault()
      emit('close')
      break
  }
}

// 点击外部关闭
const handleClickOutside = (event: MouseEvent) => {
  if (dropdownRef.value && !dropdownRef.value.contains(event.target as Node)) {
    emit('close')
  }
}

// 重置 activeIndex
watch(() => filteredTags.value, () => {
  activeIndex.value = 0
})

onMounted(() => {
  document.addEventListener('keydown', handleKeydown)
  document.addEventListener('mousedown', handleClickOutside)
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown)
  document.removeEventListener('mousedown', handleClickOutside)
  if (debounceTimer) clearTimeout(debounceTimer)
})
</script>

<style scoped>
.dropdown-enter-active,
.dropdown-leave-active {
  transition: all 0.2s ease;
}

.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}

.dropdown-enter-to,
.dropdown-leave-from {
  opacity: 1;
  transform: translateY(0);
}
</style>
