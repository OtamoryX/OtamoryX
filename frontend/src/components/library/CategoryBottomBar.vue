<template>
  <div
    class="fixed bottom-0 left-0 right-0 z-40 md:hidden bg-white dark:bg-gray-900 border-t border-gray-200 dark:border-gray-700 shadow-lg"
    style="height: 56px"
  >
    <div
      class="h-full overflow-x-auto overflow-y-hidden flex items-center px-2 gap-2 scrollbar-hide snap-x snap-mandatory"
      style="scroll-behavior: smooth"
    >
      <!-- 全部标签 -->
      <button
        :class="[
          'flex-shrink-0 px-4 h-12 rounded-lg font-medium text-sm transition-all duration-200',
          'snap-start',
          !selectedCategoryId
            ? 'bg-blue-500 text-white shadow-md'
            : 'bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700',
        ]"
        style="min-width: 80px"
        @click="selectCategory(null)"
      >
        全部
      </button>

      <!-- 分类标签 -->
      <button
        v-for="category in categories"
        :key="category.id"
        :class="[
          'flex-shrink-0 px-4 h-12 rounded-lg font-medium text-sm transition-all duration-200',
          'snap-start',
          selectedCategoryId === category.id
            ? 'bg-blue-500 text-white shadow-md'
            : 'bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700',
        ]"
        style="min-width: 80px"
        @click="selectCategory(category.id)"
      >
        {{ category.name }}
      </button>

      <!-- 创建按钮 -->
      <button
        class="flex-shrink-0 w-12 h-12 rounded-lg bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700 transition-all duration-200 flex items-center justify-center snap-start"
        @click="createCategory"
      >
        <svg
          class="w-5 h-5"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M12 6v6m0 0v6m0-6h6m-6 0H6"
          />
        </svg>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useQuery } from "@tanstack/vue-query";
import { getCategories } from "@/utils/api";

interface Props {
  selectedCategoryId?: string | null;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  "select-category": [categoryId: string | null];
  "create-category": [];
}>();

// 获取分类数据
const { data: categories } = useQuery({
  queryKey: ["categories"],
  queryFn: getCategories,
});

const selectCategory = (categoryId: string | null) => {
  emit("select-category", categoryId);
};

const createCategory = () => {
  emit("create-category");
};
</script>

<style scoped>
/* 隐藏滚动条但保持可滚动 */
.scrollbar-hide {
  -ms-overflow-style: none; /* IE and Edge */
  scrollbar-width: none; /* Firefox */
}

.scrollbar-hide::-webkit-scrollbar {
  display: none; /* Chrome, Safari and Opera */
}

/* Snap 滚动优化 */
.snap-x {
  scroll-snap-type: x mandatory;
}

.snap-start {
  scroll-snap-align: start;
}
</style>
