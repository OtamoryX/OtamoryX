<template>
  <div
    class="w-full border border-[var(--border)] bg-[var(--bg-card)] hover:border-[var(--accent)] transition-colors rounded p-2 flex gap-2"
  >
    <label class="flex items-start pt-0.5" @click.stop>
      <input
        :checked="selected"
        type="checkbox"
        class="h-3.5 w-3.5 accent-[var(--accent)]"
        aria-label="选择多版本组"
        @change="emit('toggle', group.id)"
      />
    </label>
    <button
      class="min-w-0 flex flex-1 gap-2.5 text-left"
      @click="emit('open', group)"
    >
      <div
        class="w-9 h-[54px] flex-shrink-0 rounded-sm overflow-hidden bg-[var(--bg-tertiary)]"
      >
        <img
          v-if="coverUrl"
          :src="coverUrl"
          :alt="group.displayTitle"
          class="w-full h-full object-cover"
        />
      </div>
      <div class="min-w-0 flex-1">
        <div class="flex items-center justify-between gap-2">
          <div
            class="min-w-0 text-sm font-semibold text-[var(--text-primary)] truncate"
          >
            {{ group.displayTitle }}
          </div>
          <div class="shrink-0 text-xs text-[var(--text-secondary)]">
            {{ group.unitLabel }} · {{ group.members.length }} 本<span
              v-if="group.matchedMemberCount < group.members.length"
            >
              · 命中 {{ group.matchedMemberCount }}</span
            >
          </div>
        </div>
        <div class="mt-0.5 flex items-center justify-between gap-2">
          <div
            v-if="group.subtitle"
            class="min-w-0 truncate text-xs text-[var(--text-tertiary)]"
          >
            {{ group.subtitle }}
          </div>
          <div v-else class="min-w-0" />
          <div
            class="shrink-0 text-[10px]"
            :class="
              group.recommendedArchiveId ? 'text-emerald-400' : 'text-amber-400'
            "
          >
            {{
              group.recommendedArchiveId
                ? `已推荐保留 · 可释放 ${formatSize(group.reclaimableSize)}`
                : group.status === "keep_all"
                  ? "无需处理"
                  : "需要人工比较"
            }}
          </div>
        </div>
      </div>
    </button>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import type { VersionGroup } from "@/types/api";
import { getArchiveThumbnail } from "@/utils/api";

const props = defineProps<{ group: VersionGroup; selected?: boolean }>();
const emit = defineEmits<{
  open: [group: VersionGroup];
  toggle: [id: string];
}>();
const coverUrl = ref<string | null>(null);
const coverId = computed(
  () => props.group.recommendedArchiveId || props.group.members[0]?.archive.id,
);
const formatSize = (bytes: number) =>
  bytes >= 1024 * 1024
    ? `${(bytes / 1024 / 1024).toFixed(1)} MB`
    : `${Math.ceil(bytes / 1024)} KB`;
onMounted(async () => {
  if (coverId.value)
    coverUrl.value = await getArchiveThumbnail(coverId.value).catch(() => null);
});
</script>
