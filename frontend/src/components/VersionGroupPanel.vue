<template>
  <BaseSidePanel :show="show" width="wide" title="多版本比较" @close="emit('close')">
    <div v-if="group" class="space-y-4">
      <div class="border-b border-[var(--border)] pb-4">
        <h2 class="text-base font-semibold text-[var(--text-primary)] break-words">{{ group.displayTitle }}</h2>
        <p v-if="group.subtitle" class="mt-1 text-sm text-[var(--text-tertiary)]">{{ group.subtitle }}</p>
        <p class="mt-2 text-xs text-[var(--text-secondary)]">{{ group.unitLabel }} · {{ group.members.length }} 个文件</p>
        <p class="mt-1 text-[11px] text-[var(--text-tertiary)]">选择最多 4 个版本并排比较；保留版本用于后续清理，不会影响并排阅读</p>
      </div>

      <article v-for="member in group.members" :key="member.archive.id" class="border rounded p-3 transition-colors" :class="selectedId === member.archive.id ? 'border-[var(--accent)] bg-[var(--accent)]/10' : 'border-[var(--border)] hover:bg-[var(--bg-tertiary)]'">
        <div class="flex gap-3">
          <label class="mt-0.5 flex h-5 items-center gap-1.5 text-[10px] text-[var(--text-tertiary)]">
            <input :checked="compareIds.includes(member.archive.id)" type="checkbox" :disabled="!compareIds.includes(member.archive.id) && compareIds.length >= 4" class="h-3.5 w-3.5 accent-[var(--accent)] disabled:opacity-40" @change="toggleCompare(member.archive.id)" />
            比较
          </label>
          <div class="min-w-0 flex-1">
            <div class="flex items-start justify-between gap-3">
              <div class="min-w-0">
                <div class="text-sm font-semibold text-[var(--text-primary)] truncate">{{ member.archive.title }}</div>
                <div class="mt-1 text-xs text-[var(--text-tertiary)] truncate">{{ member.archive.path }}</div>
              </div>
              <div class="flex shrink-0 items-center gap-1">
                <button v-if="canManage" class="inline-flex h-7 items-center gap-1 px-2 text-[10px] transition-colors" :class="selectedId === member.archive.id ? 'text-[var(--accent)]' : 'text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)]'" :title="selectedId === member.archive.id ? '当前保留版本' : '设为保留版本'" @click="selectedId = member.archive.id">
                  <CheckIcon class="h-3.5 w-3.5" />{{ selectedId === member.archive.id ? '保留' : '设为保留' }}
                </button>
                <button class="p-1 text-[var(--text-tertiary)] hover:text-[var(--text-primary)]" title="阅读" @click="emit('open-reader', member.archive.id)"><BookOpenIcon class="w-4 h-4" /></button>
              </div>
            </div>
            <div class="mt-2 text-xs text-[var(--text-secondary)]">{{ member.archive.pageCount }} 页 · {{ formatSize(member.archive.fileSize) }} · {{ extension(member.archive.path) }}</div>
            <div v-if="member.isRecommended" class="mt-2 text-xs text-emerald-400">推荐保留：{{ member.recommendationReasons.join('；') }}</div>
          </div>
        </div>
      </article>

      <button v-if="compareIds.length > 1" class="inline-flex h-9 items-center gap-1.5 border border-[var(--accent)]/50 px-3 text-xs text-[var(--accent)] hover:bg-[var(--accent)]/10" @click="openComparison">
        并排打开 {{ compareIds.length }} 个版本
      </button>

      <div v-if="canManage" class="flex flex-wrap gap-2 border-t border-[var(--border)] pt-3">
        <button v-if="group.status !== 'keep_all'" class="inline-flex h-9 items-center border border-[var(--border)] px-3 text-xs text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)]" @click="emit('keep-all', group.id)">全部保留</button>
        <button v-else class="inline-flex h-9 items-center border border-[var(--border)] px-3 text-xs text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)]" @click="emit('restore', group.id)">恢复待处理</button>
        <button class="inline-flex h-9 items-center bg-red-500 px-3 text-xs text-white hover:bg-red-400 disabled:opacity-50" :disabled="!selectedId || group.members.length < 2" @click="emit('cleanup', group, selectedId)">
          保留选中版本，删除另外 {{ group.members.length - 1 }} 本
        </button>
      </div>
      <p v-else class="text-xs text-[var(--text-tertiary)]">只有管理员可以执行版本清理。</p>
    </div>
  </BaseSidePanel>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { BookOpenIcon, CheckIcon } from '@heroicons/vue/24/outline'
import BaseSidePanel from '@/components/base/BaseSidePanel.vue'
import type { VersionGroup } from '@/types/api'

const props = defineProps<{ show: boolean; group: VersionGroup | null; canManage: boolean }>()
const emit = defineEmits<{ close: []; 'open-reader': [archiveId: string]; 'open-comparison': [groupId: string, archiveIds: string[], memberIds: string[]]; cleanup: [group: VersionGroup, keepArchiveId: string]; 'keep-all': [id: string]; restore: [id: string] }>()
const selectedId = ref('')
const compareIds = ref<string[]>([])
watch(() => props.group, (group) => {
  selectedId.value = group?.recommendedArchiveId || group?.members[0]?.archive.id || ''
  compareIds.value = group?.members.slice(0, 2).map(member => member.archive.id) || []
}, { immediate: true })
const openComparison = () => {
  if (!props.group) return
  emit('open-comparison', props.group.id, compareIds.value, props.group.members.map(member => member.archive.id))
}
const toggleCompare = (archiveId: string) => {
  if (compareIds.value.includes(archiveId)) {
    compareIds.value = compareIds.value.filter(id => id !== archiveId)
  } else if (compareIds.value.length < 4) {
    compareIds.value = [...compareIds.value, archiveId]
  }
}
const formatSize = (bytes: number) => bytes >= 1024 * 1024 ? `${(bytes / 1024 / 1024).toFixed(1)} MB` : `${Math.ceil(bytes / 1024)} KB`
const extension = (path: string) => path.split('.').pop()?.toUpperCase() || '文件'
</script>
