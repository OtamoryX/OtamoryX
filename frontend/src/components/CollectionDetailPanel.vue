<template>
  <BaseSidePanel :show="show" title="合集详情" width="wide" @close="$emit('close')">
    <div v-if="isLoading" class="p-4 text-sm text-[var(--text-tertiary)]">加载中...</div>
    <div v-else-if="!detail" class="p-4 text-sm text-[var(--text-tertiary)]">合集不存在或没有访问权限。</div>
    <div v-else class="p-4">
      <div class="flex items-start gap-3 pb-4 border-b border-[var(--border)]">
        <div class="w-16 h-24 rounded-sm overflow-hidden bg-[var(--bg-tertiary)] flex-shrink-0">
          <img v-if="coverUrl" :src="coverUrl" :alt="detail.collection.displayTitle" class="w-full h-full object-cover" />
        </div>
        <div class="min-w-0 flex-1">
          <h2 class="text-base font-semibold text-[var(--text-primary)] break-words">{{ detail.collection.displayTitle }}</h2>
          <p v-if="detail.collection.subtitle" class="mt-1 text-sm text-[var(--text-tertiary)] break-words">{{ detail.collection.subtitle }}</p>
          <p class="mt-1 text-xs text-[var(--text-tertiary)]">{{ detail.collection.contentCount }} 个内容 · {{ detail.collection.memberCount }} 个文件</p>
          <p v-if="detail.collection.reviewCount" class="mt-1 text-xs text-amber-400">{{ detail.collection.reviewCount }} 条待确认</p>
          <button class="mt-3 inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded text-xs bg-[var(--accent)] text-white hover:opacity-90" @click="openReader(nextMember?.archive.id)">
            <BookOpenIcon class="w-3.5 h-3.5" />继续阅读
          </button>
          <button v-if="detail.collection.variantGroupCount" class="mt-3 ml-2 inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded text-xs border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)]" @click="$emit('viewVersions')">
            查看多版本
          </button>
        </div>
      </div>

      <div class="pt-4">
        <div class="flex items-center justify-between mb-2">
          <h3 class="text-sm font-medium text-[var(--text-primary)]">成员</h3>
          <span class="text-[10px] text-[var(--text-tertiary)]">按识别顺序</span>
        </div>
        <div v-for="member in detail.members" :key="member.archive.id" class="py-2.5 border-t border-[var(--border)] flex items-center gap-2.5">
          <div class="w-7 h-10 rounded-sm bg-[var(--bg-tertiary)] flex-shrink-0 overflow-hidden">
            <img v-if="memberCovers[member.archive.id]" :src="memberCovers[member.archive.id]" :alt="member.archive.title" class="w-full h-full object-cover" />
          </div>
          <div class="min-w-0 flex-1">
            <div class="text-xs font-medium text-[var(--text-primary)] truncate">{{ memberLabel(member) }}</div>
            <div class="mt-0.5 text-[10px] text-[var(--text-tertiary)] truncate">{{ member.archive.title }}</div>
          </div>
          <div class="text-[10px] text-right flex-shrink-0" :class="member.confidence >= 0.75 ? 'text-emerald-400' : 'text-amber-400'">
            {{ member.confidence >= 0.75 ? '高置信' : '待确认' }}
          </div>
          <div class="flex items-center gap-0.5">
            <button class="p-1 text-[var(--text-tertiary)] hover:text-[var(--text-primary)]" title="阅读" @click="openReader(member.archive.id)"><BookOpenIcon class="w-4 h-4" /></button>
            <button class="p-1 text-[var(--text-tertiary)] hover:text-red-400" title="从合集中移出" @click="$emit('removeMember', member.archive.id)"><XMarkIcon class="w-4 h-4" /></button>
          </div>
        </div>
      </div>
    </div>
  </BaseSidePanel>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { BookOpenIcon, XMarkIcon } from '@heroicons/vue/24/outline'
import BaseSidePanel from '@/components/base/BaseSidePanel.vue'
import type { CollectionDetail, CollectionMember } from '@/types/api'
import { getArchiveThumbnail } from '@/utils/api'

const props = defineProps<{
  show: boolean
  detail: CollectionDetail | null
  isLoading?: boolean
}>()
const emit = defineEmits<{ close: []; openReader: [archiveId: string]; removeMember: [archiveId: string]; viewVersions: [] }>()
const coverUrl = ref<string | null>(null)
const memberCovers = ref<Record<string, string>>({})
const nextMember = computed(() => props.detail?.members.find(member => member.confidence >= 0.75) || props.detail?.members[0])

const loadCovers = async (detail: CollectionDetail | null) => {
  coverUrl.value = null
  memberCovers.value = {}
  if (!detail) return
  const coverId = detail.collection.coverArchiveId || detail.members[0]?.archive.id
  if (coverId) coverUrl.value = await getArchiveThumbnail(coverId).catch(() => null)
  const entries = await Promise.all(detail.members.slice(0, 12).map(async member => [member.archive.id, await getArchiveThumbnail(member.archive.id).catch(() => '')] as const))
  memberCovers.value = Object.fromEntries(entries.filter(([, url]) => url))
}

const memberLabel = (member: CollectionMember) => {
  if (member.volumeNumber) return `第 ${member.volumeNumber} 卷${member.chapterNumber ? ` / 第 ${member.chapterNumber} 话` : ''}`
  if (member.chapterNumber) return `第 ${member.chapterNumber} 话`
  if (member.issueNumber) return `期号 ${member.issueNumber}`
  return member.rawNumber ? `编号 ${member.rawNumber}` : '未编号成员'
}
const openReader = (archiveId?: string) => { if (archiveId) emit('openReader', archiveId) }
watch(() => props.detail, detail => { void loadCovers(detail) }, { immediate: true })
onMounted(() => { if (props.detail) void loadCovers(props.detail) })
</script>
