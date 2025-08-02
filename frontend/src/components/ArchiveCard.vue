<template>
  <div 
    class="archive-card bg-white rounded-lg shadow-md overflow-hidden hover:shadow-lg transition-shadow cursor-pointer"
    @click="$emit('click')"
  >
    <div class="aspect-[3/4] bg-gray-200 relative">
      <img
        v-if="coverImageUrl"
        :src="coverImageUrl"
        :alt="archive.title"
        class="w-full h-full object-cover"
        @error="handleImageError"
      />
      <div v-else class="w-full h-full flex items-center justify-center text-gray-400">
        <svg class="w-12 h-12" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 4V2a1 1 0 011-1h8a1 1 0 011 1v2h4a1 1 0 110 2h-1v12a2 2 0 01-2 2H6a2 2 0 01-2-2V6H3a1 1 0 110-2h4z" />
        </svg>
      </div>
    </div>
    
    <div class="p-3">
      <h3 class="font-medium text-gray-900 text-sm mb-1 line-clamp-2">
        {{ archive.title }}
      </h3>
      
      <div class="flex items-center justify-between text-xs text-gray-500 mb-2">
        <span v-if="archive.pageCount">{{ archive.pageCount }}页</span>
        <span v-if="archive.createdAt">{{ formatDate(archive.createdAt) }}</span>
      </div>
      
      <!-- 阅读进度条 -->
      <div v-if="progressPercentage !== undefined && progressPercentage > 0" class="mt-2">
        <div class="flex items-center justify-between text-xs text-gray-600 mb-1">
          <span>进度</span>
          <span>{{ progressPercentage.toFixed(1) }}%</span>
        </div>
        <div class="w-full bg-gray-200 rounded-full h-1.5">
          <div 
            class="bg-blue-500 h-1.5 rounded-full transition-all duration-300"
            :style="{ width: `${progressPercentage}%` }"
          ></div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import type { Archive } from '@/types/api'

interface Props {
  archive: Archive
  progressPercentage?: number
}

const props = defineProps<Props>()
defineEmits<{
  click: []
}>()

// 计算封面图片URL - 使用缩略图接口
const coverImageUrl = computed(() => {
  return `/api/v1/archives/${props.archive.id}/thumbnail`
})

const handleImageError = (event: Event) => {
  const img = event.target as HTMLImageElement
  img.style.display = 'none'
}

const formatDate = (dateString: string) => {
  const date = new Date(dateString)
  return date.toLocaleDateString('zh-CN', {
    year: 'numeric',
    month: 'short',
    day: 'numeric'
  })
}
</script>

<style scoped>
.line-clamp-2 {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
</style>