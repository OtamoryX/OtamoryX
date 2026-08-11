<template>
  <div class="w-full border border-[var(--border)] bg-[var(--bg-card)] hover:border-[var(--accent)] transition-colors rounded p-3 flex gap-3">
    <label class="flex items-start pt-0.5" @click.stop>
      <input :checked="selected" type="checkbox" class="h-3.5 w-3.5 accent-[var(--accent)]" aria-label="选择多版本组" @change="emit('toggle', group.id)" />
    </label>
    <button class="min-w-0 flex flex-1 gap-3 text-left" @click="emit('open', group)">
    <div class="w-12 h-[72px] flex-shrink-0 rounded-sm overflow-hidden bg-[var(--bg-tertiary)]">
      <img v-if="coverUrl" :src="coverUrl" :alt="group.displayTitle" class="w-full h-full object-cover" />
    </div>
    <div class="min-w-0 flex-1">
      <div class="text-sm font-semibold text-[var(--text-primary)] truncate">{{ group.displayTitle }}</div>
      <div v-if="group.subtitle" class="mt-0.5 text-xs text-[var(--text-tertiary)] truncate">{{ group.subtitle }}</div>
      <div class="mt-1 text-xs text-[var(--text-secondary)]">{{ group.unitLabel }} · {{ group.members.length }} 个文件</div>
      <div class="mt-1 text-[10px]" :class="group.recommendedArchiveId ? 'text-emerald-400' : 'text-amber-400'">
        {{ group.recommendedArchiveId ? `已推荐保留 · 可释放 ${formatSize(group.reclaimableSize)}` : group.status === 'keep_all' ? '全部保留' : '需要人工选择' }}
      </div>
    </div>
    </button>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import type { VersionGroup } from '@/types/api'
import { getArchiveThumbnail } from '@/utils/api'

const props = defineProps<{ group: VersionGroup; selected?: boolean }>()
const emit = defineEmits<{ open: [group: VersionGroup]; toggle: [id: string] }>()
const coverUrl = ref<string | null>(null)
const coverId = computed(() => props.group.recommendedArchiveId || props.group.members[0]?.archive.id)
const formatSize = (bytes: number) => bytes >= 1024 * 1024 ? `${(bytes / 1024 / 1024).toFixed(1)} MB` : `${Math.ceil(bytes / 1024)} KB`
onMounted(async () => { if (coverId.value) coverUrl.value = await getArchiveThumbnail(coverId.value).catch(() => null) })
</script>
