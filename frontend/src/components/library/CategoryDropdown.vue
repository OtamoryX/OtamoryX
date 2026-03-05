<template>
  <div class="relative flex-shrink-0">
    <!-- 下拉按钮 -->
    <button
      :class="[
        'flex items-center justify-between gap-2 px-4 py-2 rounded-lg transition-all duration-200',
        'bg-[var(--bg-tertiary)] hover:bg-[var(--border)]',
        'border border-[var(--border)]',
        'min-w-[160px] lg:min-w-[200px]',
      ]"
      @click="toggleDropdown"
    >
      <div class="flex items-center gap-2">
        <svg
          class="w-4 h-4 text-[var(--text-secondary)]"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
          />
        </svg>
        <span class="text-sm font-medium text-[var(--text-primary)]">
          {{ selectedCategoryName }}
        </span>
      </div>
      <svg
        :class="[
          'w-4 h-4 text-[var(--text-secondary)] transition-transform duration-200',
          isOpen ? 'rotate-180' : '',
        ]"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M19 9l-7 7-7-7"
        />
      </svg>
    </button>

    <!-- 下拉面板 -->
    <Transition
      enter-active-class="transition-all duration-200 ease-out"
      enter-from-class="opacity-0 scale-95 -translate-y-2"
      enter-to-class="opacity-100 scale-100 translate-y-0"
      leave-active-class="transition-all duration-150 ease-in"
      leave-from-class="opacity-100 scale-100 translate-y-0"
      leave-to-class="opacity-0 scale-95 -translate-y-2"
    >
      <div
        v-if="isOpen"
        class="absolute top-full left-0 mt-2 w-80 bg-[var(--bg-card)] border border-[var(--border)] rounded-lg shadow-xl z-50 overflow-hidden"
      >
        <!-- 标题栏 -->
        <div
          class="flex items-center justify-between px-4 py-3 border-b border-[var(--border)]"
        >
          <div>
            <h3 class="text-sm font-semibold text-[var(--text-primary)]">
              选择分类
            </h3>
            <p class="text-xs text-[var(--text-tertiary)] mt-0.5">
              点击右侧铅笔可编辑分类
            </p>
          </div>
          <button
            class="p-1 text-[var(--text-tertiary)] hover:text-[var(--text-primary)] rounded transition-colors"
            @click="closeDropdown"
          >
            <svg
              class="w-4 h-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        </div>

        <!-- 分类列表 -->
        <div class="max-h-96 overflow-y-auto">
          <!-- 全部漫画 -->
          <button
            :class="[
              'w-full flex items-center justify-between px-4 py-3 transition-colors',
              !selectedCategoryId
                ? 'bg-[var(--accent)]/10 text-[var(--accent)]'
                : 'hover:bg-[var(--bg-tertiary)] text-[var(--text-primary)]',
            ]"
            @click="selectCategory(null)"
          >
            <div class="flex items-center gap-3">
              <div
                :class="[
                  'w-4 h-4 rounded border-2 flex items-center justify-center',
                  !selectedCategoryId
                    ? 'bg-[var(--accent)] border-[var(--accent)]'
                    : 'border-[var(--border)]',
                ]"
              >
                <svg
                  v-if="!selectedCategoryId"
                  class="w-3 h-3 text-white"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="3"
                    d="M5 13l4 4L19 7"
                  />
                </svg>
              </div>
              <span class="text-sm font-medium">全部漫画</span>
            </div>
            <span
              class="text-xs px-2 py-1 rounded-full bg-[var(--bg-tertiary)] text-[var(--text-tertiary)]"
            >
              {{ totalArchives }}
            </span>
          </button>

          <!-- 加载状态 -->
          <div
            v-if="isLoading"
            class="px-4 py-8 text-center text-sm text-[var(--text-tertiary)]"
          >
            加载中...
          </div>

          <!-- 错误状态 -->
          <div
            v-else-if="error"
            class="px-4 py-8 text-center text-sm text-red-500"
          >
            加载失败
          </div>

          <!-- 分类列表 -->
          <template v-else>
            <div
              v-for="category in categories"
              :key="category.id"
              class="group flex items-center gap-2 px-2 py-1.5"
            >
              <button
                :class="[
                  'flex-1 flex items-center justify-between rounded-lg px-2.5 py-2.5 transition-all duration-200',
                  selectedCategoryId === category.id
                    ? 'bg-[var(--accent)]/12 text-[var(--accent)] ring-1 ring-[var(--accent)]/30'
                    : 'text-[var(--text-primary)] hover:bg-[var(--bg-tertiary)]',
                ]"
                @click="selectCategory(category.id)"
              >
                <div class="flex items-center gap-3">
                  <div
                    :class="[
                      'w-4 h-4 rounded border-2 flex items-center justify-center',
                      selectedCategoryId === category.id
                        ? 'bg-[var(--accent)] border-[var(--accent)]'
                        : 'border-[var(--border)]',
                    ]"
                  >
                    <svg
                      v-if="selectedCategoryId === category.id"
                      class="w-3 h-3 text-white"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="3"
                        d="M5 13l4 4L19 7"
                      />
                    </svg>
                  </div>
                  <div class="text-left">
                    <div class="flex items-center gap-2">
                      <div class="text-sm font-medium">{{ category.name }}</div>
                      <span
                        :class="[
                          'text-[10px] px-1.5 py-0.5 rounded-md border',
                          category.isStatic
                            ? 'bg-[#7b68ee]/20 text-[#c8c2ff] border-[#7b68ee]/35'
                            : 'bg-emerald-500/15 text-emerald-300 border-emerald-500/35',
                        ]"
                      >
                        {{ category.isStatic ? "静态" : "动态" }}
                      </span>
                    </div>
                    <div
                      v-if="category.description"
                      class="text-xs text-[var(--text-tertiary)] mt-0.5 line-clamp-1"
                    >
                      {{ category.description }}
                    </div>
                  </div>
                </div>
                <span
                  v-if="category.isStatic"
                  :class="[
                    'text-xs px-2 py-1 rounded-full transition-colors',
                    selectedCategoryId === category.id
                      ? 'bg-[var(--accent)]/20 text-[var(--accent)]'
                      : 'bg-[var(--bg-tertiary)] text-[var(--text-tertiary)]',
                  ]"
                >
                  {{ category.archiveCount }}
                </span>
              </button>
              <button
                class="h-9 w-9 flex-shrink-0 rounded-lg border border-[var(--border)] text-[var(--text-tertiary)] bg-[var(--bg-tertiary)] hover:text-[var(--accent)] hover:border-[var(--accent)]/50 hover:bg-[var(--accent)]/10 transition-all duration-200 opacity-70 group-hover:opacity-100"
                title="编辑分类"
                @click.stop="editCategory(category)"
              >
                <svg class="w-4 h-4 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                </svg>
              </button>
            </div>
          </template>

          <!-- 空状态 -->
          <div
            v-if="!isLoading && !error && categories?.length === 0"
            class="px-4 py-8 text-center text-sm text-[var(--text-tertiary)]"
          >
            暂无分类
          </div>
        </div>
      </div>
    </Transition>

    <!-- 遮罩层 -->
    <Transition
      enter-active-class="transition-opacity duration-200"
      enter-from-class="opacity-0"
      enter-to-class="opacity-100"
      leave-active-class="transition-opacity duration-150"
      leave-from-class="opacity-100"
      leave-to-class="opacity-0"
    >
      <div
        v-if="isOpen"
        class="fixed inset-0 z-40"
        @click="closeDropdown"
      />
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { useQuery } from "@tanstack/vue-query";
import { getCategories } from "@/utils/api";
import type { Category } from "@/types/api";

interface Props {
  selectedCategoryId?: string | null;
  totalArchives?: number;
}

const props = withDefaults(defineProps<Props>(), {
  totalArchives: 0,
});

const emit = defineEmits<{
  "select-category": [categoryId: string | null];
  "edit-category": [category: Category];
}>();

const isOpen = ref(false);

// 获取分类数据
const {
  data: categories,
  isLoading,
  error,
} = useQuery({
  queryKey: ["categories"],
  queryFn: getCategories,
});

// 计算当前选中的分类名称
const selectedCategoryName = computed(() => {
  if (!props.selectedCategoryId) {
    return "全部漫画";
  }
  const category = categories.value?.find(
    (cat) => cat.id === props.selectedCategoryId,
  );
  return category?.name || "全部漫画";
});

const toggleDropdown = () => {
  isOpen.value = !isOpen.value;
};

const closeDropdown = () => {
  isOpen.value = false;
};

const selectCategory = (categoryId: string | null) => {
  emit("select-category", categoryId);
  closeDropdown();
};

const editCategory = (category: Category) => {
  emit("edit-category", category);
  closeDropdown();
};

// ESC 键关闭
const handleKeydown = (e: KeyboardEvent) => {
  if (e.key === "Escape" && isOpen.value) {
    closeDropdown();
  }
};

onMounted(() => {
  document.addEventListener("keydown", handleKeydown);
});

onUnmounted(() => {
  document.removeEventListener("keydown", handleKeydown);
});
</script>

<style scoped>
/* 滚动条样式 */
.max-h-96::-webkit-scrollbar {
  width: 6px;
}

.max-h-96::-webkit-scrollbar-track {
  background: transparent;
}

.max-h-96::-webkit-scrollbar-thumb {
  background: rgba(156, 163, 175, 0.3);
  border-radius: 3px;
}

.max-h-96::-webkit-scrollbar-thumb:hover {
  background: rgba(156, 163, 175, 0.5);
}

/* Dark mode 滚动条 */
.dark .max-h-96::-webkit-scrollbar-thumb {
  background: rgba(75, 85, 99, 0.5);
}

.dark .max-h-96::-webkit-scrollbar-thumb:hover {
  background: rgba(75, 85, 99, 0.7);
}
</style>
