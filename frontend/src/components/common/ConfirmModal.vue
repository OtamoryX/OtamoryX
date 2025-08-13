<template>
  <BaseModal
    :show="show"
    :title="title"
    width="md"
    :z-index="9999"
    @close="$emit('close')"
  >
    <!-- 确认内容 -->
    <div class="space-y-4">
      <!-- 图标 -->
      <div v-if="showIcon" class="flex justify-center">
        <div :class="iconClasses">
          <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path 
              v-if="type === 'danger'"
              stroke-linecap="round" 
              stroke-linejoin="round" 
              stroke-width="2" 
              d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z" 
            />
            <path 
              v-else-if="type === 'warning'"
              stroke-linecap="round" 
              stroke-linejoin="round" 
              stroke-width="2" 
              d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" 
            />
            <path 
              v-else-if="type === 'info'"
              stroke-linecap="round" 
              stroke-linejoin="round" 
              stroke-width="2" 
              d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" 
            />
            <path 
              v-else
              stroke-linecap="round" 
              stroke-linejoin="round" 
              stroke-width="2" 
              d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" 
            />
          </svg>
        </div>
      </div>

      <!-- 消息内容 -->
      <div class="text-center">
        <p class="text-white/80 leading-6">{{ message }}</p>
      </div>
    </div>

    <!-- 操作按钮 -->
    <template #footer>
      <div class="flex justify-end space-x-3">
        <button
          @click="$emit('close')"
          class="px-4 py-2 text-white/70 hover:text-white bg-white/10 hover:bg-white/20 rounded-lg transition-all duration-200"
        >
          {{ cancelText }}
        </button>
        <button
          @click="handleConfirm"
          :class="confirmButtonClasses"
          :disabled="loading"
        >
          <svg 
            v-if="loading" 
            class="animate-spin -ml-1 mr-2 h-4 w-4" 
            fill="none" 
            viewBox="0 0 24 24"
          >
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
          {{ loading ? loadingText : confirmText }}
        </button>
      </div>
    </template>
  </BaseModal>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import BaseModal from '@/components/base/BaseModal.vue'

interface Props {
  show: boolean
  title?: string
  message: string
  type?: 'default' | 'danger' | 'warning' | 'info'
  confirmText?: string
  cancelText?: string
  loadingText?: string
  showIcon?: boolean
  loading?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  title: '确认操作',
  type: 'default',
  confirmText: '确认',
  cancelText: '取消',
  loadingText: '处理中...',
  showIcon: true,
  loading: false
})

const emit = defineEmits<{
  close: []
  confirm: []
}>()

const iconClasses = computed(() => {
  const baseClasses = 'mx-auto flex-shrink-0 flex items-center justify-center h-12 w-12 rounded-full backdrop-blur-md border'
  
  const typeMap = {
    danger: 'bg-red-500/20 text-red-300 border-red-400/30',
    warning: 'bg-yellow-500/20 text-yellow-300 border-yellow-400/30',
    info: 'bg-blue-500/20 text-blue-300 border-blue-400/30',
    default: 'bg-white/20 text-white/70 border-white/30'
  }
  
  return `${baseClasses} ${typeMap[props.type]}`
})

const confirmButtonClasses = computed(() => {
  const baseClasses = 'px-4 py-2 rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center'
  
  const typeMap = {
    danger: 'bg-red-600 text-white hover:bg-red-700',
    warning: 'bg-yellow-600 text-white hover:bg-yellow-700',
    info: 'bg-blue-600 text-white hover:bg-blue-700',
    default: 'bg-gray-600 text-white hover:bg-gray-700'
  }
  
  return `${baseClasses} ${typeMap[props.type]}`
})

const handleConfirm = () => {
  if (!props.loading) {
    emit('confirm')
  }
}
</script>

<style scoped>
/* 样式已在Tailwind中定义，无需额外样式 */
</style>