<template>
  <BaseSidePanel :show="show" title="详细信息" @close="$emit('close')">
    <!-- 基本信息 -->
    <div class="space-y-6">
      <section class="space-y-4">
        <h3 class="text-base font-medium text-blue-400 flex items-center">
          <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          基本信息
        </h3>
        <div class="space-y-3">
          <div>
            <p class="text-sm text-gray-400 mb-1">书名</p>
            <p class="text-white break-words">
              {{ archiveInfo?.title || "加载中..." }}
            </p>
          </div>
          <div class="grid grid-cols-2 gap-3">
            <div>
              <p class="text-sm text-gray-400 mb-1">当前页</p>
              <p class="text-white">{{ currentPage }} / {{ totalPages }}</p>
            </div>
            <div>
              <p class="text-sm text-gray-400 mb-1">阅读进度</p>
              <p class="text-white">{{ progressPercentage }}%</p>
            </div>
            <div>
              <p class="text-sm text-gray-400 mb-1">总页数</p>
              <p class="text-white">{{ archiveInfo?.pageCount }} 页</p>
            </div>
            <div>
              <p class="text-sm text-gray-400 mb-1">文件大小</p>
              <p class="text-white">
                {{ formatFileSize(archiveInfo?.fileSize) }}
              </p>
            </div>
          </div>
          <div>
            <p class="text-sm text-gray-400 mb-1">文件路径</p>
            <p class="text-xs text-gray-300 break-all font-mono bg-white/5 rounded p-2">
              {{ archiveInfo?.path }}
            </p>
          </div>
        </div>
      </section>

      <!-- 标签管理 -->
      <section>
        <TagManager :tags="archiveInfo?.tags" @add-tag="handleAddTag" @remove-tag="handleRemoveTag" />
      </section>

      <!-- 快速阅读设置 -->
      <section class="space-y-4">
        <h3 class="text-base font-medium text-purple-400 flex items-center">
          <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
          </svg>
          阅读设置
        </h3>
        <div class="grid grid-cols-2 gap-3">
          <button class="p-3 bg-white/10 hover:bg-white/20 rounded-lg transition-colors text-left"
            @click="$emit('switch-display-mode')">
            <div class="text-sm text-gray-300 mb-1">显示模式</div>
            <div class="text-white font-medium">
              {{ displayModeLabel }}
            </div>
          </button>
          <button class="p-3 bg-white/10 hover:bg-white/20 rounded-lg transition-colors text-left"
            @click="$emit('switch-reading-mode')">
            <div class="text-sm text-gray-300 mb-1">阅读模式</div>
            <div class="text-white font-medium">
              {{ readingModeLabel }}
            </div>
          </button>
        </div>
      </section>

      <!-- 操作按钮 -->
      <section class="space-y-4">
        <h3 class="text-base font-medium text-red-400 flex items-center">
          <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
          </svg>
          操作
        </h3>
        <div class="space-y-2">
          <button
            class="w-full p-3 bg-gray-700 hover:bg-gray-600 rounded-lg transition-colors flex items-center justify-center"
            @click="$emit('go-back')">
            <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18" />
            </svg>
            返回书库
          </button>
          <button
            class="w-full p-3 bg-red-600 hover:bg-red-700 rounded-lg transition-colors flex items-center justify-center"
            @click="showDeleteConfirm = true">
            <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
            </svg>
            删除漫画
          </button>
        </div>
      </section>
    </div>

    <!-- 删除确认对话框 -->
    <ConfirmModal :show="showDeleteConfirm" title="确认删除" message="确定要删除这部漫画吗？此操作不可撤销。" type="danger" confirm-text="删除"
      @close="showDeleteConfirm = false" @confirm="handleDeleteArchive" />
  </BaseSidePanel>
</template>

<script setup lang="ts">
import { ref, computed } from "vue";
import type { Archive } from "@/types/api";
import BaseSidePanel from "@/components/base/BaseSidePanel.vue";
import TagManager from "@/components/common/TagManager.vue";
import ConfirmModal from "@/components/common/ConfirmModal.vue";

interface Props {
  show: boolean;
  archiveInfo?: Archive;
  currentPage: number;
  totalPages: number;
  displayModeLabel: string;
  readingModeLabel: string;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  close: [];
  "add-tag": [tag: { namespace: string; name: string }];
  "remove-tag": [tagId: string];
  "switch-display-mode": [];
  "switch-reading-mode": [];
  "go-back": [];
  "delete-archive": [];
}>();

const showDeleteConfirm = ref(false);

const progressPercentage = computed(() => {
  if (props.totalPages === 0) return "0.0";
  return ((props.currentPage / props.totalPages) * 100).toFixed(1);
});

const handleAddTag = (tag: { namespace: string; name: string }) => {
  emit("add-tag", tag);
};

const handleRemoveTag = (tagId: string) => {
  emit("remove-tag", tagId);
};

const handleDeleteArchive = () => {
  showDeleteConfirm.value = false;
  emit("delete-archive");
};

// 工具方法
const formatFileSize = (bytes?: number): string => {
  if (!bytes) return "未知";

  const units = ["B", "KB", "MB", "GB"];
  let size = bytes;
  let unitIndex = 0;

  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex++;
  }

  return `${size.toFixed(1)} ${units[unitIndex]}`;
};
</script>

<style scoped>
/* 样式已在Tailwind中定义，无需额外样式 */
</style>
