<template>
  <div
    class="archive-card glass-card relative overflow-hidden transition-all duration-300 cursor-pointer group"
    @click="$emit('click')"
  >
    <!-- 玻璃形态背景 -->
    <div
      class="absolute inset-0 bg-white/10 backdrop-blur-md border border-white/20 rounded-lg"
    />

    <!-- 悬停光效 -->
    <div
      class="absolute inset-0 bg-linear-to-br from-white/5 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300 rounded-lg"
    />

    <div class="relative z-10">
      <div
        class="aspect-3/4 bg-black/20 relative rounded-t-lg overflow-hidden"
      >
        <!-- 加载状态 -->
        <div
          v-if="imageLoading"
          class="w-full h-full flex items-center justify-center text-white/70"
        >
          <div
            class="animate-spin rounded-full h-8 w-8 border-2 border-white/30 border-t-white"
          />
        </div>
        <!-- 缩略图 -->
        <img
          v-else-if="coverImageUrl"
          :src="coverImageUrl"
          :alt="archive.title"
          class="w-full h-full object-cover transition-transform duration-300 group-hover:scale-105"
          @error="handleImageError"
        />
        <!-- 默认图标 -->
        <div
          v-else
          class="w-full h-full flex items-center justify-center text-white/50"
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

        <!-- 玻璃形态装饰边框 -->
        <div
          class="absolute inset-0 border border-white/20 rounded-t-lg pointer-events-none"
        />
      </div>

      <div class="p-3">
        <h3
          class="font-medium text-white text-sm mb-2 line-clamp-2 group-hover:text-blue-200 transition-colors"
        >
          {{ archive.title }}
        </h3>

        <div
          class="flex items-center justify-between text-xs text-white/70 mb-2"
        >
          <span v-if="archive.pageCount"
class="flex items-center">
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
          <span v-if="archive.createdAt"
class="flex items-center">
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
            class="flex items-center justify-between text-xs text-white/80 mb-1"
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
          <div class="w-full bg-black/20 rounded-full h-2 overflow-hidden">
            <div class="relative h-full">
              <!-- 背景光效 -->
              <div
                class="absolute inset-0 bg-linear-to-r from-blue-500/20 to-purple-500/20 rounded-full"
              />
              <!-- 进度条 -->
              <div
                class="bg-linear-to-r from-blue-400 to-blue-500 h-full rounded-full transition-all duration-500 shadow-lg"
                :style="{ width: `${progressPercentage * 100}%` }"
              >
                <!-- 进度条光泽效果 -->
                <div
                  class="absolute inset-0 bg-linear-to-r from-transparent via-white/30 to-transparent rounded-full"
                />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import type { Archive } from "@/types/api";
import { getArchiveThumbnail } from "@/utils/api";

interface Props {
  archive: Archive;
  progressPercentage?: number;
}

const props = defineProps<Props>();
defineEmits<{
  click: [];
}>();

const coverImageUrl = ref<string | null>(null);
const imageLoading = ref(true);

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
</script>

<style scoped>
.line-clamp-2 {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

/* 玻璃卡片效果 */
.glass-card {
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  transform: translateZ(0); /* 启用硬件加速 */
}

.glass-card:hover {
  transform: translateY(-2px) scale(1.02);
  box-shadow:
    0 10px 30px rgba(0, 0, 0, 0.3),
    0 0 50px rgba(59, 130, 246, 0.1);
}

.glass-card:active {
  transform: translateY(0) scale(0.98);
}

/* 背景毛玻璃层 */
.glass-card > div:first-child {
  background: linear-gradient(
    135deg,
    rgba(255, 255, 255, 0.1) 0%,
    rgba(255, 255, 255, 0.05) 50%,
    rgba(255, 255, 255, 0.1) 100%
  );
}

/* 装饰性光效 */
.glass-card::before {
  content: "";
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 1px;
  background: linear-gradient(
    90deg,
    transparent,
    rgba(255, 255, 255, 0.3),
    transparent
  );
  opacity: 0;
  transition: opacity 0.3s ease;
}

.glass-card:hover::before {
  opacity: 1;
}

/* 响应式调整 */
@media (max-width: 640px) {
  .glass-card {
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
  }

  .glass-card:hover {
    transform: translateY(-1px) scale(1.01);
  }
}

/* 图片容器增强 */
.glass-card .aspect-\[3\/4\] {
  background: linear-gradient(135deg, rgba(0, 0, 0, 0.1), rgba(0, 0, 0, 0.3));
}

/* 进度条光效动画 */
@keyframes progress-shine {
  0% {
    transform: translateX(-100%);
  }
  100% {
    transform: translateX(100%);
  }
}

.glass-card .bg-gradient-to-r.from-blue-400 {
  position: relative;
  overflow: hidden;
}

.glass-card .bg-gradient-to-r.from-blue-400::after {
  content: "";
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: linear-gradient(
    90deg,
    transparent,
    rgba(255, 255, 255, 0.4),
    transparent
  );
  transform: translateX(-100%);
  animation: progress-shine 2s ease-in-out infinite;
}

/* 悬停时的额外效果 */
.glass-card:hover .aspect-\[3\/4\] img {
  filter: brightness(1.1) saturate(1.1);
}
</style>
