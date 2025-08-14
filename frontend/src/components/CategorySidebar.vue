<template>
  <!-- 移动端遮罩层 -->
  <Transition
    enter-active-class="transition-opacity duration-300 ease-out"
    enter-from-class="opacity-0"
    enter-to-class="opacity-100"
    leave-active-class="transition-opacity duration-200 ease-in"
    leave-from-class="opacity-100"
    leave-to-class="opacity-0"
  >
    <div
      v-if="!isCollapsed && isMobile"
      class="fixed inset-0 bg-black/50 backdrop-blur-sm z-40 md:hidden"
      @click="toggleCollapse"
    />
  </Transition>

  <div
    :class="[
      'glass-sidebar h-full overflow-y-auto transition-all duration-300',
      // 桌面端和移动端都保持相对定位，但在移动端展开时变为固定定位
      'relative',
      // 移动端展开时：固定定位覆盖层样式
      !isCollapsed && isMobile ? 'md:relative fixed left-0 top-0 z-50' : '',
      // 宽度控制
      isCollapsed ? 'w-16' : 'w-64',
    ]"
  >
    <!-- 玻璃形态背景层 -->
    <div
      class="absolute inset-0 bg-white/5 backdrop-blur-lg border-r border-white/20"
    />

    <!-- 分类标题 -->
    <div class="relative p-4 border-b border-white/20">
      <div v-if="!isCollapsed"
class="flex items-center justify-between">
        <h2 class="text-lg font-semibold text-white flex items-center">
          <svg
            class="w-5 h-5 mr-2 text-blue-400"
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
          分类
        </h2>
        <div class="flex items-center space-x-2">
          <button
            class="p-1 text-white/60 hover:text-blue-400 hover:bg-white/10 rounded transition-all"
            title="创建分类"
            @click="$emit('create-category')"
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
          <button
            class="p-1 text-white/60 hover:text-blue-400 hover:bg-white/10 rounded transition-all"
            title="折叠侧边栏"
            @click="toggleCollapse"
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
                d="M15 19l-7-7 7-7"
              />
            </svg>
          </button>
        </div>
      </div>
      <!-- 折叠状态下的按钮 -->
      <div v-else
class="flex flex-col items-center space-y-3">
        <button
          class="p-2 w-10 h-10 text-white/60 hover:text-blue-400 hover:bg-white/10 rounded-lg transition-all"
          title="创建分类"
          @click="$emit('create-category')"
        >
          <svg
            class="w-6 h-6"
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
        <button
          class="p-2 w-10 h-10 text-white/60 hover:text-blue-400 hover:bg-white/10 rounded-lg transition-all"
          title="展开侧边栏"
          @click="toggleCollapse"
        >
          <svg
            class="w-6 h-6"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M9 5l7 7-7 7"
            />
          </svg>
        </button>
      </div>
    </div>

    <!-- 全部漫画 -->
    <div class="relative p-2">
      <button
        :class="[
          'w-full text-left px-3 py-2 rounded-lg transition-all duration-200',
          !selectedCategoryId
            ? 'bg-blue-500/20 text-blue-200 border border-blue-400/30 shadow-lg'
            : 'text-white/80 hover:bg-white/10 hover:text-white',
        ]"
        :title="isCollapsed ? `全部漫画 (${totalArchives})` : ''"
        @click="selectCategory(null)"
      >
        <div v-if="!isCollapsed"
class="flex items-center justify-between">
          <div class="flex items-center space-x-2">
            <svg
              class="w-4 h-4 text-blue-400"
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
            <span class="font-medium">全部漫画</span>
          </div>
          <span class="text-xs bg-white/20 px-2 py-1 rounded-full">{{
            totalArchives
          }}</span>
        </div>
        <div v-else
class="flex justify-center">
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
              d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
            />
          </svg>
        </div>
      </button>
    </div>

    <!-- 加载状态 -->
    <div v-if="isLoading"
class="relative p-4 text-center">
      <div class="text-white/60 text-sm">加载中...</div>
    </div>

    <!-- 错误状态 -->
    <div v-else-if="error"
class="relative p-4">
      <div class="text-red-400 text-sm">加载失败</div>
    </div>

    <!-- 分类列表 -->
    <div v-else
class="relative p-2 space-y-1">
      <!-- 静态分类 -->
      <div v-if="staticCategories.length > 0">
        <div
          v-if="!isCollapsed"
          class="px-3 py-2 text-xs font-semibold text-white/50 uppercase tracking-wider"
        >
          静态分类
        </div>
        <div
          v-for="category in staticCategories"
          :key="category.id"
          class="space-y-1"
        >
          <button
            :class="[
              'w-full text-left px-3 py-2 rounded-lg transition-all duration-200 group',
              selectedCategoryId === category.id
                ? 'bg-blue-500/20 text-blue-200 border border-blue-400/30 shadow-lg'
                : 'text-white/80 hover:bg-white/10 hover:text-white',
            ]"
            :title="
              isCollapsed ? `${category.name} (${category.archiveCount})` : ''
            "
            @click="selectCategory(category.id)"
          >
            <div v-if="!isCollapsed">
              <div class="flex items-center justify-between">
                <div class="flex items-center space-x-2">
                  <svg
                    class="w-4 h-4 text-purple-400"
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
                  <span class="text-sm font-medium">{{ category.name }}</span>
                </div>
                <div class="flex items-center space-x-1">
                  <span class="text-xs bg-white/20 px-2 py-1 rounded-full">{{
                    category.archiveCount
                  }}</span>
                  <button
                    class="opacity-0 group-hover:opacity-100 p-1 text-white/60 hover:text-blue-400 hover:bg-white/10 rounded transition-all"
                    @click.stop="$emit('edit-category', category)"
                  >
                    <svg
                      class="w-3 h-3"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
                      />
                    </svg>
                  </button>
                </div>
              </div>
              <div
                v-if="category.description"
                class="text-xs text-white/50 mt-1 pl-6"
              >
                {{ category.description }}
              </div>
            </div>
            <div v-else
class="flex justify-center">
              <svg
                class="w-4 h-4 text-purple-400"
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
            </div>
          </button>
        </div>
      </div>

      <!-- 动态分类 -->
      <div v-if="dynamicCategories.length > 0"
class="mt-4">
        <div
          v-if="!isCollapsed"
          class="px-3 py-2 text-xs font-semibold text-white/50 uppercase tracking-wider"
        >
          动态分类
        </div>
        <div
          v-for="category in dynamicCategories"
          :key="category.id"
          class="space-y-1"
        >
          <button
            :class="[
              'w-full text-left px-3 py-2 rounded-lg transition-all duration-200 group',
              selectedCategoryId === category.id
                ? 'bg-green-500/20 text-green-200 border border-green-400/30 shadow-lg'
                : 'text-white/80 hover:bg-white/10 hover:text-white',
            ]"
            :title="isCollapsed ? category.name : ''"
            @click="selectCategory(category.id)"
          >
            <div v-if="!isCollapsed">
              <div class="flex items-center justify-between">
                <div class="flex items-center space-x-2">
                  <svg
                    class="w-4 h-4 text-green-400"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M13 10V3L4 14h7v7l9-11h-7z"
                    />
                  </svg>
                  <span class="text-sm font-medium">{{ category.name }}</span>
                </div>
                <button
                  class="opacity-0 group-hover:opacity-100 p-1 text-white/60 hover:text-green-400 hover:bg-white/10 rounded transition-all"
                  @click.stop="$emit('edit-category', category)"
                >
                  <svg
                    class="w-3 h-3"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
                    />
                  </svg>
                </button>
              </div>
              <div
                v-if="category.description"
                class="text-xs text-white/50 mt-1 pl-6"
              >
                {{ category.description }}
              </div>
            </div>
            <div v-else
class="flex justify-center">
              <svg
                class="w-4 h-4 text-green-400"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M13 10V3L4 14h7v7l9-11h-7z"
                />
              </svg>
            </div>
          </button>
        </div>
      </div>

      <!-- 空状态 -->
      <div
        v-if="staticCategories.length === 0 && dynamicCategories.length === 0"
        class="relative p-4 text-center"
      >
        <div
v-if="!isCollapsed" class="text-white/50 text-sm">暂无分类</div>
        <button
          v-if="!isCollapsed"
          class="mt-2 text-blue-400 hover:text-blue-300 text-sm transition-colors"
          @click="$emit('create-category')"
        >
          创建第一个分类
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from "vue";
import { useQuery } from "@tanstack/vue-query";
import { getCategories } from "@/utils/api";
import type { Category, DynamicCategory } from "@/types/api";

interface Props {
  selectedCategoryId?: string | null;
  totalArchives?: number;
  collapsed?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  totalArchives: 0,
  collapsed: false,
});

// 折叠状态管理 - 现在使用外部传入的状态，并支持本地切换
const isCollapsed = ref(props.collapsed);

// 移动端检测
const isMobile = ref(false);
const MOBILE_BREAKPOINT = 768; // md 断点

const checkIsMobile = () => {
  isMobile.value = window.innerWidth < MOBILE_BREAKPOINT;
};

const handleResize = () => {
  checkIsMobile();
};

// 获取分类数据
const {
  data: categories,
  isLoading,
  error,
} = useQuery({
  queryKey: ["categories"],
  queryFn: getCategories,
});

// 分离静态和动态分类
const staticCategories = computed(() => {
  return categories.value?.filter((cat) => cat.isStatic) || [];
});

const dynamicCategories = computed(() => {
  // 动态分类需要特殊处理，因为后端返回的数据结构可能不同
  return categories.value?.filter((cat) => !cat.isStatic) || [];
});

const emit = defineEmits<{
  "select-category": [categoryId: string | null];
  "create-category": [];
  "edit-category": [category: Category | DynamicCategory];
  "toggle-collapse": [collapsed: boolean];
}>();

const selectCategory = (categoryId: string | null) => {
  emit("select-category", categoryId);
};

const toggleCollapse = () => {
  isCollapsed.value = !isCollapsed.value;
  emit("toggle-collapse", isCollapsed.value);
};

// 监听外部传入的 collapsed 属性变化
watch(
  () => props.collapsed,
  (newCollapsed) => {
    isCollapsed.value = newCollapsed;
  },
);

// 生命周期钩子
onMounted(() => {
  checkIsMobile();
  window.addEventListener('resize', handleResize);
});

onUnmounted(() => {
  window.removeEventListener('resize', handleResize);
});
</script>

<style scoped>
.category-sidebar {
  height: 100%;
}

/* 玻璃形态侧边栏效果 */
.glass-sidebar {
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
}

.glass-sidebar > div:first-child {
  background: linear-gradient(
    135deg,
    rgba(255, 255, 255, 0.08) 0%,
    rgba(255, 255, 255, 0.04) 50%,
    rgba(255, 255, 255, 0.08) 100%
  );
  border-right: 1px solid rgba(255, 255, 255, 0.1);
}

/* 装饰性边缘光效 */
.glass-sidebar::before {
  content: "";
  position: absolute;
  top: 0;
  left: 0;
  bottom: 0;
  width: 2px;
  background: linear-gradient(
    to bottom,
    transparent,
    rgba(59, 130, 246, 0.3) 30%,
    rgba(147, 51, 234, 0.3) 70%,
    transparent
  );
  opacity: 0.6;
}

/* 按钮悬停效果增强 */
.category-sidebar button {
  position: relative;
  overflow: hidden;
}

.category-sidebar button::before {
  content: "";
  position: absolute;
  top: 0;
  left: -100%;
  width: 100%;
  height: 100%;
  background: linear-gradient(
    90deg,
    transparent,
    rgba(255, 255, 255, 0.1),
    transparent
  );
  transition: left 0.5s ease;
}

.category-sidebar button:hover::before {
  left: 100%;
}

/* 滚动条样式 */
.category-sidebar::-webkit-scrollbar {
  width: 6px;
}

.category-sidebar::-webkit-scrollbar-track {
  background: rgba(255, 255, 255, 0.1);
  border-radius: 3px;
}

.category-sidebar::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.3);
  border-radius: 3px;
}

.category-sidebar::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.4);
}

/* 折叠动画优化 */
.category-sidebar {
  transition: width 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

/* 响应式调整 */
@media (max-width: 768px) {
  .glass-sidebar {
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
  }
}

/* 分类标题的特殊效果 */
.category-sidebar h2 svg {
  filter: drop-shadow(0 0 10px rgba(59, 130, 246, 0.3));
}

/* 选中状态的特殊光效 */
.bg-blue-500\/20 {
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.1);
  position: relative;
}

.bg-blue-500\/20::after {
  content: "";
  position: absolute;
  inset: 0;
  background: linear-gradient(
    135deg,
    rgba(59, 130, 246, 0.1) 0%,
    transparent 50%
  );
  border-radius: inherit;
  pointer-events: none;
}

.bg-green-500\/20 {
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.1);
  position: relative;
}

.bg-green-500\/20::after {
  content: "";
  position: absolute;
  inset: 0;
  background: linear-gradient(
    135deg,
    rgba(34, 197, 94, 0.1) 0%,
    transparent 50%
  );
  border-radius: inherit;
  pointer-events: none;
}
</style>
