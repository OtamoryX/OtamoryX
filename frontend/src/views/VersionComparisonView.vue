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
        class="relative flex min-h-0 flex-col border-2 bg-[var(--bg-primary)] transition-colors"
        :class="
          canManageVersions
            ? deleteIds.has(archive.id)
              ? 'border-red-500 bg-red-500/[0.035]'
              : 'border-emerald-500/70'
            : 'border-transparent'
        "
      >
        <div class="border-b border-[var(--border)] px-3 py-2">
          <div class="flex items-center justify-between gap-2">
            <h2 class="truncate text-xs font-medium" :title="archive.title">
              {{ archive.title }}
            </h2>
            <span
              v-if="canManageVersions"
              class="shrink-0 border px-1.5 py-0.5 text-[10px] font-medium"
              :class="
                deleteIds.has(archive.id)
                  ? 'border-red-500/70 bg-red-500/10 text-red-400'
                  : 'border-emerald-500/60 bg-emerald-500/10 text-emerald-400'
              "
              >{{ deleteIds.has(archive.id) ? "待删除" : "保留" }}</span
            >
          </div>
          <div class="mt-1 flex items-center justify-between gap-2">
            <div class="flex min-w-0 items-center gap-2">
              <p class="shrink-0 text-[10px] text-[var(--text-tertiary)]">
                {{ archive.pageCount }} 页 · {{ formatSize(archive.fileSize) }}
              </p>
              <span
                v-if="canManageVersions && recommendedArchiveId"
                class="shrink-0 px-1.5 py-0.5 text-[10px] font-medium"
                :class="
                  recommendedArchiveId === archive.id
                    ? 'bg-emerald-500/15 text-emerald-400'
                    : 'bg-amber-500/10 text-amber-400'
                "
                >{{
                  recommendedArchiveId === archive.id
                    ? "推荐保留"
                    : "推荐删除"
                }}</span
              >
            </div>
            <div
              class="inline-flex h-6 shrink-0 items-center border border-[var(--border)] text-[10px]"
            >
              <button
                class="flex h-full w-6 items-center justify-center text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] disabled:opacity-35"
                :disabled="displayPage(archive) <= 1"
                title="此页向前对齐"
                @click.stop="changeOffset(archive, -1)"
              >
                <MinusIcon class="h-3 w-3" /></button
              ><span
                class="min-w-16 border-x border-[var(--border)] px-1 text-center tabular-nums text-[var(--text-secondary)]"
                >第 {{ displayPage(archive) }} 页</span
              ><button
                class="flex h-full w-6 items-center justify-center text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] disabled:opacity-35"
                :disabled="displayPage(archive) >= archive.pageCount"
                title="此页向后对齐"
                @click.stop="changeOffset(archive, 1)"
              >
                <PlusIcon class="h-3 w-3" />
              </button>
            </div>
          </div>
        </div>
        <div
          class="relative flex min-h-64 flex-1 items-center justify-center bg-black/10 p-2 transition-colors"
          :class="
            canManageVersions
              ? deleteIds.has(archive.id)
                ? 'cursor-pointer bg-red-950/20 hover:bg-red-950/30'
                : 'cursor-pointer hover:bg-emerald-500/10'
              : ''
          "
          @click="toggleDelete(archive.id)"
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
          <div
            v-if="loadingPageIds.has(archive.id) && pageUrls[archive.id]"
            class="absolute inset-0 flex items-center justify-center bg-black/25 text-xs text-white/80"
          >
            正在加载第 {{ displayPage(archive) }} 页...
          </div>
        </div>
      </article>
    </section>

    <footer
      v-if="canManageVersions && archives.length > 1"
      class="flex flex-wrap items-center justify-between gap-2 border-t border-[var(--border)] bg-[var(--bg-secondary)] px-3 py-2 sm:px-5"
    >
      <p class="text-xs text-[var(--text-secondary)]">
        <template v-if="deleteIds.size">
          保留 {{ archives.length - deleteIds.size }} 本，删除
          {{ deleteIds.size }} 本 · 预计释放
          {{ formatSize(selectedReclaimableSize) }}
        </template>
        <template v-else>所有版本均保留</template>
      </p>
      <button
        v-if="deleteIds.size"
        class="inline-flex h-8 items-center gap-1.5 bg-red-500 px-3 text-xs text-white hover:bg-red-400 disabled:opacity-50"
        :disabled="
          isCleaning ||
          deleteIds.size === 0 ||
          deleteIds.size === archives.length
        "
        @click="showCleanupConfirmation = true"
      >
        <TrashIcon class="h-3.5 w-3.5" />{{
          isCleaning ? "删除中..." : "删除选中版本"
        }}
      </button>
      <button
        v-else
        class="inline-flex h-8 items-center gap-1.5 border border-emerald-500/70 px-3 text-xs text-emerald-400 hover:bg-emerald-500/10 disabled:opacity-50"
        :disabled="isKeepingAll"
        @click="keepAllComparedVersions"
      >
        <CheckCircleIcon class="h-3.5 w-3.5" />{{
          isKeepingAll ? "处理中..." : "全部保留"
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
  CheckCircleIcon,
  MinusIcon,
  PlusIcon,
  TrashIcon,
} from "@heroicons/vue/24/outline";
import { useAuthStore } from "@/stores/auth";
import ConfirmModal from "@/components/common/ConfirmModal.vue";
import type { Archive, VersionGroup } from "@/types/api";
import {
  cleanupVersions,
  getArchive,
  getArchivePage,
  getVersionGroups,
  keepAllVersions,
} from "@/utils/api";
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
const recommendedArchiveId = ref<string | null>(null);
const currentPage = ref(1);
const pageOffsets = ref<Record<string, number>>({});
const pageUrls = ref<Record<string, string>>({});
const pageErrors = ref<Record<string, boolean>>({});
const isLoadingArchives = ref(false);
const archivesRequestId = ref(0);
const pageRequestIds = ref<Record<string, number>>({});
const loadingPageIds = ref(new Set<string>());
const deleteIds = ref(new Set<string>());
const isCleaning = ref(false);
const isKeepingAll = ref(false);
const showCleanupConfirmation = ref(false);
const basePageCount = computed(() => archives.value[0]?.pageCount || 1);
const canManageVersions = computed(() =>
  Boolean(groupId.value && authStore.isAdmin),
);
const currentOffset = (archive: Archive) => pageOffsets.value[archive.id] || 0;
const displayPage = (archive: Archive) =>
  currentPage.value + currentOffset(archive);
const selectedReclaimableSize = computed(() =>
  archives.value
    .filter((archive) => deleteIds.value.has(archive.id))
    .reduce((total, archive) => total + archive.fileSize, 0),
);
const cleanupMessage = computed(
  () =>
    `将永久删除 ${deleteIds.value.size} 个选中版本，保留 ${archives.value.length - deleteIds.value.size} 个版本。标签、静态分类和阅读进度会迁移到一个保留版本。未打开比较的文件不会受到影响。`,
);
const clearPageUrls = () => {
  Object.values(pageUrls.value).forEach(URL.revokeObjectURL);
  pageUrls.value = {};
  pageErrors.value = {};
  loadingPageIds.value = new Set();
  pageRequestIds.value = Object.fromEntries(
    Object.entries(pageRequestIds.value).map(([id, requestId]) => [
      id,
      requestId + 1,
    ]),
  );
};
const loadPages = async (targetIds?: string[]) => {
  const targets = targetIds
    ? archives.value.filter((archive) => targetIds.includes(archive.id))
    : archives.value;
  const requestIds = { ...pageRequestIds.value };
  targets.forEach((archive) => {
    requestIds[archive.id] = (requestIds[archive.id] || 0) + 1;
  });
  pageRequestIds.value = requestIds;
  loadingPageIds.value = new Set([
    ...loadingPageIds.value,
    ...targets.map((archive) => archive.id),
  ]);
  await Promise.all(
    targets.map(async (archive) => {
      const requestId = requestIds[archive.id];
      try {
        const url = await getArchivePage(archive.id, displayPage(archive));
        if (pageRequestIds.value[archive.id] !== requestId) {
          URL.revokeObjectURL(url);
          return;
        }
        const previousUrl = pageUrls.value[archive.id];
        pageUrls.value = { ...pageUrls.value, [archive.id]: url };
        if (previousUrl) URL.revokeObjectURL(previousUrl);
        const errors = { ...pageErrors.value };
        delete errors[archive.id];
        pageErrors.value = errors;
      } catch {
        if (pageRequestIds.value[archive.id] !== requestId) return;
        const previousUrl = pageUrls.value[archive.id];
        if (previousUrl) URL.revokeObjectURL(previousUrl);
        const urls = { ...pageUrls.value };
        delete urls[archive.id];
        pageUrls.value = urls;
        pageErrors.value = { ...pageErrors.value, [archive.id]: true };
      } finally {
        if (pageRequestIds.value[archive.id] === requestId) {
          const loading = new Set(loadingPageIds.value);
          loading.delete(archive.id);
          loadingPageIds.value = loading;
        }
      }
    }),
  );
};
const loadArchives = async () => {
  const requestId = ++archivesRequestId.value;
  isLoadingArchives.value = true;
  clearPageUrls();
  currentPage.value = 1;
  pageOffsets.value = {};
  const ids = selectedIds.value;
  const entries = await Promise.all(
    ids.map(async (id) => getArchive(id).catch(() => null)),
  );
  if (
    requestId !== archivesRequestId.value ||
    ids.join(",") !== selectedIds.value.join(",")
  )
    return;
  archives.value = entries.filter(
    (archive): archive is Archive => archive !== null,
  );
  recommendedArchiveId.value = null;
  if (canManageVersions.value && groupId.value) {
    const requestedGroupId = groupId.value;
    const group = (
      await getVersionGroups().catch(() => [] as VersionGroup[])
    ).find((candidate) => candidate.id === groupId.value);
    if (
      requestId !== archivesRequestId.value ||
      requestedGroupId !== groupId.value
    )
      return;
    recommendedArchiveId.value = group?.recommendedArchiveId ?? null;
  }
  deleteIds.value = recommendedArchiveId.value
    ? new Set(
        archives.value
          .filter((archive) => archive.id !== recommendedArchiveId.value)
          .map((archive) => archive.id),
      )
    : new Set();
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
const changeOffset = (archive: Archive, delta: number) => {
  const offset = currentOffset(archive) + delta;
  const page = currentPage.value + offset;
  if (page < 1 || page > archive.pageCount) return;
  pageOffsets.value = { ...pageOffsets.value, [archive.id]: offset };
  void loadPages([archive.id]);
};
const toggleDelete = (archiveId: string) => {
  if (!canManageVersions.value) return;
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
const keepAllComparedVersions = async () => {
  if (!groupId.value || isKeepingAll.value) return;
  isKeepingAll.value = true;
  try {
    await keepAllVersions(groupId.value);
    router.replace("/library");
  } finally {
    isKeepingAll.value = false;
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
