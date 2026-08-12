<template>
  <div
    class="archive-card bg-[var(--bg-card)] border border-[var(--border)] rounded cursor-pointer hover:border-[var(--accent)] transition-colors duration-150 overflow-hidden"
    @click="handleClick"
    @contextmenu.prevent="emit('contextmenu', $event, collection)"
    @touchstart="handleTouchStart"
    @touchend="handleTouchEnd"
    @touchcancel="clearLongPress"
    @touchmove="clearLongPress"
  >
    <div class="h-16 px-2 pt-2 pb-[0.3rem] flex flex-col gap-0.5">
      <h3 class="text-xs font-semibold text-[var(--text-primary)] leading-4 min-h-8 max-h-8 overflow-hidden line-clamp-2 [overflow-wrap:anywhere]" :title="collection.displayTitle">
        {{ collection.displayTitle }}
      </h3>
      <p v-if="collection.subtitle" class="h-3 text-[10px] leading-3 text-[var(--text-tertiary)] truncate" :title="collection.subtitle">
        {{ collection.subtitle }}
      </p>
      <div v-else class="h-3" aria-hidden="true" />
    </div>

    <div class="relative mx-1 aspect-[2/3] bg-[var(--bg-tertiary)] overflow-hidden rounded-sm">
      <img v-if="coverUrl" :src="coverUrl" :alt="collection.displayTitle" class="w-full h-full object-cover" />
      <div v-else class="w-full h-full flex items-center justify-center p-3 text-center text-xs text-[var(--text-tertiary)]">
        {{ collection.displayTitle }}
      </div>
      <div v-if="collection.progressPercentage && collection.progressPercentage > 0" class="absolute bottom-0 left-0 right-0 h-0.5 bg-black/20">
        <div class="h-full bg-[var(--accent)] transition-all duration-500" :style="{ width: `${Math.min(collection.progressPercentage, 1) * 100}%` }" />
      </div>
    </div>

    <div class="px-2 pt-1 pb-2">
      <div class="text-[10px] text-[var(--text-tertiary)] mb-1">
        {{ collection.contentCount }} 个内容 · {{ collection.memberCount }} 个文件
      </div>
      <div class="h-5 flex items-center justify-between gap-1 text-[10px] leading-5" :class="statusClass">
        <span>{{ statusLabel }}</span>
        <span v-if="collection.matchedMemberCount < collection.memberCount" class="text-[var(--accent)]">命中 {{ collection.matchedMemberCount }}/{{ collection.memberCount }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import type { CollectionSummary } from '@/types/api'
import { getArchiveThumbnail } from '@/utils/api'

const props = defineProps<{ collection: CollectionSummary }>()
const emit = defineEmits<{
  open: [collection: CollectionSummary]
  contextmenu: [event: MouseEvent, collection: CollectionSummary]
}>()

const coverUrl = ref<string | null>(null)
const longPressTimer = ref<ReturnType<typeof setTimeout> | null>(null)
const touchStart = ref<{ x: number; y: number } | null>(null)
const suppressClick = ref(false)

const statusLabel = computed(() => {
  if (props.collection.status === 'needs_review') return '待确认'
  if (props.collection.isManualLocked) return '人工整理'
  return '自动识别'
})
const statusClass = computed(() => props.collection.status === 'needs_review' ? 'text-amber-400' : 'text-[var(--text-tertiary)]')

const clearLongPress = () => {
  if (longPressTimer.value) clearTimeout(longPressTimer.value)
  longPressTimer.value = null
  touchStart.value = null
}
const handleTouchStart = (event: TouchEvent) => {
  if (event.touches.length !== 1) return
  const touch = event.touches[0]
  touchStart.value = { x: touch.clientX, y: touch.clientY }
  longPressTimer.value = setTimeout(() => {
    if (!touchStart.value) return
    suppressClick.value = true
    emit('contextmenu', new MouseEvent('contextmenu', { clientX: touchStart.value.x, clientY: touchStart.value.y }), props.collection)
    clearLongPress()
  }, 500)
}
const handleTouchEnd = () => clearLongPress()
const handleClick = () => {
  if (suppressClick.value) {
    suppressClick.value = false
    return
  }
  emit('open', props.collection)
}

onMounted(async () => {
  if (!props.collection.coverArchiveId) return
  coverUrl.value = await getArchiveThumbnail(props.collection.coverArchiveId).catch(() => null)
})
onUnmounted(clearLongPress)
</script>

<style scoped>
.line-clamp-2 { display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; }
</style>
