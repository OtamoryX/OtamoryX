<template>
  <main
    class="fixed inset-0 z-50 flex flex-col bg-[var(--bg-primary)] text-[var(--text-primary)]"
  >
    <header
      class="flex min-h-14 items-center gap-3 border-b border-[var(--border)] px-3 sm:px-5"
    >
      <button
        class="p-2 text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] hover:text-[var(--text-primary)]"
        title="返回多版本"
        @click="goBack"
      >
        <ArrowLeftIcon class="h-5 w-5" />
      </button>
      <div class="min-w-0 flex-1">
        <h1 class="truncate text-sm font-medium">多版本对比</h1>
        <p class="text-[10px] text-[var(--text-tertiary)]">
          {{ archives.length }} 个版本 · 基准第 {{ currentPage }} 页
        </p>
      </div>
      <div class="flex shrink-0 items-center gap-1">
        <button
          class="p-2 text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] disabled:opacity-40"
          title="上一页"
          :disabled="currentPage <= 1"
          @click="changePage(-1)"
        >
          <ChevronLeftIcon class="h-5 w-5" /></button
        ><span
          class="min-w-16 text-center text-xs tabular-nums text-[var(--text-secondary)]"
          >{{ currentPage }} / {{ basePageCount }}</span
        ><button
          class="p-2 text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] disabled:opacity-40"
          title="下一页"
          :disabled="currentPage >= basePageCount"
          @click="changePage(1)"
        >
          <ChevronRightIcon class="h-5 w-5" />
        </button>
      </div>
    </header>

    <div
      v-if="isLoadingArchives"
      class="flex flex-1 items-center justify-center text-sm text-[var(--text-tertiary)]"
    >
      正在加载对比内容...
    </div>
    <div
      v-else-if="archives.length < 2"
      class="flex flex-1 items-center justify-center px-6 text-center text-sm text-[var(--text-tertiary)]"
    >
      请选择至少两个可访问的版本进行对比。
    </div>
    <section
      v-else
      class="grid flex-1 grid-cols-1 overflow-y-auto bg-[var(--border)] sm:grid-cols-2"
      :class="archives.length > 2 ? 'xl:grid-cols-3' : ''"
    >
      <article
        v-for="archive in archives"
        :key="archive.id"
        class="flex min-h-0 flex-col bg-[var(--bg-primary)]"
      >
        <div class="border-b border-[var(--border)] px-3 py-2">
          <div class="flex items-center justify-between gap-2">
            <h2 class="truncate text-xs font-medium" :title="archive.title">
              {{ archive.title }}
            </h2>
            <label
              v-if="groupId && authStore.isAdmin"
              class="flex shrink-0 items-center gap-1.5 text-[10px]"
              :class="
                deleteIds.has(archive.id) ? 'text-red-400' : 'text-emerald-400'
              "
              ><input
                :checked="deleteIds.has(archive.id)"
                type="checkbox"
                class="accent-red-500"
                @change="toggleDelete(archive.id)"
              />{{ deleteIds.has(archive.id) ? "删除" : "保留" }}</label
            >
          </div>
          <div class="mt-1 flex items-center justify-between gap-2">
            <p class="text-[10px] text-[var(--text-tertiary)]">
              {{ archive.pageCount }} 页 · {{ formatSize(archive.fileSize) }}
            </p>
            <div
              class="inline-flex h-6 shrink-0 items-center border border-[var(--border)] text-[10px]"
            >
              <button
                class="flex h-full w-6 items-center justify-center text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] disabled:opacity-35"
                :disabled="displayPage(archive) <= 1"
                title="此页向前对齐"
                @click="changeOffset(archive, -1)"
              >
                <MinusIcon class="h-3 w-3" /></button
              ><span
                class="min-w-16 border-x border-[var(--border)] px-1 text-center tabular-nums text-[var(--text-secondary)]"
                >第 {{ displayPage(archive) }} 页</span
              ><button
                class="flex h-full w-6 items-center justify-center text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] disabled:opacity-35"
                :disabled="displayPage(archive) >= archive.pageCount"
                title="此页向后对齐"
                @click="changeOffset(archive, 1)"
              >
                <PlusIcon class="h-3 w-3" />
              </button>
            </div>
          </div>
          <div
            v-if="archive.id !== baseArchiveId"
            class="mt-1 flex items-center justify-between gap-2 text-[10px]"
          >
            <span class="text-[var(--text-tertiary)]"
              >当前偏移 {{ currentOffset(archive) >= 0 ? "+" : ""
              }}{{ currentOffset(archive) }}</span
            >
            <div class="flex items-center gap-2">
              <button
                class="text-[var(--accent)] hover:underline"
                title="从当前基准页开始使用这个对齐关系"
                @click="saveAlignmentPoint(archive)"
              >
                记住此处对齐</button
              ><button
                v-if="hasAlignmentPoint(archive)"
                class="text-[var(--text-tertiary)] hover:text-[var(--text-primary)]"
                @click="resetAlignmentAtCurrentPage(archive)"
              >
                重置此处
              </button>
            </div>
          </div>
        </div>
        <div
          class="flex min-h-64 flex-1 items-center justify-center bg-black/10 p-2"
        >
          <img
            v-if="pageUrls[archive.id]"
            :src="pageUrls[archive.id]"
            :alt="`${archive.title} 第 ${displayPage(archive)} 页`"
            class="max-h-[calc(100vh-14rem)] max-w-full object-contain"
          />
          <p
            v-else-if="pageErrors[archive.id]"
            class="px-4 text-center text-xs text-red-400"
          >
            该版本没有第 {{ displayPage(archive) }} 页
          </p>
          <p v-else class="text-xs text-[var(--text-tertiary)]">正在加载...</p>
        </div>
      </article>
    </section>

    <footer
      v-if="groupId && authStore.isAdmin && archives.length > 1"
      class="flex flex-wrap items-center justify-between gap-2 border-t border-[var(--border)] bg-[var(--bg-secondary)] px-3 py-2 sm:px-5"
    >
      <p class="text-xs text-[var(--text-secondary)]">
        保留 {{ archives.length - deleteIds.size }} 本，删除
        {{ deleteIds.size }} 本<span v-if="deleteIds.size">
          · 预计释放 {{ formatSize(selectedReclaimableSize) }}</span
        >
      </p>
      <button
        class="inline-flex h-8 items-center gap-1.5 bg-red-500 px-3 text-xs text-white hover:bg-red-400 disabled:opacity-50"
        :disabled="
          isCleaning ||
          deleteIds.size === 0 ||
          deleteIds.size === archives.length
        "
        @click="showCleanupConfirmation = true"
      >
        <TrashIcon class="h-3.5 w-3.5" />{{
          isCleaning ? "删除中..." : "应用保留与删除"
        }}
      </button>
    </footer>
    <ConfirmModal
      :show="showCleanupConfirmation"
      title="确认处理多版本"
      :message="cleanupMessage"
      type="danger"
      confirm-text="删除选中版本"
      @close="showCleanupConfirmation = false"
      @confirm="cleanupComparedVersions"
    />
  </main>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import {
  ArrowLeftIcon,
  ChevronLeftIcon,
  ChevronRightIcon,
  MinusIcon,
  PlusIcon,
  TrashIcon,
} from "@heroicons/vue/24/outline";
import { useAuthStore } from "@/stores/auth";
import ConfirmModal from "@/components/common/ConfirmModal.vue";
import type { Archive } from "@/types/api";
import { cleanupVersions, getArchive, getArchivePage } from "@/utils/api";

interface AlignmentPoint {
  basePage: number;
  offset: number;
}
const route = useRoute();
const router = useRouter();
const authStore = useAuthStore();
const selectedIds = computed(() => {
  const value = route.query.ids;
  const raw = Array.isArray(value) ? value.join(",") : value || "";
  return [...new Set(raw.split(",").filter(Boolean))].slice(0, 4);
});
const groupId = computed(() =>
  typeof route.query.group === "string" ? route.query.group : "",
);
const archives = ref<Archive[]>([]);
const currentPage = ref(1);
const alignmentPoints = ref<Record<string, AlignmentPoint[]>>({});
const pageUrls = ref<Record<string, string>>({});
const pageErrors = ref<Record<string, boolean>>({});
const isLoadingArchives = ref(false);
const pageRequestId = ref(0);
const deleteIds = ref(new Set<string>());
const isCleaning = ref(false);
const showCleanupConfirmation = ref(false);
const baseArchiveId = computed(() => archives.value[0]?.id || "");
const basePageCount = computed(() => archives.value[0]?.pageCount || 1);
const alignmentStorageKey = computed(
  () =>
    `version-comparison-alignment:${groupId.value}:${selectedIds.value.join(",")}`,
);
const pointsFor = (archive: Archive) => alignmentPoints.value[archive.id] || [];
const currentOffset = (archive: Archive) => {
  if (archive.id === baseArchiveId.value) return 0;
  return (
    [...pointsFor(archive)]
      .filter((point) => point.basePage <= currentPage.value)
      .sort((left, right) => right.basePage - left.basePage)[0]?.offset || 0
  );
};
const displayPage = (archive: Archive) =>
  currentPage.value + currentOffset(archive);
const hasAlignmentPoint = (archive: Archive) =>
  pointsFor(archive).some((point) => point.basePage === currentPage.value);
const selectedReclaimableSize = computed(() =>
  archives.value
    .filter((archive) => deleteIds.value.has(archive.id))
    .reduce((total, archive) => total + archive.fileSize, 0),
);
const cleanupMessage = computed(
  () =>
    `将永久删除 ${deleteIds.value.size} 个选中版本，保留 ${archives.value.length - deleteIds.value.size} 个版本。标签、静态分类和阅读进度会迁移到一个保留版本。未打开比较的文件不会受到影响。`,
);
const persistAlignmentPoints = () => {
  try {
    sessionStorage.setItem(
      alignmentStorageKey.value,
      JSON.stringify(alignmentPoints.value),
    );
  } catch {}
};
const restoreAlignmentPoints = () => {
  try {
    const raw = sessionStorage.getItem(alignmentStorageKey.value);
    alignmentPoints.value = raw ? JSON.parse(raw) : {};
  } catch {
    alignmentPoints.value = {};
  }
};
const clearPageUrls = () => {
  Object.values(pageUrls.value).forEach(URL.revokeObjectURL);
  pageUrls.value = {};
};
const loadPages = async () => {
  clearPageUrls();
  pageErrors.value = {};
  const requestId = ++pageRequestId.value;
  const entries = await Promise.all(
    archives.value.map(async (archive) => {
      try {
        return [
          archive.id,
          await getArchivePage(archive.id, displayPage(archive)),
        ] as const;
      } catch {
        return [archive.id, null] as const;
      }
    }),
  );
  if (requestId !== pageRequestId.value) {
    entries.forEach(([, url]) => {
      if (url) URL.revokeObjectURL(url);
    });
    return;
  }
  const urls: Record<string, string> = {};
  const errors: Record<string, boolean> = {};
  entries.forEach(([id, url]) => {
    if (url) urls[id] = url;
    else errors[id] = true;
  });
  pageUrls.value = urls;
  pageErrors.value = errors;
};
const loadArchives = async () => {
  isLoadingArchives.value = true;
  clearPageUrls();
  currentPage.value = 1;
  const ids = selectedIds.value;
  const entries = await Promise.all(
    ids.map(async (id) => getArchive(id).catch(() => null)),
  );
  if (ids.join(",") !== selectedIds.value.join(",")) return;
  archives.value = entries.filter(
    (archive): archive is Archive => archive !== null,
  );
  restoreAlignmentPoints();
  deleteIds.value = new Set();
  isLoadingArchives.value = false;
  if (archives.value.length) await loadPages();
};
const changePage = (offset: number) => {
  const next = Math.min(
    basePageCount.value,
    Math.max(1, currentPage.value + offset),
  );
  if (next === currentPage.value) return;
  currentPage.value = next;
  void loadPages();
};
const setOffsetAtCurrentPage = (archive: Archive, offset: number) => {
  const points = pointsFor(archive).filter(
    (point) => point.basePage !== currentPage.value,
  );
  alignmentPoints.value = {
    ...alignmentPoints.value,
    [archive.id]: [...points, { basePage: currentPage.value, offset }].sort(
      (left, right) => left.basePage - right.basePage,
    ),
  };
  persistAlignmentPoints();
  void loadPages();
};
const changeOffset = (archive: Archive, delta: number) => {
  const offset = currentOffset(archive) + delta;
  const page = currentPage.value + offset;
  if (page < 1 || page > archive.pageCount) return;
  setOffsetAtCurrentPage(archive, offset);
};
const saveAlignmentPoint = (archive: Archive) =>
  setOffsetAtCurrentPage(archive, currentOffset(archive));
const resetAlignmentAtCurrentPage = (archive: Archive) => {
  const points = pointsFor(archive).filter(
    (point) => point.basePage !== currentPage.value,
  );
  alignmentPoints.value = { ...alignmentPoints.value, [archive.id]: points };
  persistAlignmentPoints();
  void loadPages();
};
const toggleDelete = (archiveId: string) => {
  const next = new Set(deleteIds.value);
  if (next.has(archiveId)) next.delete(archiveId);
  else next.add(archiveId);
  deleteIds.value = next;
};
const handleKeydown = (event: KeyboardEvent) => {
  const target = event.target as HTMLElement | null;
  if (target?.matches("input, select, textarea, button")) return;
  if (event.key === "ArrowLeft") {
    event.preventDefault();
    changePage(-1);
  } else if (event.key === "ArrowRight") {
    event.preventDefault();
    changePage(1);
  }
};
const cleanupComparedVersions = async () => {
  if (
    !groupId.value ||
    isCleaning.value ||
    !deleteIds.value.size ||
    deleteIds.value.size === archives.value.length
  )
    return;
  isCleaning.value = true;
  try {
    const keepArchiveId = archives.value.find(
      (archive) => !deleteIds.value.has(archive.id),
    )?.id;
    if (!keepArchiveId) return;
    await cleanupVersions(groupId.value, keepArchiveId, [...deleteIds.value]);
    showCleanupConfirmation.value = false;
    router.replace("/library");
  } finally {
    isCleaning.value = false;
  }
};
const goBack = () => {
  if (window.history.state?.back) router.back();
  else router.replace("/library");
};
const formatSize = (bytes: number) =>
  bytes >= 1024 * 1024
    ? `${(bytes / 1024 / 1024).toFixed(1)} MB`
    : `${Math.ceil(bytes / 1024)} KB`;
watch(
  selectedIds,
  () => {
    void loadArchives();
  },
  { immediate: true },
);
onMounted(() => document.addEventListener("keydown", handleKeydown));
onBeforeUnmount(() => document.removeEventListener("keydown", handleKeydown));
onBeforeUnmount(clearPageUrls);
</script>
