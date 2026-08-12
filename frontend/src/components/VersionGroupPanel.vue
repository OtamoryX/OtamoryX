<template>
  <BaseSidePanel
    :show="show"
    width="wide"
    title="选择比较版本"
    @close="emit('close')"
  >
    <div v-if="group" class="space-y-4">
      <div class="border-b border-[var(--border)] pb-4">
        <h2
          class="break-words text-base font-semibold text-[var(--text-primary)]"
        >
          {{ group.displayTitle }}
        </h2>
        <p
          v-if="group.subtitle"
          class="mt-1 text-sm text-[var(--text-tertiary)]"
        >
          {{ group.subtitle }}
        </p>
        <p class="mt-2 text-xs text-[var(--text-secondary)]">
          {{ group.unitLabel }} · {{ group.members.length }} 个文件
        </p>
        <p class="mt-1 text-[11px] text-[var(--text-tertiary)]">
          选择 2 至 4 个版本进行内容对比。保留和删除会在下一步完成。
        </p>
      </div>

      <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <label
          v-for="(member, index) in group.members"
          :key="member.archive.id"
          class="relative block cursor-pointer overflow-hidden border transition-colors"
          :class="
            compareIds.includes(member.archive.id)
              ? 'border-[var(--accent)] bg-[var(--accent)]/10 ring-1 ring-[var(--accent)]'
              : 'border-[var(--border)] hover:border-[var(--text-tertiary)] hover:bg-[var(--bg-tertiary)]'
          "
        >
          <input
            :checked="compareIds.includes(member.archive.id)"
            type="checkbox"
            class="sr-only"
            :disabled="
              !compareIds.includes(member.archive.id) && compareIds.length >= 4
            "
            @change="toggleCompare(member.archive.id)"
          />
          <div class="flex gap-3 p-3">
            <div
              :ref="
                (element) =>
                  observeCover(member.archive.id, element as Element | null)
              "
              class="relative h-36 w-24 shrink-0 overflow-hidden rounded-sm bg-[var(--bg-tertiary)]"
            >
              <img
                v-if="memberCovers[member.archive.id]"
                :src="memberCovers[member.archive.id]"
                :alt="member.archive.title"
                class="h-full w-full object-cover"
              />
              <div
                v-else
                class="flex h-full w-full items-center justify-center text-xs text-[var(--text-tertiary)]"
              >
                无封面
              </div>
              <div
                v-if="compareIds.includes(member.archive.id)"
                class="absolute inset-x-0 bottom-0 bg-[var(--accent)] px-1.5 py-1 text-center text-[10px] font-medium text-white"
              >
                已加入比较
              </div>
            </div>
            <div class="min-w-0 flex-1">
              <div class="flex items-start justify-between gap-2">
                <div class="min-w-0">
                  <div class="text-xs text-[var(--text-tertiary)]">
                    版本 {{ index + 1 }}
                  </div>
                  <div
                    class="mt-0.5 break-words text-sm font-semibold text-[var(--text-primary)]"
                  >
                    {{ member.archive.title }}
                  </div>
                </div>
                <button
                  class="p-1 text-[var(--text-tertiary)] hover:text-[var(--text-primary)]"
                  title="阅读"
                  @click.prevent="emit('open-reader', member.archive.id)"
                >
                  <BookOpenIcon class="h-4 w-4" />
                </button>
              </div>
              <div class="mt-2 text-xs text-[var(--text-secondary)]">
                {{ member.archive.pageCount }} 页 ·
                {{ formatSize(member.archive.fileSize) }} ·
                {{ extension(member.archive.path) }}
              </div>
              <div
                v-if="member.isRecommended"
                class="mt-2 text-xs text-emerald-400"
              >
                推荐保留：{{ member.recommendationReasons.join("；") }}
              </div>
            </div>
          </div>
        </label>
      </div>

      <div
        class="flex flex-wrap items-center justify-between gap-2 border-t border-[var(--border)] pt-3"
      >
        <button
          v-if="canManage && group.status !== 'keep_all'"
          class="inline-flex h-9 items-center border border-[var(--border)] px-3 text-xs text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)]"
          @click="emit('keep-all', group.id)"
        >
          标记为无需处理
        </button>
        <button
          v-else-if="canManage"
          class="inline-flex h-9 items-center border border-[var(--border)] px-3 text-xs text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)]"
          @click="emit('restore', group.id)"
        >
          恢复待处理
        </button>
        <button
          class="inline-flex h-9 items-center gap-1.5 bg-[var(--accent)] px-3 text-xs text-white hover:opacity-90 disabled:opacity-50"
          :disabled="compareIds.length < 2"
          @click="openComparison"
        >
          比较选中的 {{ compareIds.length }} 个版本
        </button>
      </div>
    </div>
  </BaseSidePanel>
</template>

<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from "vue";
import { BookOpenIcon } from "@heroicons/vue/24/outline";
import BaseSidePanel from "@/components/base/BaseSidePanel.vue";
import type { VersionGroup } from "@/types/api";
import { getArchiveThumbnail } from "@/utils/api";

const props = defineProps<{
  show: boolean;
  group: VersionGroup | null;
  canManage: boolean;
}>();
const emit = defineEmits<{
  close: [];
  "open-reader": [archiveId: string];
  "open-comparison": [
    groupId: string,
    archiveIds: string[],
    memberIds: string[],
  ];
  "keep-all": [id: string];
  restore: [id: string];
}>();
const compareIds = ref<string[]>([]);
const memberCovers = ref<Record<string, string>>({});
const loadedCoverIds = new Set<string>();
const loadingCoverIds = new Set<string>();
let coverObserver: IntersectionObserver | null = null;
let coverGeneration = 0;
const clearCovers = () => {
  coverGeneration += 1;
  Object.values(memberCovers.value).forEach(URL.revokeObjectURL);
  memberCovers.value = {};
  loadedCoverIds.clear();
  loadingCoverIds.clear();
  coverObserver?.disconnect();
  coverObserver = null;
};
const loadCover = async (archiveId: string) => {
  if (loadedCoverIds.has(archiveId) || loadingCoverIds.has(archiveId)) return;
  const generation = coverGeneration;
  loadingCoverIds.add(archiveId);
  try {
    const url = await getArchiveThumbnail(archiveId);
    if (generation !== coverGeneration) {
      URL.revokeObjectURL(url);
      return;
    }
    memberCovers.value = { ...memberCovers.value, [archiveId]: url };
    loadedCoverIds.add(archiveId);
  } catch {
    if (generation === coverGeneration) loadedCoverIds.add(archiveId);
  } finally {
    if (generation === coverGeneration) loadingCoverIds.delete(archiveId);
  }
};
const observeCover = (archiveId: string, element: Element | null) => {
  if (!element || loadedCoverIds.has(archiveId)) return;
  if (!coverObserver && typeof IntersectionObserver !== "undefined")
    coverObserver = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) continue;
          const id = (entry.target as HTMLElement).dataset.archiveId;
          if (id) void loadCover(id);
          coverObserver?.unobserve(entry.target);
        }
      },
      { rootMargin: "160px" },
    );
  if (!coverObserver) {
    void loadCover(archiveId);
    return;
  }
  (element as HTMLElement).dataset.archiveId = archiveId;
  coverObserver.observe(element);
};
const toggleCompare = (archiveId: string) => {
  compareIds.value = compareIds.value.includes(archiveId)
    ? compareIds.value.filter((id) => id !== archiveId)
    : compareIds.value.length < 4
      ? [...compareIds.value, archiveId]
      : compareIds.value;
};
const openComparison = () => {
  if (props.group && compareIds.value.length > 1)
    emit(
      "open-comparison",
      props.group.id,
      compareIds.value,
      props.group.members.map((member) => member.archive.id),
    );
};
watch(
  () => props.group,
  (group) => {
    clearCovers();
    compareIds.value =
      group?.members.slice(0, 2).map((member) => member.archive.id) || [];
  },
  { immediate: true },
);
onBeforeUnmount(clearCovers);
const formatSize = (bytes: number) =>
  bytes >= 1024 * 1024
    ? `${(bytes / 1024 / 1024).toFixed(1)} MB`
    : `${Math.ceil(bytes / 1024)} KB`;
const extension = (path: string) =>
  path.split(".").pop()?.toUpperCase() || "文件";
</script>
