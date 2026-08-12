<template>
  <BaseSidePanel :show="show" width="wide" title="多版本比较" @close="emit('close')">
    <div v-if="group" class="space-y-4">
      <div class="border-b border-[var(--border)] pb-4">
        <h2 class="text-base font-semibold text-[var(--text-primary)] break-words">{{ group.displayTitle }}</h2>
        <p v-if="group.subtitle" class="mt-1 text-sm text-[var(--text-tertiary)]">{{ group.subtitle }}</p>
        <p class="mt-2 text-xs text-[var(--text-secondary)]">{{ group.unitLabel }} · {{ group.members.length }} 个文件</p>
        <p class="mt-1 text-[11px] text-[var(--text-tertiary)]">选择最多 4 个版本并排比较；点击版本卡片选择要保留的文件。</p>
      </div>

      <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <label v-for="(member, index) in group.members" :key="member.archive.id" class="relative block overflow-hidden border rounded cursor-pointer transition-colors" :class="selectedId === member.archive.id ? 'border-[var(--accent)] bg-[var(--accent)]/10 ring-1 ring-[var(--accent)]' : 'border-[var(--border)] hover:border-[var(--text-tertiary)] hover:bg-[var(--bg-tertiary)]'">
          <input v-model="selectedId" type="radio" name="version-keeper" :value="member.archive.id" class="sr-only" :disabled="!canManage" />
          <div class="flex gap-3 p-3">
            <div :ref="element => observeCover(member.archive.id, element as Element | null)" class="relative w-24 h-36 shrink-0 overflow-hidden rounded-sm bg-[var(--bg-tertiary)]">
              <img v-if="memberCovers[member.archive.id]" :src="memberCovers[member.archive.id]" :alt="member.archive.title" class="w-full h-full object-cover" />
              <div v-else class="w-full h-full flex items-center justify-center text-xs text-[var(--text-tertiary)]">无封面</div>
              <div v-if="selectedId === member.archive.id" class="absolute inset-x-0 bottom-0 bg-[var(--accent)] px-1.5 py-1 text-center text-[10px] font-medium text-white">保留此版本</div>
            </div>
            <div class="min-w-0 flex-1">
              <div class="flex items-start justify-between gap-2"><div class="min-w-0"><div class="text-xs text-[var(--text-tertiary)]">版本 {{ index + 1 }}</div><div class="mt-0.5 text-sm font-semibold text-[var(--text-primary)] break-words">{{ member.archive.title }}</div></div><button class="p-1 text-[var(--text-tertiary)] hover:text-[var(--text-primary)]" title="阅读" @click.prevent="emit('open-reader', member.archive.id)"><BookOpenIcon class="w-4 h-4" /></button></div>
              <div class="mt-2 text-xs text-[var(--text-secondary)]">{{ member.archive.pageCount }} 页 · {{ formatSize(member.archive.fileSize) }} · {{ extension(member.archive.path) }}</div>
              <div class="mt-2 text-[11px] leading-4 text-[var(--text-tertiary)] break-all">{{ member.archive.path }}</div>
              <label class="mt-2 flex items-center gap-1.5 text-[10px] text-[var(--text-tertiary)]" @click.stop><input :checked="compareIds.includes(member.archive.id)" type="checkbox" :disabled="!compareIds.includes(member.archive.id) && compareIds.length >= 4" class="h-3.5 w-3.5 accent-[var(--accent)] disabled:opacity-40" @change="toggleCompare(member.archive.id)" />比较</label>
              <div v-if="member.isRecommended" class="mt-2 text-xs text-emerald-400">推荐保留：{{ member.recommendationReasons.join('；') }}</div>
            </div>
          </div>
        </label>
      </div>
      <button v-if="compareIds.length > 1" class="inline-flex h-9 items-center gap-1.5 border border-[var(--accent)]/50 px-3 text-xs text-[var(--accent)] hover:bg-[var(--accent)]/10" @click="openComparison">并排打开 {{ compareIds.length }} 个版本</button>
      <div v-if="canManage" class="flex flex-wrap gap-2 border-t border-[var(--border)] pt-3"><button v-if="group.status !== 'keep_all'" class="inline-flex h-9 items-center border border-[var(--border)] px-3 text-xs text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)]" @click="emit('keep-all', group.id)">全部保留</button><button v-else class="inline-flex h-9 items-center border border-[var(--border)] px-3 text-xs text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)]" @click="emit('restore', group.id)">恢复待处理</button><button class="inline-flex h-9 items-center bg-red-500 px-3 text-xs text-white hover:bg-red-400 disabled:opacity-50" :disabled="!selectedId || group.members.length < 2" @click="emit('cleanup', group, selectedId)">保留选中版本，删除另外 {{ group.members.length - 1 }} 本</button></div>
      <p v-else class="text-xs text-[var(--text-tertiary)]">只有管理员可以执行版本清理。</p>
    </div>
  </BaseSidePanel>
</template>

<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue'
import { BookOpenIcon } from '@heroicons/vue/24/outline'
import BaseSidePanel from '@/components/base/BaseSidePanel.vue'
import type { VersionGroup } from '@/types/api'
import { getArchiveThumbnail } from '@/utils/api'

const props = defineProps<{ show: boolean; group: VersionGroup | null; canManage: boolean }>()
const emit = defineEmits<{ close: []; 'open-reader': [archiveId: string]; 'open-comparison': [groupId: string, archiveIds: string[], memberIds: string[]]; cleanup: [group: VersionGroup, keepArchiveId: string]; 'keep-all': [id: string]; restore: [id: string] }>()
const selectedId = ref('')
const compareIds = ref<string[]>([])
const memberCovers = ref<Record<string, string>>({})
const loadedCoverIds = new Set<string>(); const loadingCoverIds = new Set<string>(); let coverObserver: IntersectionObserver | null = null; let coverGeneration = 0
const clearCovers = () => { coverGeneration += 1; Object.values(memberCovers.value).forEach(URL.revokeObjectURL); memberCovers.value = {}; loadedCoverIds.clear(); loadingCoverIds.clear(); coverObserver?.disconnect(); coverObserver = null }
const loadCover = async (archiveId: string) => { if (loadedCoverIds.has(archiveId) || loadingCoverIds.has(archiveId)) return; const generation = coverGeneration; loadingCoverIds.add(archiveId); try { const url = await getArchiveThumbnail(archiveId); if (generation !== coverGeneration) { URL.revokeObjectURL(url); return }; memberCovers.value = { ...memberCovers.value, [archiveId]: url }; loadedCoverIds.add(archiveId) } catch { if (generation === coverGeneration) loadedCoverIds.add(archiveId) } finally { if (generation === coverGeneration) loadingCoverIds.delete(archiveId) } }
const observeCover = (archiveId: string, element: Element | null) => { if (!element || loadedCoverIds.has(archiveId)) return; if (!coverObserver && typeof IntersectionObserver !== 'undefined') coverObserver = new IntersectionObserver(entries => { for (const entry of entries) { if (!entry.isIntersecting) continue; const id = (entry.target as HTMLElement).dataset.archiveId; if (id) void loadCover(id); coverObserver?.unobserve(entry.target) } }, { rootMargin: '160px' }); if (!coverObserver) { void loadCover(archiveId); return }; (element as HTMLElement).dataset.archiveId = archiveId; coverObserver.observe(element) }
const toggleCompare = (archiveId: string) => { compareIds.value = compareIds.value.includes(archiveId) ? compareIds.value.filter(id => id !== archiveId) : compareIds.value.length < 4 ? [...compareIds.value, archiveId] : compareIds.value }
const openComparison = () => { if (props.group) emit('open-comparison', props.group.id, compareIds.value, props.group.members.map(member => member.archive.id)) }
watch(() => props.group, group => { clearCovers(); selectedId.value = group?.recommendedArchiveId || group?.members[0]?.archive.id || ''; compareIds.value = group?.members.slice(0, 2).map(member => member.archive.id) || [] }, { immediate: true })
onBeforeUnmount(clearCovers)
const formatSize = (bytes: number) => bytes >= 1024 * 1024 ? `${(bytes / 1024 / 1024).toFixed(1)} MB` : `${Math.ceil(bytes / 1024)} KB`
const extension = (path: string) => path.split('.').pop()?.toUpperCase() || '文件'
</script>
