<template>
  <teleport to="body">
    <transition
      enter-active-class="transition-all duration-300 ease-out"
      enter-from-class="opacity-0"
      enter-to-class="opacity-100"
      leave-active-class="transition-all duration-300 ease-in"
      leave-from-class="opacity-100"
      leave-to-class="opacity-0"
    >
      <div
        v-if="show"
        :class="['fixed inset-0 flex items-center justify-center', zIndexClass]"
        @click="handleMaskClick"
      >
        <!-- 背景遮罩 -->
        <div class="absolute inset-0 bg-black/50 backdrop-blur-sm"></div>
        
        <!-- 模态框内容 -->
        <transition
          enter-active-class="transition-all duration-300 ease-out"
          enter-from-class="opacity-0 scale-95 translate-y-4"
          enter-to-class="opacity-100 scale-100 translate-y-0"
          leave-active-class="transition-all duration-300 ease-in"
          leave-from-class="opacity-100 scale-100 translate-y-0"
          leave-to-class="opacity-0 scale-95 translate-y-4"
        >
          <div
            v-if="show"
            :class="[
              'relative bg-black/20 backdrop-blur-xl border border-white/20 rounded-lg shadow-2xl mx-4 flex flex-col',
              widthClass,
              maxHeightClass
            ]"
            @click.stop
          >
            <!-- 模态框头部 -->
            <div v-if="$slots.header || title" class="modal-header border-b border-white/20 p-6 flex-shrink-0">
              <slot name="header" :title="title" :onClose="handleClose">
                <div class="flex items-center justify-between">
                  <h3 class="text-lg font-bold text-white">{{ title }}</h3>
                  <button
                    v-if="closable"
                    @click="handleClose"
                    class="text-white/60 hover:text-white/80 transition-colors p-1 rounded-md hover:bg-white/10"
                  >
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                    </svg>
                  </button>
                </div>
              </slot>
            </div>

            <!-- 模态框内容 -->
            <div :class="['modal-body', 'flex-1 min-h-0', contentPadding ? 'p-6' : '']">
              <slot></slot>
            </div>

            <!-- 模态框底部 -->
            <div v-if="$slots.footer" class="modal-footer border-t border-white/20 p-6 flex-shrink-0">
              <slot name="footer" :onClose="handleClose"></slot>
            </div>
          </div>
        </transition>
      </div>
    </transition>
  </teleport>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue'

interface Props {
  show: boolean
  title?: string
  width?: 'sm' | 'md' | 'lg' | 'xl' | 'full'
  maxHeight?: 'sm' | 'md' | 'lg' | 'xl' | 'full' | 'screen'
  closable?: boolean
  maskClosable?: boolean
  contentPadding?: boolean
  zIndex?: number
}

const props = withDefaults(defineProps<Props>(), {
  width: 'md',
  maxHeight: 'lg',
  closable: true,
  maskClosable: true,
  contentPadding: true,
  zIndex: 50
})

const emit = defineEmits<{
  close: []
}>()

const widthClass = computed(() => {
  const widthMap = {
    sm: 'max-w-sm w-full',
    md: 'max-w-md w-full',
    lg: 'max-w-lg w-full',
    xl: 'max-w-xl w-full',
    full: 'max-w-full w-full'
  }
  return widthMap[props.width]
})

const maxHeightClass = computed(() => {
  const heightMap = {
    sm: 'max-h-60',
    md: 'max-h-96',
    lg: 'max-h-[32rem]',
    xl: 'max-h-[40rem]',
    full: 'max-h-full',
    screen: 'max-h-screen'
  }
  return heightMap[props.maxHeight]
})

const zIndexClass = computed(() => `z-${props.zIndex}`)

const handleClose = () => {
  emit('close')
}

const handleMaskClick = () => {
  if (props.maskClosable) {
    handleClose()
  }
}

// ESC键关闭
const handleKeydown = (event: KeyboardEvent) => {
  if (event.key === 'Escape' && props.show && props.closable) {
    handleClose()
  }
}

onMounted(() => {
  document.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown)
})
</script>

<style scoped>
.modal-body {
  overflow-y: auto;
}

/* 滚动条样式 */
.modal-body::-webkit-scrollbar {
  width: 6px;
}

.modal-body::-webkit-scrollbar-track {
  background: rgba(0, 0, 0, 0.1);
  border-radius: 3px;
}

.modal-body::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.3);
  border-radius: 3px;
}

.modal-body::-webkit-scrollbar-thumb:hover {
  background: rgba(0, 0, 0, 0.5);
}
</style>