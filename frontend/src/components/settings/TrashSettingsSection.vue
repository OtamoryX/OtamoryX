<template>
  <div class="space-y-6">
    <GlassCard size="md" radius="lg">
      <div class="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h2 class="text-lg font-medium text-[var(--text-primary)]">回收站</h2>
          <p class="mt-1 text-sm text-[var(--text-secondary)]">
            删除的漫画会先移入原目录下的隐藏回收站，默认保留 14 天；在此期间可以恢复。
          </p>
        </div>
        <button
          type="button"
          title="刷新回收站"
          class="rounded-md p-2 text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] hover:text-[var(--text-primary)] disabled:cursor-not-allowed disabled:opacity-50"
          :disabled="trashQuery.isFetching.value"
          @click="trashQuery.refetch()"
        >
          <ArrowPathIcon class="h-4 w-4" :class="{ 'animate-spin': trashQuery.isFetching.value }" />
        </button>
      </div>

      <div v-if="trashQuery.isLoading.value" class="mt-6 divide-y divide-[var(--border)] border-y border-[var(--border)]">
        <div v-for="index in 3" :key="index" class="h-20 animate-pulse bg-[var(--bg-tertiary)]/60" />
      </div>

      <div v-else-if="trashQuery.isError.value" class="mt-6 border-y border-[var(--border)] py-5 text-sm text-[var(--text-secondary)]">
        暂时无法读取回收站，请稍后重试。
      </div>

      <div v-else-if="entries.length === 0" class="mt-6 border-y border-[var(--border)] py-6 text-sm text-[var(--text-secondary)]">
        回收站目前是空的。
      </div>

      <div v-else class="mt-6 divide-y divide-[var(--border)] border-y border-[var(--border)]">
        <div v-for="entry in entries" :key="entry.id" class="flex items-start justify-between gap-4 py-4">
          <div class="min-w-0">
            <div class="truncate text-sm font-medium text-[var(--text-primary)]" :title="entryTitle(entry)">
              {{ entryTitle(entry) }}
            </div>
            <div class="mt-1 text-xs text-[var(--text-secondary)]">
              删除于 {{ formatDate(entry.deletedAt) }} · {{ expiryLabel(entry.expiresAt) }}
            </div>
            <div class="mt-1 truncate text-xs text-[var(--text-tertiary)]" :title="entry.originalPath">
              {{ entry.originalPath }}
            </div>
          </div>
          <GlassButton
            size="sm"
            variant="secondary"
            :disabled="restoreMutation.isPending.value"
            :loading="restoreMutation.isPending.value && restoringId === entry.id"
            loading-text="恢复中..."
            @click="restore(entry)"
          >
            <template #icon><ArrowUturnLeftIcon class="mr-1.5 h-4 w-4" /></template>
            {{ entry.operationId ? "恢复操作" : "恢复" }}
          </GlassButton>
        </div>
      </div>
    </GlassCard>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { useMutation, useQuery, useQueryClient } from "@tanstack/vue-query";
import { ArrowPathIcon, ArrowUturnLeftIcon } from "@heroicons/vue/24/outline";
import GlassCard from "@/components/base/GlassCard.vue";
import GlassButton from "@/components/base/GlassButton.vue";
import { listTrashEntries, restoreTrashEntry, restoreTrashOperation } from "@/utils/api";
import type { TrashEntry } from "@/types/api";

const queryClient = useQueryClient();
const restoringId = ref<string | null>(null);

const trashQuery = useQuery({
  queryKey: ["trash-entries", "active"],
  queryFn: () => listTrashEntries({ status: "active", limit: 100 }),
  staleTime: 30_000,
});

const restoreMutation = useMutation({
  mutationFn: async (entry: TrashEntry): Promise<TrashEntry[]> =>
    entry.operationId
      ? restoreTrashOperation(entry.operationId)
      : [await restoreTrashEntry(entry.id)],
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ["trash-entries"] });
    queryClient.invalidateQueries({ queryKey: ["randomArchives"] });
    queryClient.invalidateQueries({ queryKey: ["allArchivesCount"] });
  },
  onSettled: () => {
    restoringId.value = null;
  },
});

const entries = computed(() => trashQuery.data.value ?? []);

const restore = (entry: TrashEntry) => {
  restoringId.value = entry.id;
  restoreMutation.mutate(entry);
};

const parseMetadata = (entry: TrashEntry): Record<string, unknown> => {
  try {
    const parsed = JSON.parse(entry.metadataJson);
    return parsed && typeof parsed === "object" ? parsed as Record<string, unknown> : {};
  } catch {
    return {};
  }
};

const entryTitle = (entry: TrashEntry) => {
  const title = parseMetadata(entry).title;
  return typeof title === "string" && title.trim() ? title : entry.archiveId;
};

const formatDate = (value?: string) => {
  if (!value) return "未知时间";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "未知时间" : date.toLocaleString("zh-CN");
};

const expiryLabel = (value?: string) => value ? `预计 ${formatDate(value)} 自动清理` : "保留期限未知";
</script>
