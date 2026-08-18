<template>
  <div class="space-y-6">
    <GlassCard size="md" radius="lg">
      <div class="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h2 class="text-lg font-medium text-[var(--text-primary)]">回收站</h2>
          <p class="mt-1 max-w-2xl text-sm text-[var(--text-secondary)]">
            删除的漫画会先保留 14 天。自动删除会保留规则、触发原因和证据页，便于你在恢复前核对。
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

      <div v-if="!trashQuery.isLoading.value && !trashQuery.isError.value" class="mt-6 grid grid-cols-1 gap-3 sm:grid-cols-3">
        <div class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)]/60 px-4 py-3">
          <div class="text-xs text-[var(--text-tertiary)]">待处理</div>
          <div class="mt-1 text-xl font-semibold text-[var(--text-primary)]">{{ entries.length }}</div>
        </div>
        <div class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)]/60 px-4 py-3">
          <div class="text-xs text-[var(--text-tertiary)]">自动删除</div>
          <div class="mt-1 text-xl font-semibold text-[var(--text-primary)]">{{ automaticCount }}</div>
        </div>
        <div class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)]/60 px-4 py-3">
          <div class="text-xs text-[var(--text-tertiary)]">7 天内到期</div>
          <div class="mt-1 text-xl font-semibold text-[var(--text-primary)]">{{ expiringSoonCount }}</div>
        </div>
      </div>

      <div v-if="trashQuery.isLoading.value" class="mt-6 divide-y divide-[var(--border)] border-y border-[var(--border)]">
        <div v-for="index in 3" :key="index" class="h-28 animate-pulse bg-[var(--bg-tertiary)]/60" />
      </div>

      <div v-else-if="trashQuery.isError.value" class="mt-6 border-y border-[var(--border)] py-5 text-sm text-[var(--text-secondary)]">
        暂时无法读取回收站，请稍后重试。
      </div>

      <div v-else-if="entries.length === 0" class="mt-6 border-y border-[var(--border)] py-8 text-center text-sm text-[var(--text-secondary)]">
        回收站目前是空的。
      </div>

      <div v-else class="mt-6 space-y-3">
        <article v-for="entry in entries" :key="entry.id" class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)]/35 p-4">
          <div class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
            <div class="min-w-0 flex-1">
              <div class="flex flex-wrap items-center gap-2">
                <span :class="['rounded-full border px-2 py-0.5 text-[11px] font-medium', sourceClass(entry)]">
                  {{ sourceLabel(entry) }}
                </span>
                <span v-if="entry.operationId" class="rounded-full border border-sky-400/30 bg-sky-500/10 px-2 py-0.5 text-[11px] text-sky-300">
                  版本清理操作
                </span>
                <span v-if="entry.modelConfidence != null" class="rounded-full border border-violet-400/30 bg-violet-500/10 px-2 py-0.5 text-[11px] text-violet-300">
                  置信度 {{ confidenceLabel(entry) }}
                </span>
              </div>
              <h3 class="mt-2 truncate text-sm font-semibold text-[var(--text-primary)]" :title="entryTitle(entry)">
                {{ entryTitle(entry) }}
              </h3>
              <div class="mt-1 text-xs text-[var(--text-secondary)]">
                删除于 {{ formatDate(entry.deletedAt) }} · {{ expiryLabel(entry.expiresAt) }}
              </div>
              <dl class="mt-3 grid grid-cols-1 gap-x-6 gap-y-2 text-xs sm:grid-cols-2">
                <div>
                  <dt class="text-[var(--text-tertiary)]">触发原因</dt>
                  <dd class="mt-0.5 text-[var(--text-secondary)]">{{ reasonLabel(entry) }}</dd>
                </div>
                <div>
                  <dt class="text-[var(--text-tertiary)]">模型依据</dt>
                  <dd class="mt-0.5 text-[var(--text-secondary)]">{{ evidenceLabel(entry) }}</dd>
                </div>
              </dl>
              <div class="mt-3 truncate text-xs text-[var(--text-tertiary)]" :title="entry.originalPath">
                {{ entry.originalPath }}
              </div>
            </div>

            <div class="flex shrink-0 flex-wrap items-center gap-2 lg:max-w-[300px] lg:justify-end">
              <GlassButton size="sm" variant="ghost" :disabled="previewLoading && previewEntry?.id === entry.id" @click="openPreview(entry)">
                <template #icon><EyeIcon class="mr-1.5 h-4 w-4" /></template>
                {{ previewLoading && previewEntry?.id === entry.id ? "加载中..." : "预览内容" }}
              </GlassButton>
              <GlassButton
                size="sm"
                variant="secondary"
                :disabled="restoreMutation.isPending.value || purgeMutation.isPending.value"
                :loading="restoreMutation.isPending.value && restoringId === entry.id"
                loading-text="恢复中..."
                @click="restore(entry)"
              >
                <template #icon><ArrowUturnLeftIcon class="mr-1.5 h-4 w-4" /></template>
                {{ entry.operationId ? "恢复操作" : "恢复" }}
              </GlassButton>
              <GlassButton
                size="sm"
                variant="danger"
                :disabled="restoreMutation.isPending.value || purgeMutation.isPending.value"
                :loading="purgeMutation.isPending.value && purgingId === entry.id"
                loading-text="删除中..."
                @click="requestPurge(entry)"
              >
                <template #icon><TrashIcon class="mr-1.5 h-4 w-4" /></template>
                {{ entry.operationId ? "立即删除操作" : "立即删除" }}
              </GlassButton>
            </div>
          </div>
        </article>
      </div>

      <p v-if="actionError" class="mt-4 text-sm text-red-400">{{ actionError }}</p>
    </GlassCard>

    <BaseModal :show="Boolean(purgeTarget)" title="确认立即删除" width="md" @close="purgeTarget = null">
      <div class="space-y-4 text-sm text-[var(--text-secondary)]">
        <p>这会永久删除回收站中的文件，之后无法恢复。</p>
        <p v-if="purgeTarget" class="rounded-lg border border-red-400/30 bg-red-500/10 p-3 text-red-200">
          {{ entryTitle(purgeTarget) }}{{ purgeTarget.operationId ? "及同一版本清理操作中的其他文件" : "" }}
        </p>
        <div class="flex justify-end gap-2">
          <GlassButton variant="ghost" @click="purgeTarget = null">取消</GlassButton>
          <GlassButton variant="danger" :loading="purgeMutation.isPending.value" loading-text="删除中..." @click="confirmPurge">永久删除</GlassButton>
        </div>
      </div>
    </BaseModal>

    <BaseModal :show="Boolean(previewEntry)" :title="previewEntry ? `预览：${entryTitle(previewEntry)}` : '预览内容'" width="xl" max-height="screen" @close="closePreview">
      <div v-if="previewEntry" class="space-y-4">
        <div class="flex flex-wrap items-center justify-between gap-3 text-xs text-[var(--text-secondary)]">
          <span>{{ previewIsEvidence ? "当前为自动删除证据页" : "只读预览，不会移出回收站" }}</span>
          <span>第 {{ previewPage }} 页<span v-if="previewPageCount"> / {{ previewPageCount }}</span></span>
        </div>
        <div class="flex min-h-[45vh] items-center justify-center rounded-lg bg-black/80 p-2">
          <div v-if="previewLoading" class="text-sm text-white/70">正在读取页面...</div>
          <div v-else-if="previewError" class="text-sm text-red-300">{{ previewError }}</div>
          <img v-else-if="previewUrl" :src="previewUrl" :alt="`第 ${previewPage} 页`" class="max-h-[65vh] max-w-full object-contain" />
        </div>
        <div class="flex items-center justify-between gap-2">
          <GlassButton size="sm" variant="secondary" :disabled="previewLoading || previewPage <= 1" @click="changePreviewPage(-1)">上一页</GlassButton>
          <GlassButton size="sm" variant="secondary" :disabled="previewLoading || (previewPageCount > 0 && previewPage >= previewPageCount)" @click="changePreviewPage(1)">下一页</GlassButton>
        </div>
      </div>
    </BaseModal>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from "vue";
import { useMutation, useQuery, useQueryClient } from "@tanstack/vue-query";
import { ArrowPathIcon, ArrowUturnLeftIcon, EyeIcon, TrashIcon } from "@heroicons/vue/24/outline";
import BaseModal from "@/components/base/BaseModal.vue";
import GlassCard from "@/components/base/GlassCard.vue";
import GlassButton from "@/components/base/GlassButton.vue";
import { getApiErrorMessage } from "@/utils/error";
import { getTrashPage, listTrashEntries, purgeTrashEntry, purgeTrashOperation, restoreTrashEntry, restoreTrashOperation } from "@/utils/api";
import type { TrashEntry } from "@/types/api";

const queryClient = useQueryClient();
const restoringId = ref<string | null>(null);
const purgingId = ref<string | null>(null);
const actionError = ref<string | null>(null);
const purgeTarget = ref<TrashEntry | null>(null);
const previewEntry = ref<TrashEntry | null>(null);
const previewPage = ref(1);
const previewPageCount = ref(0);
const previewUrl = ref<string | null>(null);
const previewLoading = ref(false);
const previewError = ref<string | null>(null);

const trashQuery = useQuery({
  queryKey: ["trash-entries", "active"],
  queryFn: () => listTrashEntries({ status: "active", limit: 100 }),
  staleTime: 30_000,
});

const invalidateTrash = () => {
  queryClient.invalidateQueries({ queryKey: ["trash-entries"] });
  queryClient.invalidateQueries({ queryKey: ["randomArchives"] });
  queryClient.invalidateQueries({ queryKey: ["allArchivesCount"] });
};

const restoreMutation = useMutation({
  mutationFn: async (entry: TrashEntry): Promise<TrashEntry[]> =>
    entry.operationId ? restoreTrashOperation(entry.operationId) : [await restoreTrashEntry(entry.id)],
  onSuccess: invalidateTrash,
  onError: (error) => { actionError.value = getApiErrorMessage(error, "恢复失败，请稍后重试"); },
  onSettled: () => { restoringId.value = null; },
});

const purgeMutation = useMutation({
  mutationFn: async (entry: TrashEntry): Promise<void> =>
    entry.operationId ? purgeTrashOperation(entry.operationId) : purgeTrashEntry(entry.id),
  onSuccess: () => { purgeTarget.value = null; invalidateTrash(); },
  onError: (error) => { actionError.value = getApiErrorMessage(error, "永久删除失败，文件仍保留在回收站"); },
  onSettled: () => { purgingId.value = null; },
});

const entries = computed(() => trashQuery.data.value ?? []);
const automaticCount = computed(() => entries.value.filter((entry) => sourceLabel(entry) === "自动删除").length);
const expiringSoonCount = computed(() => entries.value.filter((entry) => {
  if (!entry.expiresAt) return false;
  const remaining = new Date(entry.expiresAt).getTime() - Date.now();
  return remaining >= 0 && remaining <= 7 * 24 * 60 * 60 * 1000;
}).length);

const parseMetadata = (entry: TrashEntry): Record<string, unknown> => {
  try {
    const parsed = JSON.parse(entry.metadataJson);
    return parsed && typeof parsed === "object" ? parsed as Record<string, unknown> : {};
  } catch { return {}; }
};

const metadataString = (entry: TrashEntry, key: string) => {
  const value = parseMetadata(entry)[key];
  return typeof value === "string" ? value : undefined;
};

const evidencePages = (entry: TrashEntry) => {
  const value = parseMetadata(entry).evidence_pages;
  return Array.isArray(value) ? value.filter((page): page is number => typeof page === "number" && page > 0) : [];
};

const previewIsEvidence = computed(() => previewEntry.value ? evidencePages(previewEntry.value).includes(previewPage.value) : false);

const entryTitle = (entry: TrashEntry) => {
  const title = parseMetadata(entry).title;
  return typeof title === "string" && title.trim() ? title : entry.archiveId;
};

const sourceLabel = (entry: TrashEntry) => {
  const source = metadataString(entry, "source");
  if (source === "auto_delete" || entry.ruleId) return "自动删除";
  if (source === "version_cleanup" || entry.operationType === "version_cleanup") return "版本清理";
  return "手动删除";
};

const sourceClass = (entry: TrashEntry) => sourceLabel(entry) === "自动删除"
  ? "border-violet-400/30 bg-violet-500/10 text-violet-300"
  : sourceLabel(entry) === "版本清理"
    ? "border-sky-400/30 bg-sky-500/10 text-sky-300"
    : "border-amber-400/30 bg-amber-500/10 text-amber-300";

const reasonLabel = (entry: TrashEntry) => {
  if (sourceLabel(entry) === "版本清理") return "按版本清理策略移入回收站";
  if (sourceLabel(entry) === "手动删除") return entry.reason || "用户主动删除";
  return entry.reason || "匹配自动删除规则";
};

const evidenceLabel = (entry: TrashEntry) => {
  const pages = evidencePages(entry);
  if (sourceLabel(entry) !== "自动删除") return "未使用模型证据";
  const rule = entry.ruleId
    ? `规则 ${entry.ruleId}${entry.ruleVersion ? ` · v${entry.ruleVersion}` : ""}`
    : "偏好规则";
  return pages.length ? `${rule} · 第 ${pages.join("、")} 页` : rule;
};

const confidenceLabel = (entry: TrashEntry) => entry.modelConfidence == null ? "未知" : `${Math.round(entry.modelConfidence * 100)}%`;
const formatDate = (value?: string) => {
  if (!value) return "未知时间";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "未知时间" : date.toLocaleString("zh-CN");
};
const expiryLabel = (value?: string) => value ? `预计 ${formatDate(value)} 自动清理` : "保留期限未知";
const previewPageCountFor = (entry: TrashEntry) => {
  const value = parseMetadata(entry).page_count;
  return typeof value === "number" && value > 0 ? value : 0;
};

const restore = (entry: TrashEntry) => {
  actionError.value = null;
  restoringId.value = entry.id;
  restoreMutation.mutate(entry);
};
const requestPurge = (entry: TrashEntry) => {
  actionError.value = null;
  purgeTarget.value = entry;
};
const confirmPurge = () => {
  if (!purgeTarget.value) return;
  purgingId.value = purgeTarget.value.id;
  purgeMutation.mutate(purgeTarget.value);
};

const loadPreview = async () => {
  if (!previewEntry.value) return;
  previewLoading.value = true;
  previewError.value = null;
  if (previewUrl.value) URL.revokeObjectURL(previewUrl.value);
  previewUrl.value = null;
  try {
    previewUrl.value = await getTrashPage(previewEntry.value.id, previewPage.value);
  } catch (error) {
    previewError.value = getApiErrorMessage(error, "无法读取该页面，可能已被清理");
  } finally { previewLoading.value = false; }
};
const openPreview = async (entry: TrashEntry) => {
  previewEntry.value = entry;
  previewPageCount.value = previewPageCountFor(entry);
  previewPage.value = evidencePages(entry)[0] ?? 1;
  await loadPreview();
};
const changePreviewPage = async (delta: number) => {
  const next = previewPage.value + delta;
  if (next < 1 || (previewPageCount.value > 0 && next > previewPageCount.value)) return;
  previewPage.value = next;
  await loadPreview();
};
const closePreview = () => {
  if (previewUrl.value) URL.revokeObjectURL(previewUrl.value);
  previewUrl.value = null;
  previewEntry.value = null;
  previewError.value = null;
};
onBeforeUnmount(closePreview);
</script>
