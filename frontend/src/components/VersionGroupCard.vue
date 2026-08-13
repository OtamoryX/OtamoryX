<template>
  <div
    class="group-card overflow-hidden rounded border bg-[var(--bg-card)] transition-colors"
    :class="
      selected
        ? 'border-[var(--accent)]'
        : 'border-[var(--border)] hover:border-[var(--accent)]'
    "
    @click="emit('open', group)"
  >
    <div
      class="relative mx-1 mt-1 aspect-[2/3] overflow-hidden rounded-sm bg-[var(--bg-tertiary)]"
    >
      <img
        v-if="coverUrl"
        :src="coverUrl"
        :alt="group.displayTitle"
        class="h-full w-full object-cover"
      />
      <div
        v-else
        class="flex h-full w-full items-center justify-center text-[var(--text-tertiary)]"
      >
        <svg
          class="h-10 w-10"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="1.5"
            d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"
          />
        </svg>
      </div>
      <label
        class="absolute left-1 top-1 flex h-5 w-5 cursor-pointer items-center justify-center rounded-sm bg-black/45"
        @click.stop
      >
        <input
          :checked="selected"
          type="checkbox"
          class="h-3.5 w-3.5 accent-[var(--accent)]"
          aria-label="选择多版本组"
          @change="emit('toggle', group.id)"
        />
      </label>
    </div>
    <div class="px-2 pb-2 pt-1.5">
      <h3
        class="line-clamp-2 min-h-8 text-xs font-semibold leading-4 text-[var(--text-primary)] [overflow-wrap:anywhere]"
        :title="group.displayTitle"
      >
        {{ group.displayTitle }}
      </h3>
      <div class="mt-1 flex items-center justify-between gap-1">
        <span class="min-w-0 truncate text-[10px] text-[var(--text-tertiary)]">
          {{ group.unitLabel }} · {{ group.members.length }} 本<span
            v-if="group.matchedMemberCount < group.members.length"
          >
            · 命中 {{ group.matchedMemberCount }}</span
          >
        </span>
        <span
          class="shrink-0 text-[10px]"
          :class="
            group.recommendedArchiveId
              ? 'text-emerald-400'
              : group.status === 'keep_all'
                ? 'text-[var(--text-secondary)]'
                : 'text-amber-400'
          "
        >
          {{
            group.recommendedArchiveId
              ? "已推荐保留"
              : group.status === "keep_all"
                ? "无需处理"
                : "需要人工比较"
          }}
        </span>
      </div>
    </div>
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
onMounted(async () => {
  if (coverId.value)
    coverUrl.value = await getArchiveThumbnail(coverId.value).catch(() => null);
});
</script>

<style scoped>
.line-clamp-2 {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}
</style>
