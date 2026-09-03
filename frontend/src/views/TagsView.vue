<template>
  <BasePageView>
    <div class="min-h-screen bg-[var(--bg-secondary)]">
      <div class="mx-auto w-full max-w-7xl px-4 pb-10 pt-5 sm:px-6 sm:pt-7 lg:px-8">
        <header class="flex items-start justify-between gap-4">
          <div class="flex min-w-0 items-center gap-3">
            <div
              class="flex h-10 w-10 shrink-0 items-center justify-center rounded-md bg-[var(--accent)]/15 text-[var(--accent)]"
              aria-hidden="true"
            >
              <TagIcon class="h-5 w-5" />
            </div>
            <div class="min-w-0">
              <h1 class="truncate text-xl font-semibold text-[var(--text-primary)] sm:text-2xl">
                标签
              </h1>
              <p class="mt-0.5 text-xs text-[var(--text-tertiary)] sm:text-sm">
                {{ kind === "theme" ? "系统主题" : "书库标签" }}
              </p>
            </div>
          </div>

          <RouterLink
            to="/library"
            class="inline-flex h-10 shrink-0 items-center gap-1.5 rounded-md px-2.5 text-sm text-[var(--text-secondary)] transition-colors hover:bg-[var(--bg-tertiary)] hover:text-[var(--text-primary)]"
          >
            <ArrowLeftIcon class="h-4 w-4" />
            <span class="hidden sm:inline">书库</span>
          </RouterLink>
        </header>

        <section class="mt-6 border-y border-[var(--border)] py-4">
          <div class="flex flex-col gap-3 lg:flex-row lg:items-center">
            <div
              class="grid h-10 w-full shrink-0 grid-cols-2 rounded-md border border-[var(--border)] bg-[var(--bg-primary)] p-0.5 sm:w-56"
              role="tablist"
              aria-label="标签类型"
            >
              <button
                type="button"
                role="tab"
                :aria-selected="kind === 'tag'"
                :class="tabClass(kind === 'tag')"
                @click="setKind('tag')"
              >
                全部标签
              </button>
              <button
                type="button"
                role="tab"
                :aria-selected="kind === 'theme'"
                :class="tabClass(kind === 'theme')"
                @click="setKind('theme')"
              >
                主题
              </button>
            </div>

            <div class="relative min-w-0 flex-1">
              <label for="tag-directory-search" class="sr-only">搜索标签</label>
              <MagnifyingGlassIcon
                class="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-[var(--text-tertiary)]"
              />
              <input
                id="tag-directory-search"
                v-model="searchInput"
                type="search"
                autocomplete="off"
                :placeholder="kind === 'theme' ? '搜索主题或别名' : '搜索标签或命名空间'"
                class="h-10 w-full rounded-md border border-[var(--border)] bg-[var(--bg-primary)] px-9 text-sm text-[var(--text-primary)] outline-none transition-colors placeholder:text-[var(--text-tertiary)] focus:border-[var(--accent)]"
              />
              <button
                v-if="searchInput"
                type="button"
                class="absolute right-2 top-1/2 flex h-7 w-7 -translate-y-1/2 items-center justify-center rounded text-[var(--text-tertiary)] transition-colors hover:bg-[var(--bg-tertiary)] hover:text-[var(--text-primary)]"
                aria-label="清除搜索"
                title="清除搜索"
                @click="clearSearch"
              >
                <XMarkIcon class="h-4 w-4" />
              </button>
            </div>

            <label class="min-w-0 lg:w-52">
              <span class="sr-only">命名空间</span>
              <select
                v-model="namespace"
                :disabled="kind === 'theme'"
                class="h-10 w-full rounded-md border border-[var(--border)] bg-[var(--bg-primary)] px-3 text-sm text-[var(--text-primary)] outline-none transition-colors focus:border-[var(--accent)] disabled:cursor-not-allowed disabled:opacity-50"
              >
                <option value="">全部命名空间</option>
                <option v-for="item in namespaces" :key="item" :value="item">
                  {{ tagNamespaceLabel(item) }}
                </option>
              </select>
            </label>
          </div>

          <div class="mt-3 flex items-center justify-between gap-3">
            <p class="text-xs text-[var(--text-secondary)]" aria-live="polite">
              {{ total }} {{ kind === "theme" ? "个主题" : "个标签" }}
            </p>
            <label class="flex shrink-0 items-center gap-2 text-xs text-[var(--text-secondary)]">
              <span>排序</span>
              <select
                v-model="sort"
                class="h-9 rounded-md border border-[var(--border)] bg-[var(--bg-primary)] px-2.5 text-xs text-[var(--text-primary)] outline-none focus:border-[var(--accent)]"
              >
                <option value="usage">使用最多</option>
                <option value="name">名称</option>
              </select>
            </label>
          </div>
        </section>

        <div
          v-if="kind === 'theme'"
          class="mt-4 flex items-start gap-2 border-l-2 border-[var(--accent)]/60 pl-3 text-xs leading-5 text-[var(--text-secondary)]"
        >
          <InformationCircleIcon class="mt-0.5 h-4 w-4 shrink-0 text-[var(--accent)]" />
          <span>主题由系统维护，仅显示已确认的归一化名称。</span>
        </div>

        <div
          v-if="directoryQuery.isLoading.value"
          class="mt-5 grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4"
          aria-label="加载中"
        >
          <div
            v-for="index in 8"
            :key="index"
            class="min-h-[112px] animate-pulse rounded-md border border-[var(--border)] bg-[var(--bg-card)]"
          />
        </div>

        <div
          v-else-if="directoryQuery.error.value"
          class="mt-5 flex flex-col items-center justify-center border border-[var(--border)] bg-[var(--bg-card)] px-4 py-16 text-center"
        >
          <p class="text-sm text-[var(--text-secondary)]">标签加载失败</p>
          <button
            type="button"
            class="mt-3 inline-flex h-10 items-center gap-1.5 rounded-md border border-[var(--border)] px-3 text-sm text-[var(--text-primary)] transition-colors hover:bg-[var(--bg-tertiary)]"
            @click="directoryQuery.refetch()"
          >
            <ArrowPathIcon class="h-4 w-4" />
            重试
          </button>
        </div>

        <div
          v-else-if="items.length === 0"
          class="mt-5 flex flex-col items-center justify-center border border-dashed border-[var(--border)] px-4 py-16 text-center"
        >
          <TagIcon class="h-8 w-8 text-[var(--text-tertiary)]" />
          <p class="mt-3 text-sm text-[var(--text-secondary)]">
            {{ kind === "theme" ? "暂无已确认主题" : "没有找到标签" }}
          </p>
          <button
            v-if="searchInput || namespace"
            type="button"
            class="mt-3 text-sm text-[var(--accent)] hover:underline"
            @click="resetFilters"
          >
            清除筛选
          </button>
        </div>

        <div
          v-else
          class="mt-5 grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4"
        >
          <button
            v-for="item in items"
            :key="item.id"
            type="button"
            class="group flex min-h-[112px] flex-col justify-between rounded-md border border-[var(--border)] bg-[var(--bg-card)] p-3 text-left transition-colors hover:border-[var(--accent)] hover:bg-[var(--bg-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]/60"
            :aria-label="`查看${tagDisplayName(item)}下的漫画`"
            @click="openItem(item)"
          >
            <span class="flex min-w-0 items-start gap-2.5">
              <span
                class="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-[var(--bg-tertiary)] text-[var(--accent)] transition-colors group-hover:bg-[var(--accent)]/15"
                aria-hidden="true"
              >
                <SparklesIcon v-if="kind === 'theme'" class="h-4 w-4" />
                <TagIcon v-else class="h-4 w-4" />
              </span>
              <span class="min-w-0">
                <span class="block truncate text-sm font-medium text-[var(--text-primary)]">
                  {{ tagDisplayName(item) }}
                </span>
                <span class="mt-1 block truncate text-xs text-[var(--text-tertiary)]">
                  {{ kind === "theme" ? "系统主题" : tagNamespaceLabel(item.namespace) }}
                </span>
              </span>
              <span class="ml-auto shrink-0 text-right">
                <span class="block text-sm font-semibold tabular-nums text-[var(--text-primary)]">
                  {{ item.archiveCount }}
                </span>
                <span class="block text-[10px] text-[var(--text-tertiary)]">部</span>
              </span>
            </span>

            <span v-if="kind === 'theme' && item.aliases.length" class="mt-3 block min-w-0">
              <span class="mr-1.5 text-[10px] text-[var(--text-tertiary)]">别名</span>
              <span
                v-for="alias in item.aliases.slice(0, 3)"
                :key="alias"
                class="mr-1 inline-block max-w-[38%] truncate align-bottom text-[10px] text-[var(--text-secondary)]"
                :title="alias"
              >
                {{ alias }}
              </span>
              <span v-if="item.aliases.length > 3" class="text-[10px] text-[var(--text-tertiary)]">
                +{{ item.aliases.length - 3 }}
              </span>
            </span>
          </button>
        </div>

        <nav
          v-if="totalPages > 1"
          class="mt-6 flex items-center justify-center gap-3"
          aria-label="标签分页"
        >
          <button
            type="button"
            class="flex h-10 w-10 items-center justify-center rounded-md border border-[var(--border)] text-[var(--text-secondary)] transition-colors hover:bg-[var(--bg-tertiary)] hover:text-[var(--text-primary)] disabled:cursor-not-allowed disabled:opacity-40"
            :disabled="page <= 1"
            aria-label="上一页"
            title="上一页"
            @click="setPage(page - 1)"
          >
            <ChevronLeftIcon class="h-4 w-4" />
          </button>
          <span class="min-w-24 text-center text-xs tabular-nums text-[var(--text-secondary)]">
            第 {{ page }} / {{ totalPages }} 页
          </span>
          <button
            type="button"
            class="flex h-10 w-10 items-center justify-center rounded-md border border-[var(--border)] text-[var(--text-secondary)] transition-colors hover:bg-[var(--bg-tertiary)] hover:text-[var(--text-primary)] disabled:cursor-not-allowed disabled:opacity-40"
            :disabled="page >= totalPages"
            aria-label="下一页"
            title="下一页"
            @click="setPage(page + 1)"
          >
            <ChevronRightIcon class="h-4 w-4" />
          </button>
        </nav>
      </div>
    </div>
  </BasePageView>
</template>

<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from "vue";
import { useQuery } from "@tanstack/vue-query";
import { RouterLink, useRoute, useRouter } from "vue-router";
import {
  ArrowLeftIcon,
  ArrowPathIcon,
  ChevronLeftIcon,
  ChevronRightIcon,
  InformationCircleIcon,
  MagnifyingGlassIcon,
  SparklesIcon,
  TagIcon,
  XMarkIcon,
} from "@heroicons/vue/24/outline";
import BasePageView from "@/components/layout/BasePageView.vue";
import { getTagDirectory } from "@/utils/api";
import { tagDisplayName, tagNamespaceLabel } from "@/utils/tagDisplay";
import type { TagDirectoryItem, TagDirectoryParams } from "@/types/api";

type DirectoryKind = TagDirectoryParams["kind"];
type DirectorySort = NonNullable<TagDirectoryParams["sort"]>;

const route = useRoute();
const router = useRouter();
const pageSize = 48;

const routeKind = typeof route.query.kind === "string" ? route.query.kind : "tag";
const routeQuery = typeof route.query.q === "string" ? route.query.q : "";
const kind = ref<DirectoryKind>(routeKind === "theme" ? "theme" : "tag");
const searchInput = ref(routeQuery);
const appliedQuery = ref(routeQuery.trim());
const namespace = ref("");
const sort = ref<DirectorySort>("usage");
const page = ref(1);
let searchTimer: ReturnType<typeof setTimeout> | null = null;

const directoryQuery = useQuery({
  queryKey: computed(() => [
    "tag-directory",
    kind.value,
    appliedQuery.value,
    namespace.value,
    sort.value,
    page.value,
  ]),
  queryFn: () =>
    getTagDirectory({
      kind: kind.value,
      query: appliedQuery.value || undefined,
      namespace: kind.value === "tag" ? namespace.value || undefined : undefined,
      sort: sort.value,
      pageNumb: page.value,
      pageSize,
    }),
  staleTime: 60_000,
  retry: 1,
});

const items = computed(() => directoryQuery.data.value?.data ?? []);
const namespaces = computed(() => directoryQuery.data.value?.namespaces ?? []);
const total = computed(() => directoryQuery.data.value?.total ?? 0);
const totalPages = computed(() => Math.max(1, Math.ceil(total.value / pageSize)));

watch([kind, namespace, sort], () => {
  page.value = 1;
});

watch(searchInput, (value) => {
  if (searchTimer) clearTimeout(searchTimer);
  searchTimer = setTimeout(() => {
    appliedQuery.value = value.trim();
    page.value = 1;
    searchTimer = null;
  }, 220);
});

watch(totalPages, (value) => {
  if (page.value > value) page.value = value;
});

onUnmounted(() => {
  if (searchTimer) clearTimeout(searchTimer);
});

const tabClass = (active: boolean) =>
  [
    "rounded px-3 text-xs font-medium transition-colors",
    active
      ? "bg-[var(--accent)] text-white"
      : "text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] hover:text-[var(--text-primary)]",
  ];

const setKind = (nextKind: DirectoryKind) => {
  if (kind.value === nextKind) return;
  kind.value = nextKind;
  namespace.value = "";
};

const setPage = (nextPage: number) => {
  page.value = Math.min(Math.max(nextPage, 1), totalPages.value);
};

const clearSearch = () => {
  if (searchTimer) clearTimeout(searchTimer);
  searchTimer = null;
  searchInput.value = "";
  appliedQuery.value = "";
  page.value = 1;
};

const resetFilters = () => {
  clearSearch();
  namespace.value = "";
};

const openItem = (item: TagDirectoryItem) => {
  if (kind.value === "theme") {
    void router.push({
      name: "library",
      query: {
        theme: item.id,
        themeName: tagDisplayName(item),
      },
    });
    return;
  }

  void router.push({
    name: "library",
    query: { tag: `${item.namespace}:${item.name}` },
  });
};
</script>
