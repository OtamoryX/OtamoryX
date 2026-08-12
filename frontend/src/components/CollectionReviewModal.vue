<template>
  <BaseModal :show="show" title="合集待确认" @close="emit('close')">
    <div class="max-h-[65vh] overflow-y-auto">
      <div v-if="reviews.length === 0" class="py-8 text-center text-sm text-[var(--text-tertiary)]">暂无待确认项目</div>
      <article v-for="review in reviews" :key="review.id" class="py-4 border-b border-[var(--border)] last:border-b-0">
        <div class="flex items-start justify-between gap-3">
          <div>
            <h3 class="text-sm font-semibold text-[var(--text-primary)] break-words">{{ review.collection.displayTitle }}</h3>
            <p class="mt-1 text-[11px] text-amber-400">{{ review.reason }}</p>
          </div>
          <span class="shrink-0 text-[10px] text-[var(--text-tertiary)]">{{ review.collection.memberCount }} 个已有文件</span>
        </div>

        <div class="mt-3 grid gap-3 sm:grid-cols-2">
          <section class="min-w-0">
            <p class="text-[10px] text-[var(--text-tertiary)]">待加入内容</p>
            <div class="mt-1.5 flex gap-2">
              <img v-if="coverUrls[review.archive.id]" :src="coverUrls[review.archive.id]" :alt="review.archive.title" class="h-16 w-11 shrink-0 rounded-sm object-cover bg-[var(--bg-tertiary)]" />
              <div class="min-w-0">
                <p class="text-xs text-[var(--text-primary)] break-words">{{ review.archive.title }}</p>
                <p class="mt-1 text-[10px] text-[var(--text-tertiary)]">{{ review.archive.pageCount }} 页 · {{ formatSize(review.archive.fileSize) }}</p>
                <button class="mt-1.5 text-[11px] text-[var(--accent)] hover:underline" @click="emit('openReader', review.archive.id)">查看内容</button>
              </div>
            </div>
          </section>

          <section class="min-w-0 border-t border-[var(--border)] pt-3 sm:border-l sm:border-t-0 sm:pl-3 sm:pt-0">
            <p class="text-[10px] text-[var(--text-tertiary)]">候选合集已有内容</p>
            <div v-if="contexts[review.collection.id]?.members.length" class="mt-1.5 space-y-1.5">
              <div v-for="member in contexts[review.collection.id]?.members.slice(0, 3) || []" :key="member.archive.id" class="flex items-center gap-2 min-w-0">
                <img v-if="coverUrls[member.archive.id]" :src="coverUrls[member.archive.id]" :alt="member.archive.title" class="h-8 w-6 shrink-0 rounded-sm object-cover bg-[var(--bg-tertiary)]" />
                <div class="min-w-0 flex-1">
                  <p class="truncate text-[11px] text-[var(--text-secondary)]">{{ member.archive.title }}</p>
                  <p class="text-[10px] text-[var(--text-tertiary)]">{{ memberLabel(member) }} · {{ member.archive.pageCount }} 页</p>
                </div>
                <button class="shrink-0 text-[10px] text-[var(--accent)] hover:underline" @click="emit('openReader', member.archive.id)">查看</button>
              </div>
            </div>
            <p v-else class="mt-2 text-[11px] text-[var(--text-tertiary)]">正在加载已有内容...</p>
          </section>
        </div>

        <div class="mt-3 flex justify-end gap-2">
          <button class="px-2.5 py-1.5 rounded text-xs border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)]" :disabled="busyId === review.id" @click="apply(review, 'reject')">不是同一合集</button>
          <button class="px-2.5 py-1.5 rounded text-xs bg-[var(--accent)] text-white hover:opacity-90" :disabled="busyId === review.id" @click="apply(review, 'approve')">确认加入</button>
        </div>
      </article>
    </div>
  </BaseModal>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import BaseModal from '@/components/base/BaseModal.vue'
import type { CollectionDetail, CollectionMember, CollectionReviewItem } from '@/types/api'
import { applyCollectionReview, getArchiveThumbnail, getCollection } from '@/utils/api'

const props = defineProps<{ show: boolean; reviews: CollectionReviewItem[] }>()
const emit = defineEmits<{ close: []; changed: []; openReader: [archiveId: string] }>()
const busyId = ref<string | null>(null)
const contexts = ref<Record<string, CollectionDetail>>({})
const coverUrls = ref<Record<string, string>>({})

const loadContexts = async () => {
  if (!props.show) return
  const ids = [...new Set(props.reviews.map(review => review.collection.id))]
  const details = await Promise.all(ids.map(async id => [id, await getCollection(id).catch(() => null)] as const))
  contexts.value = Object.fromEntries(details.filter(([, detail]) => detail).map(([id, detail]) => [id, detail!]))
  const archiveIds = new Set(props.reviews.map(review => review.archive.id))
  Object.values(contexts.value).forEach(detail => detail.members.slice(0, 3).forEach(member => archiveIds.add(member.archive.id)))
  const covers = await Promise.all([...archiveIds].map(async id => [id, await getArchiveThumbnail(id).catch(() => '')] as const))
  coverUrls.value = Object.fromEntries(covers.filter(([, url]) => url))
}

watch([() => props.show, () => props.reviews], () => { void loadContexts() }, { immediate: true, deep: true })

const apply = async (review: CollectionReviewItem, action: 'approve' | 'reject') => {
  busyId.value = review.id
  try { await applyCollectionReview(review.id, action); emit('changed') } finally { busyId.value = null }
}
const formatSize = (bytes: number) => bytes >= 1024 * 1024 ? `${(bytes / 1024 / 1024).toFixed(1)} MB` : `${Math.ceil(bytes / 1024)} KB`
const memberLabel = (member: CollectionMember) => member.volumeNumber ? `第 ${member.volumeNumber} 卷` : member.chapterNumber ? `第 ${member.chapterNumber} 话` : member.issueNumber ? `期号 ${member.issueNumber}` : member.rawNumber ? `编号 ${member.rawNumber}` : '未编号'
</script>
