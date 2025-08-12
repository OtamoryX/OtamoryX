<template>
  <span
    :class="[
      'inline-flex items-center px-3 py-1.5 rounded-lg text-sm transition-colors',
      colorClasses,
      removable && 'group cursor-pointer hover:bg-opacity-80'
    ]"
    @click="handleClick"
  >
    <!-- 标签内容 -->
    <span class="mr-2 truncate">{{ displayText }}</span>
    
    <!-- 删除按钮 -->
    <svg 
      v-if="removable"
      class="w-3 h-3 opacity-0 group-hover:opacity-100 transition-opacity flex-shrink-0" 
      fill="none" 
      stroke="currentColor" 
      viewBox="0 0 24 24"
    >
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
    </svg>
  </span>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { Tag } from '@/types/api'

interface Props {
  tag: Tag
  removable?: boolean
  color?: 'default' | 'blue' | 'green' | 'purple' | 'red' | 'yellow'
  showNamespace?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  removable: false,
  color: 'default',
  showNamespace: true
})

const emit = defineEmits<{
  remove: [tagId: string]
  click: [tag: Tag]
}>()

const displayText = computed(() => {
  if (props.showNamespace && props.tag.namespace) {
    return `${props.tag.namespace}:${props.tag.name}`
  }
  return props.tag.name
})

const colorClasses = computed(() => {
  const colorMap = {
    default: 'bg-gray-700 hover:bg-gray-600 text-white',
    blue: 'bg-blue-100 hover:bg-blue-200 text-blue-800',
    green: 'bg-green-100 hover:bg-green-200 text-green-800',
    purple: 'bg-purple-100 hover:bg-purple-200 text-purple-800',
    red: 'bg-red-100 hover:bg-red-200 text-red-800',
    yellow: 'bg-yellow-100 hover:bg-yellow-200 text-yellow-800'
  }
  return colorMap[props.color]
})

const handleClick = () => {
  if (props.removable) {
    emit('remove', props.tag.id)
  } else {
    emit('click', props.tag)
  }
}
</script>

<style scoped>
.truncate {
  max-width: 200px;
}
</style>