<template>
  <header
    class="fixed left-0 right-0 top-[calc(env(safe-area-inset-top,0px)+3.5rem)] z-30 border-b border-[#2d2d44] bg-[#1b1b2f] md:top-14"
  >
    <!-- 移动端书库工具栏 -->
    <div class="flex h-14 items-center justify-between px-3 md:hidden">
      <div
        class="flex items-center rounded border border-[#3d3d5c] bg-[#2d2d44] p-0.5"
      >
        <button
          class="min-h-9 rounded px-2.5 py-1 text-xs transition-colors"
          :class="
            viewMode !== 'collections'
              ? 'bg-[#4b4b70] text-white'
              : 'text-[#a0a0c0] hover:text-white'
          "
          @click="emit('set-view-mode', 'single')"
        >
          单本
        </button>
        <button
          class="min-h-9 rounded px-2.5 py-1 text-xs transition-colors"
          :class="
            viewMode === 'collections'
              ? 'bg-[#4b4b70] text-white'
              : 'text-[#a0a0c0] hover:text-white'
          "
          @click="emit('set-view-mode', 'collections')"
        >
          合集
        </button>
      </div>

      <button
        type="button"
        class="relative flex h-10 w-10 items-center justify-center rounded text-[#a0a0a0] transition-colors hover:bg-white/10 hover:text-white"
        aria-label="搜索"
        title="搜索"
        @click="emit('toggle-mobile-search')"
      >
        <svg
          class="h-5 w-5"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
          aria-hidden="true"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M21 21l-6-6m2-5a7 7 0 11-14 0a7 7 0 0114 0z"
          />
        </svg>
        <span
          v-if="activeFilterCount > 0 || searchQuery"
          class="absolute right-1 top-1 h-2.5 w-2.5 rounded-full border border-[#1b1b2f] bg-[#7b68ee]"
        />
      </button>
    </div>

    <!-- 桌面端书库工具栏 -->
    <div class="hidden h-14 items-center gap-4 px-4 md:flex">
      <!-- 视图切换 -->
      <div
        class="flex shrink-0 items-center rounded border border-[#3d3d5c] bg-[#2d2d44] p-0.5"
      >
        <button
          class="rounded px-2.5 py-1 text-xs transition-colors"
          :class="
            viewMode === 'single'
              ? 'bg-[#4b4b70] text-white'
              : 'text-[#a0a0c0] hover:text-white'
          "
          @click="emit('set-view-mode', 'single')"
        >
          单本
        </button>
        <button
          class="rounded px-2.5 py-1 text-xs transition-colors"
          :class="
            viewMode === 'collections'
              ? 'bg-[#4b4b70] text-white'
              : 'text-[#a0a0c0] hover:text-white'
          "
          @click="emit('set-view-mode', 'collections')"
        >
          合集
        </button>
        <button
          class="rounded px-2.5 py-1 text-xs transition-colors"
          :class="
            viewMode === 'versions'
              ? 'bg-[#4b4b70] text-white'
              : 'text-[#a0a0c0] hover:text-white'
          "
          @click="emit('set-view-mode', 'versions')"
        >
          多版本
        </button>
      </div>

      <!-- 搜索框 -->
      <div class="min-w-0 max-w-lg flex-1">
        <div class="flex items-center gap-2">
          <div class="relative min-w-0 flex-1">
            <input
              v-model="localSearchQuery"
              type="text"
              placeholder="搜索漫画、标签..."
              class="w-full rounded border border-[#3d3d5c] bg-[#2d2d44] px-3 py-1.5 pl-9 text-sm text-[#e0e0e0] placeholder-[#707090] transition-all focus:border-[#7b68ee] focus:bg-[#35355c] focus:outline-none"
              @input="handleSearch"
            />
            <svg
              class="absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-[#707090]"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
              aria-hidden="true"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M21 21l-6-6m2-5a7 7 0 11-14 0a7 7 0 0114 0z"
              />
            </svg>
          </div>

          <button
            type="button"
            :class="[
              'relative flex-shrink-0 rounded border p-1.5 transition-colors',
              showAdvancedSearch
                ? 'border-[#7b68ee] bg-[#7b68ee]/20 text-[#7b68ee]'
                : 'border-[#3d3d5c] bg-[#2d2d44] text-[#a0a0c0] hover:border-[#7b68ee] hover:text-white',
            ]"
            aria-label="高级筛选"
            title="高级筛选"
            :aria-expanded="showAdvancedSearch"
            @click="emit('toggle-advanced-search')"
          >
            <svg
              class="h-4 w-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
              aria-hidden="true"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2a1 1 0 01-.293.707L13 13.414V19a1 1 0 01-.553.894l-4 2A1 1 0 017 21v-7.586L3.293 6.707A1 1 0 013 6V4z"
              />
            </svg>
            <span
              v-if="activeFilterCount > 0"
              class="absolute -right-1.5 -top-1.5 flex h-4 w-4 items-center justify-center rounded-full bg-[#7b68ee] text-[9px] font-bold text-white"
            >
              {{ activeFilterCount }}
            </span>
          </button>
        </div>
      </div>

      <CategoryDropdown
        :selected-category-id="selectedCategoryId"
        :total-archives="totalArchives"
        @select-category="emit('select-category', $event)"
        @edit-category="emit('edit-category', $event)"
        @create-category="emit('create-category')"
      />
    </div>
  </header>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import CategoryDropdown from "@/components/library/CategoryDropdown.vue";
import type { Category } from "@/types/api";

interface Props {
  searchQuery?: string;
  showAdvancedSearch?: boolean;
  activeFilterCount?: number;
  selectedCategoryId?: string | null;
  totalArchives?: number;
  viewMode?: "single" | "collections" | "versions";
}

const props = withDefaults(defineProps<Props>(), {
  searchQuery: "",
  showAdvancedSearch: false,
  activeFilterCount: 0,
  selectedCategoryId: null,
  totalArchives: 0,
  viewMode: "single",
});

const emit = defineEmits<{
  "toggle-mobile-search": [];
  search: [query: string];
  "toggle-advanced-search": [];
  "select-category": [categoryId: string | null];
  "edit-category": [category: Category];
  "create-category": [];
  "set-view-mode": [mode: "single" | "collections" | "versions"];
}>();

const localSearchQuery = ref(props.searchQuery);

watch(
  () => props.searchQuery,
  (newValue) => {
    localSearchQuery.value = newValue;
  },
);

const handleSearch = () => {
  emit("search", localSearchQuery.value);
};
</script>
