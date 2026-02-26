<template>
  <div
    class="archive-card relative overflow-hidden transition-all duration-300 cursor-pointer group bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg hover:shadow-lg"
    @click="handleClick"
    @contextmenu.prevent="handleContextMenu"
    @touchstart="handleTouchStart"
    @touchend="handleTouchEnd"
    @touchcancel="handleTouchCancel"
    @touchmove="handleTouchMove"
  >
    <div class="relative">
      <div
        class="aspect-[3/4] bg-gray-100 dark:bg-gray-900 relative rounded-t-lg overflow-hidden"
      >
        <!-- 加载状态 -->
        <div
          v-if="imageLoading"
          class="w-full h-full flex items-center justify-center text-gray-400 dark:text-gray-600"
        >
          <div
            class="animate-spin rounded-full h-8 w-8 border-2 border-gray-300 dark:border-gray-600 border-t-gray-600 dark:border-t-gray-300"
          />
        </div>
        <!-- 缩略图 -->
        <img
          v-else-if="coverImageUrl"
          :src="coverImageUrl"
          :alt="archive.title"
          class="w-full h-full object-cover"
          @error="handleImageError"
        />
        <!-- 默认图标 -->
        <div
          v-else
          class="w-full h-full flex items-center justify-center text-gray-400 dark:text-gray-600"
        >
          <svg
            class="w-12 h-12"
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
      </div>

      <div class="p-3">
        <h3
          class="font-medium text-gray-900 dark:text-gray-100 text-sm mb-2 line-clamp-2"
        >
          {{ archive.title }}
        </h3>

        <div
          class="flex items-center justify-between text-xs text-gray-600 dark:text-gray-400 mb-2"
        >
          <span v-if="archive.pageCount" class="flex items-center">
            <svg
              class="w-3 h-3 mr-1"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
              />
            </svg>
            {{ archive.pageCount }}页
          </span>
          <span v-if="archive.createdAt" class="flex items-center">
            <svg
              class="w-3 h-3 mr-1"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M8 7V3a1 1 0 011-1h6a1 1 0 011 1v4m-6 0h10a2 2 0 012 2v10a2 2 0 01-2 2H7a2 2 0 01-2-2V9a2 2 0 012-2z"
              />
            </svg>
            {{ formatDate(archive.createdAt) }}
          </span>
        </div>

        <!-- 阅读进度条 -->
        <div
          v-if="progressPercentage !== undefined && progressPercentage > 0"
          class="mt-3"
        >
          <div
            class="flex items-center justify-between text-xs text-gray-600 dark:text-gray-400 mb-1"
          >
            <span class="flex items-center">
              <svg
                class="w-3 h-3 mr-1"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
              进度
            </span>
            <span class="font-semibold">{{ (progressPercentage * 100).toFixed(1) }}%</span>
          </div>
          <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2 overflow-hidden">
            <div
              class="bg-blue-500 h-full rounded-full transition-all duration-500"
              :style="{ width: `${progressPercentage * 100}%` }"
            />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import type { Archive } from "@/types/api";
import { getArchiveThumbnail } from "@/utils/api";

interface Props {
  archive: Archive;
  progressPercentage?: number;
}

const props = defineProps<Props>();
const emit = defineEmits<{
  click: [];
  contextmenu: [event: MouseEvent, archive: Archive];
}>();

const coverImageUrl = ref<string | null>(null);
const imageLoading = ref(true);

// 长按相关状态
const longPressTimer = ref<NodeJS.Timeout | null>(null);
const touchStartTime = ref(0);
const touchMoved = ref(false);
const LONG_PRESS_DURATION = 500; // 500ms 长按时间

// 加载缩略图
const loadThumbnail = async () => {
  try {
    imageLoading.value = true;
    const thumbnailUrl = await getArchiveThumbnail(props.archive.id);
    coverImageUrl.value = thumbnailUrl;
  } catch (error) {
    console.error("Failed to load thumbnail:", error);
    coverImageUrl.value = null;
  } finally {
    imageLoading.value = false;
  }
};

onMounted(() => {
  loadThumbnail();
});

onUnmounted(() => {
  clearLongPressTimer();
});

const handleImageError = (event: Event) => {
  const img = event.target as HTMLImageElement;
  img.style.display = "none";
};

const formatDate = (dateString: string) => {
  const date = new Date(dateString);
  return date.toLocaleDateString("zh-CN", {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
};

const handleContextMenu = (event: MouseEvent) => {
  emit('contextmenu', event, props.archive);
};

// 处理点击事件（只有在非长按时才触发）
const handleClick = (event: Event) => {
  // 如果是长按后的点击，不触发普通点击事件
  if (touchStartTime.value > 0 && Date.now() - touchStartTime.value >= LONG_PRESS_DURATION) {
    return;
  }
  emit('click');
};

// 触摸开始
const handleTouchStart = (event: TouchEvent) => {
  if (event.touches.length === 1) {
    touchStartTime.value = Date.now();
    touchMoved.value = false;

    // 设置长按定时器
    longPressTimer.value = setTimeout(() => {
      if (!touchMoved.value) {
        // 触发长按（模拟右键菜单）
        const touch = event.touches[0];
        const syntheticEvent = new MouseEvent('contextmenu', {
          bubbles: true,
          cancelable: true,
          clientX: touch.clientX,
          clientY: touch.clientY,
          view: window
        });

        // 添加轻微的触觉反馈（如果支持）
        if ('vibrate' in navigator) {
          navigator.vibrate(50);
        }

        handleContextMenu(syntheticEvent);
      }
    }, LONG_PRESS_DURATION);
  }
};

// 触摸移动
const handleTouchMove = (event: TouchEvent) => {
  touchMoved.value = true;
  clearLongPressTimer();
};

// 触摸结束
const handleTouchEnd = (event: TouchEvent) => {
  clearLongPressTimer();

  // 延迟重置，确保点击事件能正确判断
  setTimeout(() => {
    touchStartTime.value = 0;
  }, 50);
};

// 触摸取消
const handleTouchCancel = (event: TouchEvent) => {
  clearLongPressTimer();
  touchStartTime.value = 0;
  touchMoved.value = false;
};

// 清除长按定时器
const clearLongPressTimer = () => {
  if (longPressTimer.value) {
    clearTimeout(longPressTimer.value);
    longPressTimer.value = null;
  }
};
</script>

<style scoped>
.line-clamp-2 {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
</style>
