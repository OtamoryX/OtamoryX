<template>
  <BaseSidePanel :show="show" width="wide" title="多版本比较" @close="emit('close')">
    <div v-if="group" class="space-y-4">
      <div class="border-b border-[var(--border)] pb-4">
        <h2 class="text-base font-semibold text-[var(--text-primary)] break-words">{{ group.displayTitle }}</h2>
        <p v-if="group.subtitle" class="mt-1 text-sm text-[var(--text-tertiary)]">{{ group.subtitle }}</p>
        <p class="mt-2 text-xs text-[var(--text-secondary)]">{{ group.unitLabel }} · {{ group.members.length }} 个文件</p>
      </div>

      <label v-for="member in group.members" :key="member.archive.id" class="block border rounded p-3 cursor-pointer transition-colors" :class="selectedId === member.archive.id ? 'border-[var(--accent)] bg-[var(--accent)]/10' : 'border-[var(--border)] hover:bg-[var(--bg-tertiary)]'">
        <div class="flex gap-3">
          <input v-model="selectedId" type="radio" name="version-keeper" :value="member.archive.id" class="mt-1 accent-[var(--accent)]" :disabled="!canManage" />
          <div class="min-w-0 flex-1">
            <div class="flex items-start justify-between gap-3">
              <div class="min-w-0">
                <div class="text-sm font-semibold text-[var(--text-primary)] truncate">{{ member.archive.title }}</div>
                <div class="mt-1 text-xs text-[var(--text-tertiary)] truncate">{{ member.archive.path }}</div>
              </div>
              <button class="p-1 text-[var(--text-tertiary)] hover:text-[var(--text-primary)]" title="阅读" @click.prevent="emit('open-reader', member.archive.id)"><BookOpenIcon class="w-4 h-4" /></button>
            </div>
            <div class="mt-2 text-xs text-[var(--text-secondary)]">{{ member.archive.pageCount }} 页 · {{ formatSize(member.archive.fileSize) }} · {{ extension(member.archive.path) }}</div>
            <div v-if="member.isRecommended" class="mt-2 text-xs text-emerald-400">推荐保留：{{ member.recommendationReasons.join('；') }}</div>
          </div>
        </div>
      </label>

      <div v-if="canManage" class="pt-2 flex flex-wrap gap-2">
        <button class="px-3 py-2 rounded text-sm border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)]" @click="emit('keep-all', group.id)">这些版本都保留</button>
        <button class="px-3 py-2 rounded text-sm bg-red-500 text-white hover:bg-red-400 disabled:opacity-50" :disabled="!selectedId || group.members.length < 2" @click="emit('cleanup', group, selectedId)">
          保留选中版本，删除另外 {{ group.members.length - 1 }} 本
        </button>
      </div>
      <p v-else class="text-xs text-[var(--text-tertiary)]">只有管理员可以执行版本清理。</p>
    </div>
  </BaseSidePanel>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { BookOpenIcon } from '@heroicons/vue/24/outline'
import BaseSidePanel from '@/components/base/BaseSidePanel.vue'
import type { VersionGroup } from '@/types/api'

const props = defineProps<{ show: boolean; group: VersionGroup | null; canManage: boolean }>()
const emit = defineEmits<{ close: []; 'open-reader': [archiveId: string]; cleanup: [group: VersionGroup, keepArchiveId: string]; 'keep-all': [id: string] }>()
const selectedId = ref('')
watch(() => props.group, (group) => { selectedId.value = group?.recommendedArchiveId || group?.members[0]?.archive.id || '' }, { immediate: true })
const formatSize = (bytes: number) => bytes >= 1024 * 1024 ? `${(bytes / 1024 / 1024).toFixed(1)} MB` : `${Math.ceil(bytes / 1024)} KB`
const extension = (path: string) => path.split('.').pop()?.toUpperCase() || '文件'
</script>
