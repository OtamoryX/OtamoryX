<template>
  <div class="min-h-screen bg-[var(--bg-secondary)]">
    <main class="mx-auto w-full max-w-5xl px-3 pb-10 pt-6 sm:px-6 lg:px-8">
      <header class="mb-5 flex items-center justify-between gap-3">
        <div class="min-w-0">
          <h1 class="truncate text-xl font-semibold text-[var(--text-primary)] sm:text-2xl">
            阅读记录
          </h1>
        </div>
        <RouterLink
          to="/library"
          class="inline-flex min-h-9 shrink-0 items-center gap-1.5 rounded border border-[var(--border)] px-3 text-sm text-[var(--text-secondary)] transition-colors hover:bg-[var(--bg-tertiary)] hover:text-[var(--text-primary)]"
        >
          <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
          </svg>
          书库
        </RouterLink>
      </header>

      <div class="mb-4 flex gap-1 overflow-x-auto border-b border-[var(--border)]" role="tablist" aria-label="阅读记录筛选">
        <button
          v-for="tab in tabs"
          :key="tab.value"
          type="button"
          role="tab"
          :aria-selected="selectedStatus === tab.value"
          class="min-h-10 shrink-0 border-b-2 px-3 text-sm transition-colors"
          :class="selectedStatus === tab.value
            ? 'border-[var(--accent)] text-[var(--text-primary)]'
            : 'border-transparent text-[var(--text-secondary)] hover:text-[var(--text-primary)]'"
          @click="selectedStatus = tab.value"
        >
          {{ tab.label }}
        </button>
      </div>

      <div v-if="isLoading" class="py-16 text-center text-sm text-[var(--text-tertiary)]">
        加载中...
      </div>

      <div v-else-if="error" class="py-16 text-center">
        <p class="text-sm text-red-400">阅读记录加载失败</p>
        <button
          type="button"
          class="mt-3 min-h-9 rounded border border-[var(--border)] px-3 text-sm text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)]"
          @click="refetch()"
        >
          重试
        </button>
      </div>

      <div v-else-if="historyItems.length === 0" class="py-16 text-center text-sm text-[var(--text-tertiary)]">
        暂无阅读记录
      </div>

      <div v-else class="space-y-2">
        <button
          v-for="item in historyItems"
          :key="item.archiveId"
          type="button"
          class="group flex w-full items-center gap-3 rounded border border-[var(--border)] bg-[var(--bg-card)] p-2.5 text-left transition-colors hover:border-[var(--accent)] sm:gap-4 sm:p-3"
          :aria-label="`继续阅读 ${displayTitle(item)}`"
          @click="continueReading(item.archiveId)"
        >
          <div class="h-24 w-16 shrink-0 overflow-hidden rounded-sm bg-[var(--bg-tertiary)] sm:h-32 sm:w-[5.333rem]">
            <img
              v-if="coverUrls[item.archiveId]"
              :src="coverUrls[item.archiveId]"
              :alt="displayTitle(item)"
              class="h-full w-full object-cover"
              @error="removeCover(item.archiveId)"
            />
            <div v-else class="flex h-full w-full items-center justify-center text-[var(--text-tertiary)]">
              <svg class="h-7 w-7" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
              </svg>
            </div>
          </div>

          <div class="min-w-0 flex-1 self-stretch py-0.5 sm:py-1">
            <div class="flex items-start justify-between gap-2">
              <div class="min-w-0">
                <h2 class="truncate text-sm font-semibold text-[var(--text-primary)] sm:text-base">
                  {{ displayTitle(item) }}
                </h2>
                <p v-if="displaySubtitle(item)" class="mt-0.5 truncate text-xs text-[var(--text-tertiary)]">
                  {{ displaySubtitle(item) }}
                </p>
              </div>
              <span
                class="shrink-0 rounded-sm px-1.5 py-0.5 text-[11px]"
                :class="item.status === 'read'
                  ? 'bg-emerald-500/10 text-emerald-500'
                  : 'bg-[var(--accent)]/10 text-[var(--accent)]'"
              >
                {{ item.status === "read" ? "已读" : "阅读中" }}
              </span>
            </div>

            <div class="mt-3 flex items-center justify-between gap-2 text-xs text-[var(--text-secondary)]">
              <span class="truncate">第 {{ item.currentPage }} / {{ item.pageCount || item.totalPages }} 页</span>
              <span class="shrink-0">{{ progressPercent(item) }}%</span>
            </div>
            <div class="mt-1.5 h-1 overflow-hidden rounded-full bg-[var(--bg-tertiary)]">
              <div class="h-full rounded-full bg-[var(--accent)] transition-all" :style="{ width: `${progressPercent(item)}%` }" />
            </div>
            <div class="mt-2 flex items-center justify-between gap-2 text-[11px] text-[var(--text-tertiary)]">
              <span class="truncate">{{ formatTime(item.lastReadAt) }}</span>
              <span class="shrink-0 text-[var(--accent)] transition-colors group-hover:text-[var(--accent-hover)]">继续阅读 →</span>
            </div>
          </div>
        </button>
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from "vue";
import { useQuery } from "@tanstack/vue-query";
import { RouterLink, useRouter } from "vue-router";
import { getArchiveThumbnail, listReadingHistory } from "@/utils/api";
import { useTitleDisplayStore } from "@/stores/titleDisplay";
import type { ReadingHistoryItem, ReadingHistoryStatus } from "@/types/api";

const router = useRouter();
const titleDisplayStore = useTitleDisplayStore();
const selectedStatus = ref<ReadingHistoryStatus>("all");
const tabs: { value: ReadingHistoryStatus; label: string }[] = [
  { value: "all", label: "全部" },
  { value: "reading", label: "阅读中" },
  { value: "read", label: "已读" },
];
const coverUrls = ref<Record<string, string>>({});
let coverLoadId = 0;

const { data, error, isLoading, refetch } = useQuery({
  queryKey: computed(() => ["reading-history", selectedStatus.value]),
  queryFn: () => listReadingHistory(selectedStatus.value),
  retry: 1,
});

const historyItems = computed<ReadingHistoryItem[]>(() => data.value ?? []);

const displayTitle = (item: ReadingHistoryItem) => {
  const hasSubtitle = Boolean(item.subtitle?.trim() && item.subtitleLanguage);
  return titleDisplayStore.displayTranslatedTitle && hasSubtitle
    ? item.subtitle!.trim()
    : item.title;
};

const displaySubtitle = (item: ReadingHistoryItem) => {
  const hasSubtitle = Boolean(item.subtitle?.trim() && item.subtitleLanguage);
  return titleDisplayStore.displayTranslatedTitle && hasSubtitle
    ? item.title
    : item.subtitle?.trim() ?? "";
};

const progressPercent = (item: ReadingHistoryItem) =>
  Math.round(Math.min(1, Math.max(0, item.progressPercentage)) * 100);

const formatTime = (value: string) =>
  new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(value));

const releaseCovers = () => {
  Object.values(coverUrls.value).forEach((url) => URL.revokeObjectURL(url));
  coverUrls.value = {};
};

const loadCovers = async (items: ReadingHistoryItem[]) => {
  const loadId = ++coverLoadId;
  releaseCovers();
  await Promise.all(items.map(async (item) => {
    try {
      const url = await getArchiveThumbnail(item.archiveId);
      if (loadId !== coverLoadId) {
        URL.revokeObjectURL(url);
        return;
      }
      coverUrls.value[item.archiveId] = url;
    } catch {
      // Keep the neutral placeholder when a cover cannot be loaded.
    }
  }));
};

const removeCover = (archiveId: string) => {
  const url = coverUrls.value[archiveId];
  if (url) URL.revokeObjectURL(url);
  delete coverUrls.value[archiveId];
};

const continueReading = (archiveId: string) => {
  router.push({ name: "reader", params: { id: archiveId } });
};

watch(historyItems, (items) => void loadCovers(items), { immediate: true });
onUnmounted(() => {
  coverLoadId++;
  releaseCovers();
});
</script>
