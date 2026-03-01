<template>
  <div
    class="fixed bottom-0 left-0 right-0 z-40 md:hidden bg-[#1b1b2f] border-t border-[#2d2d44]"
    style="height: 48px"
  >
    <div
      class="h-full overflow-x-auto overflow-y-hidden flex items-center px-2 gap-1.5 scrollbar-hide"
    >
      <!-- 全部标签 -->
      <button
        :class="[
          'flex-shrink-0 px-3 h-8 rounded text-xs font-medium transition-colors duration-150',
          !selectedCategoryId
            ? 'bg-[#7b68ee] text-white'
            : 'bg-[#2d2d44] text-[#a0a0a0] hover:text-white hover:bg-[#3d3d5c]',
        ]"
        style="min-width: 56px"
        @click="selectCategory(null)"
      >
        全部
      </button>

      <!-- 分类标签 -->
      <button
        v-for="category in categories"
        :key="category.id"
        :class="[
          'flex-shrink-0 px-3 h-8 rounded text-xs font-medium transition-colors duration-150',
          selectedCategoryId === category.id
            ? 'bg-[#7b68ee] text-white'
            : 'bg-[#2d2d44] text-[#a0a0a0] hover:text-white hover:bg-[#3d3d5c]',
        ]"
        style="min-width: 56px"
        @click="selectCategory(category.id)"
      >
        {{ category.name }}
      </button>

      <!-- 创建按钮 -->
      <button
        class="flex-shrink-0 w-8 h-8 rounded bg-[#2d2d44] text-[#a0a0a0] hover:text-white hover:bg-[#3d3d5c] transition-colors flex items-center justify-center"
        @click="createCategory"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
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
