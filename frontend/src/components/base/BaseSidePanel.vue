<template>
  <transition
    enter-active-class="transition-all duration-300 ease-out"
    enter-from-class="opacity-0 translate-x-full"
    enter-to-class="opacity-100 translate-x-0"
    leave-active-class="transition-all duration-300 ease-in"
    leave-from-class="opacity-100 translate-x-0"
    leave-to-class="opacity-0 translate-x-full"
  >
    <div
      v-if="show"
      :class="['fixed inset-0', zIndexClass]"
      @click="handleMaskClick"
    >
      <!-- 背景遮罩 -->
      <div class="absolute inset-0 bg-black/30 backdrop-blur-sm"></div>
      
      <!-- 侧边面板 -->
      <div
        :class="[
          'absolute right-0 top-0 h-full bg-black/90 backdrop-blur-md border-l border-white/10 text-white overflow-y-auto',
          widthClass
        ]"
        @click.stop
      >
        <!-- 面板头部 -->
        <div class="sticky top-0 bg-black/80 backdrop-blur-md border-b border-white/10 p-6">
          <slot name="header" :title="title" :onClose="handleClose">
            <div class="flex items-center justify-between">
              <h2 class="text-lg font-semibold">{{ title }}</h2>
              <button
                v-if="closable"
                @click="handleClose"
                class="toolbar-button p-2 hover:bg-white/20 rounded-lg transition-colors"
              >
                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>
          </slot>
        </div>

        <!-- 面板内容 -->
        <div class="panel-content p-6">
          <slot></slot>
        </div>

        <!-- 面板底部 -->
        <div v-if="$slots.footer" class="sticky bottom-0 bg-black/80 backdrop-blur-md border-t border-white/10 p-6">
          <slot name="footer"></slot>
        </div>
      </div>
    </div>
  </transition>
</template>

<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  show: boolean
  title: string
  width?: 'narrow' | 'normal' | 'wide'
  closable?: boolean
  maskClosable?: boolean
  zIndex?: number
}

const props = withDefaults(defineProps<Props>(), {
  width: 'normal',
  closable: true,
  maskClosable: true,
  zIndex: 40
})

const emit = defineEmits<{
  close: []
}>()

const widthClass = computed(() => {
  const widthMap = {
    narrow: 'w-80 max-w-full sm:max-w-80',
    normal: 'w-96 max-w-full sm:max-w-96', 
    wide: 'w-[32rem] max-w-full sm:max-w-[32rem]'
  }
  return widthMap[props.width]
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
</script>

<style scoped>
/* 工具栏按钮样式 */
.toolbar-button {
  backdrop-filter: blur(8px);
  border: 1px solid rgba(255, 255, 255, 0.1);
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.toolbar-button:hover {
  background: rgba(255, 255, 255, 0.15);
  border-color: rgba(255, 255, 255, 0.2);
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
}

/* 滚动条样式 */
.panel-content::-webkit-scrollbar {
  width: 6px;
}

.panel-content::-webkit-scrollbar-track {
  background: rgba(255, 255, 255, 0.1);
  border-radius: 3px;
}

.panel-content::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.3);
  border-radius: 3px;
}

.panel-content::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.5);
}
</style>