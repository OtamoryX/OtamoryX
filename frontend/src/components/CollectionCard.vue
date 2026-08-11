<template>
  <button
    type="button"
    class="collection-card group text-left bg-[var(--bg-card)] border border-[var(--border)] rounded overflow-hidden hover:border-[var(--accent)] transition-colors"
    @click="$emit('open', collection)"
  >
    <div class="relative aspect-[2/3] bg-[var(--bg-tertiary)] overflow-hidden">
      <img v-if="coverUrl" :src="coverUrl" :alt="collection.displayTitle" class="w-full h-full object-cover" />
      <div v-else class="w-full h-full flex items-center justify-center p-3 text-center text-xs text-[var(--text-tertiary)]">
        {{ collection.displayTitle }}
      </div>
      <div v-if="collection.reviewCount > 0" class="absolute top-2 right-2 min-w-5 h-5 px-1.5 rounded-full bg-amber-500 text-white text-[10px] flex items-center justify-center">
        {{ collection.reviewCount }}
      </div>
    </div>
    <div class="px-2.5 py-2.5">
      <div class="h-9 text-xs font-medium text-[var(--text-primary)] leading-4 line-clamp-2 [overflow-wrap:anywhere]">
        {{ collection.displayTitle }}
      </div>
      <div class="mt-2 flex items-center justify-between gap-2 text-[10px] text-[var(--text-tertiary)]">
        <span>{{ collection.memberCount }} 本</span>
        <span :class="statusClass">{{ statusLabel }}</span>
      </div>
    </div>
  </button>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import type { CollectionSummary } from '@/types/api'
import { getArchiveThumbnail } from '@/utils/api'

const props = defineProps<{ collection: CollectionSummary }>()
defineEmits<{ open: [collection: CollectionSummary] }>()

const coverUrl = ref<string | null>(null)
const statusLabel = computed(() => {
  if (props.collection.isManualLocked || props.collection.status === 'manual') return '人工锁定'
  if (props.collection.status === 'needs_review') return '待确认'
  if (props.collection.variantCount > 0) return '含版本'
  return '自动识别'
})
const statusClass = computed(() => {
  if (props.collection.status === 'needs_review') return 'text-amber-400'
  if (props.collection.isManualLocked) return 'text-emerald-400'
  return 'text-[var(--text-tertiary)]'
})

onMounted(async () => {
  if (!props.collection.coverArchiveId) return
  try {
    coverUrl.value = await getArchiveThumbnail(props.collection.coverArchiveId)
  } catch {
    coverUrl.value = null
  }
})
</script>

<style scoped>
.line-clamp-2 {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
</style>
