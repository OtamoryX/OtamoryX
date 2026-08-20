<template>
  <div
    class="archive-card bg-[var(--bg-card)] border border-[var(--border)] rounded cursor-pointer hover:border-[var(--accent)] transition-colors duration-150 overflow-hidden"
    @click="handleClick"
    @contextmenu.prevent="handleContextMenu"
    @touchstart="handleTouchStart"
    @touchend="handleTouchEnd"
    @touchcancel="handleTouchCancel"
    @touchmove="handleTouchMove"
  >
    <!-- 标题（顶部）-->
    <div class="h-14 sm:h-16 px-2 pt-2 pb-[0.3rem] flex flex-col gap-0.5">
      <h3
        class="text-xs font-semibold text-[var(--text-primary)] leading-4 min-h-8 max-h-8 overflow-hidden line-clamp-2 [overflow-wrap:anywhere]"
        :title="displayTitle"
      >
        {{ displayTitle }}
      </h3>
      <p
        v-if="displaySubtitle"
        class="hidden sm:block h-3 text-[10px] leading-3 text-[var(--text-tertiary)] truncate"
        :title="displaySubtitle"
      >
        {{ displaySubtitle }}
      </p>
      <div v-else class="hidden sm:block h-3" aria-hidden="true" />
    </div>

    <!-- 封面图片（中间）-->
    <div class="relative mx-1 aspect-[2/3] bg-[var(--bg-tertiary)] overflow-hidden rounded-sm">
      <div
        v-if="hasNewTag"
        class="new-triangle-indicator"
        title="New archive"
      >
        <span>NEW</span>
      </div>

      <!-- 加载状态 -->
      <div
        v-if="imageLoading"
        class="w-full h-full flex items-center justify-center text-[var(--text-tertiary)]"
      >
        <div class="animate-spin rounded-full h-5 w-5 border-2 border-[var(--border)] border-t-[var(--accent)]" />
      </div>
      <!-- 缩略图 -->
      <img
        v-else-if="coverImageUrl"
        :src="coverImageUrl"
        :alt="displayTitle"
        class="w-full h-full object-cover"
        @error="handleImageError"
      />
      <!-- 无图标 -->
      <div
        v-else
        class="w-full h-full flex items-center justify-center text-[var(--text-tertiary)]"
      >
        <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
            d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
        </svg>
      </div>

      <!-- 阅读进度条（在图片最底部）-->
      <div
        v-if="progressPercentage !== undefined && progressPercentage > 0"
        class="absolute bottom-0 left-0 right-0 h-0.5 bg-black/20"
      >
        <div
          class="h-full bg-[var(--accent)] transition-all duration-500"
          :style="{ width: `${progressPercentage * 100}%` }"
        />
      </div>
    </div>

    <!-- 底部：日期 + 标签 -->
    <div class="px-2 pt-1 pb-2">
      <!-- 日期行 -->
      <div class="text-[10px] text-[var(--text-tertiary)] mb-1">
        <span class="hidden sm:inline">{{ formatDate(archive.createdAt) }}</span>
        <span v-if="archive.pageCount" :class="{ 'sm:ml-1.5': true }"><span class="hidden sm:inline">· </span>{{ archive.pageCount }}P</span>
      </div>

      <!-- 标签行（最多显示3个）-->
      <div v-if="nonSystemTags.length > 0" class="hidden sm:flex flex-wrap gap-0.5">
        <span
          v-for="tag in displayTags"
          :key="tag.id"
          class="tag-chip text-[10px] px-1.5 py-0 rounded-sm leading-5 truncate max-w-[80px]"
          :class="getTagClass(tag.namespace)"
          :title="tagDisplayText(tag)"
        >
          {{ tagDisplayName(tag) }}
        </span>
        <span
          v-if="hiddenTagCount > 0"
          class="text-[10px] text-[var(--text-tertiary)] px-0.5 leading-5"
        >+{{ hiddenTagCount }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import type { Archive, Tag } from "@/types/api";
import { tagDisplayName, tagDisplayText } from "@/utils/tagDisplay";
import { getArchiveThumbnail } from "@/utils/api";
import { useTitleDisplayStore } from "@/stores/titleDisplay";
import { archiveDisplaySubtitle, archiveDisplayTitle } from "@/utils/archiveTitle";

interface Props {
  archive: Archive;
  progressPercentage?: number;
}

const props = defineProps<Props>();
const titleDisplayStore = useTitleDisplayStore();
const emit = defineEmits<{
  click: [];
  contextmenu: [event: MouseEvent, archive: Archive];
}>();

const coverImageUrl = ref<string | null>(null);
const imageLoading = ref(true);
const displayTitle = computed(() =>
  archiveDisplayTitle(props.archive, titleDisplayStore.displayTranslatedTitle),
);
const displaySubtitle = computed(() =>
  archiveDisplaySubtitle(props.archive, titleDisplayStore.displayTranslatedTitle),
);

// 长按相关状态
const longPressTimer = ref<ReturnType<typeof setTimeout> | null>(null);
const touchStartTime = ref(0);
const touchMoved = ref(false);
const LONG_PRESS_DURATION = 500;

const isSystemNewTag = (tag: Tag) =>
  tag.name?.toLowerCase() === "new" && tag.namespace?.toLowerCase() === "system";

const hasNewTag = computed(() => (props.archive.tags || []).some(isSystemNewTag));
const nonSystemTags = computed(() => (props.archive.tags || []).filter(tag => !isSystemNewTag(tag)));

// 只显示前3个普通标签，system:new 以角标显示
const displayTags = computed(() => nonSystemTags.value.slice(0, 3));
const hiddenTagCount = computed(() => Math.max(nonSystemTags.value.length - 3, 0));

// 根据 namespace 给标签着色（类似 exhentai）
const getTagClass = (namespace: string) => {
  const map: Record<string, string> = {
    artist: 'bg-purple-500/15 text-purple-400',
    author: 'bg-purple-500/15 text-purple-400',
    series: 'bg-blue-500/15 text-blue-400',
    parody: 'bg-blue-500/15 text-blue-400',
    character: 'bg-green-500/15 text-green-400',
    group: 'bg-orange-500/15 text-orange-400',
    language: 'bg-[var(--bg-tertiary)] text-[var(--text-secondary)]',
    female: 'bg-pink-500/15 text-pink-400',
    male: 'bg-cyan-500/15 text-cyan-400',
  };
  return map[namespace?.toLowerCase()] || 'bg-[var(--bg-tertiary)] text-[var(--text-secondary)]';
};

// 加载缩略图
const loadThumbnail = async () => {
  try {
    imageLoading.value = true;
    const thumbnailUrl = await getArchiveThumbnail(props.archive.id);
    coverImageUrl.value = thumbnailUrl;
  } catch (error) {
    coverImageUrl.value = null;
  } finally {
    imageLoading.value = false;
  }
};

onMounted(() => { loadThumbnail(); });
onUnmounted(() => { clearLongPressTimer(); });

const handleImageError = (event: Event) => {
  const img = event.target as HTMLImageElement;
  img.style.display = "none";
};

const formatDate = (dateString: string) => {
  const date = new Date(dateString);
  return date.toLocaleDateString("zh-CN", { year: "numeric", month: "2-digit", day: "2-digit" });
};

const handleContextMenu = (event: MouseEvent) => {
  emit('contextmenu', event, props.archive);
};

const handleClick = () => {
  if (touchStartTime.value > 0 && Date.now() - touchStartTime.value >= LONG_PRESS_DURATION) {
    return;
  }
  emit('click');
};

const handleTouchStart = (event: TouchEvent) => {
  if (event.touches.length === 1) {
    touchStartTime.value = Date.now();
    touchMoved.value = false;
    longPressTimer.value = setTimeout(() => {
      if (!touchMoved.value) {
        const touch = event.touches[0]!;
        const syntheticEvent = new MouseEvent('contextmenu', {
          bubbles: true, cancelable: true,
          clientX: touch.clientX, clientY: touch.clientY, view: window
        });
        if ('vibrate' in navigator) navigator.vibrate(50);
        handleContextMenu(syntheticEvent);
      }
    }, LONG_PRESS_DURATION);
  }
};

const handleTouchMove = () => {
  touchMoved.value = true;
  clearLongPressTimer();
};

const handleTouchEnd = () => {
  clearLongPressTimer();
  setTimeout(() => { touchStartTime.value = 0; }, 50);
};

const handleTouchCancel = () => {
  clearLongPressTimer();
  touchStartTime.value = 0;
  touchMoved.value = false;
};

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
}
.tag-chip {
  display: inline-block;
}

.new-triangle-indicator {
  --triangle-size: 44px;
  --new-text-right: 9px;
  --new-text-bottom: 14px;
  --new-text-size: 8.5px;
  position: absolute;
  right: 0;
  bottom: 0;
  width: var(--triangle-size);
  height: var(--triangle-size);
  z-index: 10;
  pointer-events: none;
}

.new-triangle-indicator::before {
  content: "";
  position: absolute;
  right: 0;
  bottom: 0;
  width: 100%;
  height: 100%;
  background: var(--accent);
  clip-path: polygon(100% 0, 0 100%, 100% 100%);
  opacity: 0.96;
  box-shadow: inset 1px 1px 0 rgba(255, 255, 255, 0.2);
}

.new-triangle-indicator > span {
  position: absolute;
  right: var(--new-text-right);
  bottom: var(--new-text-bottom);
  z-index: 1;
  color: #fff;
  font-size: var(--new-text-size);
  line-height: 1;
  font-weight: 800;
  letter-spacing: 0.02em;
  white-space: nowrap;
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.35);
  transform: rotate(-45deg);
  transform-origin: bottom right;
}

@media (max-width: 768px) {
  .new-triangle-indicator {
    --triangle-size: 36px;
    --new-text-right: 7px;
    --new-text-bottom: 11px;
    --new-text-size: 7.2px;
  }
}

@media (max-width: 480px) {
  .new-triangle-indicator {
    --triangle-size: 32px;
    --new-text-right: 6px;
    --new-text-bottom: 9px;
    --new-text-size: 6.6px;
  }
}
</style>
