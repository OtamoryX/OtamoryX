<template>
  <BaseModal :show="show" title="待确认合集" @close="$emit('close')">
    <div class="max-h-[65vh] overflow-y-auto">
      <div v-if="reviews.length === 0" class="py-8 text-center text-sm text-[var(--text-tertiary)]">暂无待确认项目</div>
      <div v-for="review in reviews" :key="review.id" class="py-3 border-b border-[var(--border)] last:border-b-0">
        <div class="grid grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] gap-2 items-start">
          <div class="min-w-0">
            <div class="text-[10px] text-[var(--text-tertiary)]">新漫画</div>
            <div class="mt-1 text-xs text-[var(--text-primary)] break-words">{{ review.archive.title }}</div>
          </div>
          <ArrowRightIcon class="w-4 h-4 mt-5 text-[var(--text-tertiary)]" />
          <div class="min-w-0">
            <div class="text-[10px] text-[var(--text-tertiary)]">候选合集</div>
            <div class="mt-1 text-xs text-[var(--text-primary)] break-words">{{ review.collection.displayTitle }}</div>
          </div>
        </div>
        <div class="mt-2 text-[11px] text-amber-400">{{ review.reason }}</div>
        <div class="mt-2 flex justify-end gap-2">
          <button class="px-2.5 py-1.5 rounded text-xs border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)]" :disabled="busyId === review.id" @click="apply(review, 'reject')">不是同一合集</button>
          <button class="px-2.5 py-1.5 rounded text-xs bg-[var(--accent)] text-white hover:opacity-90" :disabled="busyId === review.id" @click="apply(review, 'approve')">确认加入</button>
        </div>
      </div>
    </div>
  </BaseModal>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { ArrowRightIcon } from '@heroicons/vue/24/outline'
import BaseModal from '@/components/base/BaseModal.vue'
import type { CollectionReviewItem } from '@/types/api'
import { applyCollectionReview } from '@/utils/api'

defineProps<{ show: boolean; reviews: CollectionReviewItem[] }>()
const emit = defineEmits<{ close: []; changed: [] }>()
const busyId = ref<string | null>(null)
const apply = async (review: CollectionReviewItem, action: 'approve' | 'reject') => {
  busyId.value = review.id
  try { await applyCollectionReview(review.id, action); emit('changed') } finally { busyId.value = null }
}
</script>
