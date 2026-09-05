<template>
  <BaseSidePanel :show="show" title="详细信息" @close="$emit('close')">
    <!-- 基本信息 -->
    <div class="space-y-6">
      <section class="space-y-4">
        <h3 class="text-base font-medium text-[var(--accent)] flex items-center">
          <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          基本信息
        </h3>
        <div class="space-y-3">
          <div>
            <p class="text-sm text-[var(--text-tertiary)] mb-1">原始标题</p>
            <p class="text-[var(--text-primary)] break-words">
              {{ archiveInfo?.title || "加载中..." }}
            </p>
          </div>
          <div v-if="archiveInfo" class="space-y-2">
            <div v-if="archiveInfo.subtitle">
              <p class="text-sm text-[var(--text-tertiary)] mb-1">
                翻译标题<span v-if="archiveInfo.subtitleLanguage">（{{ archiveInfo.subtitleLanguage }}）</span>
              </p>
              <p class="text-[var(--text-secondary)] break-words">
                {{ archiveInfo.subtitle }}
              </p>
            </div>
            <button
              class="inline-flex items-center gap-1.5 text-sm text-[var(--accent)] hover:opacity-80 disabled:cursor-not-allowed disabled:opacity-50"
              :disabled="translationRetrying"
              @click="$emit('retry-title-translation')"
            >
              <ArrowPathIcon class="h-4 w-4" />
              {{ translationRetrying ? "重新入队中..." : "重新翻译标题" }}
            </button>
            <p
              v-if="translationRetryMessage"
              class="text-xs text-[var(--text-tertiary)] break-words"
            >
              {{ translationRetryMessage }}
            </p>
          </div>
          <div class="grid grid-cols-2 gap-3">
            <div>
              <p class="text-sm text-[var(--text-tertiary)] mb-1">当前页</p>
              <p class="text-[var(--text-primary)]">{{ currentPage }} / {{ totalPages }}</p>
            </div>
            <div>
              <p class="text-sm text-[var(--text-tertiary)] mb-1">阅读进度</p>
              <p class="text-[var(--text-primary)]">{{ progressPercentage }}%</p>
            </div>
            <div>
              <p class="text-sm text-[var(--text-tertiary)] mb-1">总页数</p>
              <p class="text-[var(--text-primary)]">{{ archiveInfo?.pageCount }} 页</p>
            </div>
            <div>
              <p class="text-sm text-[var(--text-tertiary)] mb-1">文件大小</p>
              <p class="text-[var(--text-primary)]">
                {{ formatFileSize(archiveInfo?.fileSize) }}
              </p>
            </div>
          </div>
          <div>
            <p class="text-sm text-[var(--text-tertiary)] mb-1">文件路径</p>
            <p class="text-xs text-[var(--text-secondary)] break-all font-mono bg-[var(--bg-tertiary)] rounded p-2">
              {{ archiveInfo?.path }}
            </p>
          </div>
        </div>
      </section>

      <!-- 标签管理 -->
      <section class="space-y-4">
        <div class="flex items-center justify-between">
          <h3 class="text-base font-medium text-green-400 flex items-center">
            <svg
              class="w-4 h-4 mr-2"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z"
              />
            </svg>
            标签管理
          </h3>
          <button
            class="px-3 py-1 bg-green-600 hover:bg-green-700 text-white text-sm rounded-lg transition-colors flex items-center"
            @click="showTagModal = true"
          >
            <svg
              class="w-4 h-4 mr-1"
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
            添加标签
          </button>
        </div>

        <!-- 标签列表 -->
        <div class="space-y-2">
          <div v-if="archiveInfo?.tags?.length" class="flex flex-wrap gap-2">
            <TagChip
              v-for="tag in archiveInfo?.tags"
              :key="tag.id"
              :tag="tag"
              removable
              @remove="handleRemoveTag"
            />
          </div>
          <p v-else class="text-[var(--text-tertiary)] text-sm italic">暂无标签</p>
        </div>
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
          <button class="p-3 bg-[var(--bg-tertiary)] hover:bg-[var(--bg-tertiary)] rounded-lg transition-colors text-left"
            @click="$emit('switch-display-mode')">
            <div class="text-sm text-[var(--text-secondary)] mb-1">显示模式</div>
            <div class="text-[var(--text-primary)] font-medium">
              {{ displayModeLabel }}
            </div>
          </button>
          <button class="p-3 bg-[var(--bg-tertiary)] hover:bg-[var(--bg-tertiary)] rounded-lg transition-colors text-left"
            @click="$emit('switch-reading-mode')">
            <div class="text-sm text-[var(--text-secondary)] mb-1">阅读模式</div>
            <div class="text-[var(--text-primary)] font-medium">
              {{ readingModeLabel }}
            </div>
          </button>
        </div>
      </section>

      <!-- 插件 one-shot -->
      <section class="space-y-4">
        <h3 class="text-base font-medium text-amber-400 flex items-center">
          <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M19.428 15.428a4 4 0 00-5.656 0L6 23.2M6 12a4 4 0 015.656 0l.707.707M9 7h.01M15 7h.01M12 4h.01M21 12h.01M3 12h.01M12 20h.01" />
          </svg>
          插件 one-shot
        </h3>
        <div class="space-y-3">
          <div v-if="hasEhentaiPlugin" class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3 space-y-2">
            <div class="flex items-center justify-between gap-3">
              <p class="text-sm font-medium text-[var(--text-primary)]">E-Hentai 元数据</p>
              <button
                type="button"
                class="text-sm text-[var(--accent)] hover:opacity-80 disabled:cursor-not-allowed disabled:opacity-50"
                :disabled="pluginExecuting || ehentaiSearching"
                @click="handleSearchEhentai"
              >
                {{ ehentaiSearching ? "搜索中..." : "搜索候选" }}
              </button>
            </div>
            <p class="text-xs leading-5 text-[var(--text-secondary)]">搜索只显示候选，选择后才会写入标签。</p>
            <p v-if="ehentaiSearchError" class="text-xs text-red-400">{{ ehentaiSearchError }}</p>
            <div v-if="ehentaiCandidates.length" class="space-y-2">
              <button
                v-for="candidate in ehentaiCandidates"
                :key="candidate.sourceUrl"
                type="button"
                class="w-full rounded border border-[var(--border)] bg-[var(--bg-secondary)] p-2 text-left text-sm transition-colors hover:border-[var(--accent)] disabled:cursor-not-allowed disabled:opacity-50"
                :disabled="pluginExecuting"
                @click="emit('execute-plugin', { pluginId: 'ehentai-metadata', oneshotParam: candidate.sourceUrl })"
              >
                <span class="block break-words text-[var(--text-primary)]">{{ candidate.title }}</span>
                <span class="mt-1 block text-xs text-[var(--text-tertiary)]">应用此匹配</span>
              </button>
            </div>
          </div>
          <div v-if="hasNhentaiPlugin" class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3 space-y-2">
            <div class="flex items-center justify-between gap-3">
              <p class="text-sm font-medium text-[var(--text-primary)]">nHentai 元数据</p>
              <button type="button" class="text-sm text-[var(--accent)] hover:opacity-80 disabled:cursor-not-allowed disabled:opacity-50" :disabled="pluginExecuting || nhentaiSearching" @click="handleSearchNhentai">
                {{ nhentaiSearching ? "搜索中..." : "搜索候选" }}
              </button>
            </div>
            <p class="text-xs leading-5 text-[var(--text-secondary)]">搜索只显示候选，选择后才会写入标签。</p>
            <p v-if="nhentaiSearchError" class="text-xs text-red-400">{{ nhentaiSearchError }}</p>
            <div v-if="nhentaiCandidates.length" class="space-y-2">
              <button v-for="candidate in nhentaiCandidates" :key="candidate.sourceUrl" type="button" class="w-full rounded border border-[var(--border)] bg-[var(--bg-secondary)] p-2 text-left text-sm transition-colors hover:border-[var(--accent)] disabled:cursor-not-allowed disabled:opacity-50" :disabled="pluginExecuting" @click="emit('execute-plugin', { pluginId: 'nhentai-metadata', oneshotParam: candidate.sourceUrl })">
                <span class="block break-words text-[var(--text-primary)]">{{ candidate.title }}</span>
                <span class="mt-1 block text-xs text-[var(--text-tertiary)]">应用此匹配</span>
              </button>
            </div>
          </div>
          <div>
            <p class="text-sm text-[var(--text-tertiary)] mb-1">插件</p>
            <select
              v-model="selectedPluginId"
              class="w-full px-3 py-2 rounded-lg bg-[var(--bg-tertiary)] border border-[var(--border)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-amber-500/60"
              :disabled="pluginsLoading || pluginExecuting"
            >
              <option value="" disabled>请选择插件</option>
              <option
                v-for="plugin in pluginOptions"
                :key="plugin.id"
                :value="plugin.id"
              >
                {{ plugin.name }}
              </option>
            </select>
            <p v-if="pluginsLoading" class="text-xs text-[var(--text-tertiary)] mt-1">
              正在加载插件列表...
            </p>
            <p v-else-if="!pluginOptions.length" class="text-xs text-[var(--text-tertiary)] mt-1">
              暂无可执行插件
            </p>
          </div>

          <div>
            <p class="text-sm text-[var(--text-tertiary)] mb-1">参数（可选）</p>
            <input
              v-model.trim="oneshotParam"
              type="text"
              class="w-full px-3 py-2 rounded-lg bg-[var(--bg-tertiary)] border border-[var(--border)] text-[var(--text-primary)] text-sm placeholder:text-[var(--text-tertiary)] focus:outline-none focus:ring-2 focus:ring-amber-500/60"
              :disabled="pluginExecuting"
              placeholder="例如：URL、ID 或其它 one-shot 参数"
            >
          </div>

          <button
            class="w-full p-3 bg-amber-600 hover:bg-amber-700 disabled:opacity-60 disabled:cursor-not-allowed rounded-lg transition-colors text-white"
            :disabled="!canExecutePlugin"
            @click="handleExecutePlugin"
          >
            {{ pluginExecuting ? "执行中..." : "执行插件" }}
          </button>

          <div
            v-if="pluginExecutionSummary"
            class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3 space-y-2"
          >
            <div class="flex items-center justify-between">
              <p class="text-sm text-[var(--text-secondary)]">最近一次执行</p>
              <span
                class="text-xs px-2 py-0.5 rounded-full"
                :class="pluginExecutionSummary.status === 'success'
                  ? 'bg-green-500/15 text-green-400'
                  : 'bg-red-500/15 text-red-400'"
              >
                {{ pluginExecutionSummary.status === "success" ? "成功" : "失败" }}
              </span>
            </div>
            <p class="text-sm text-[var(--text-primary)] break-words">
              {{ pluginExecutionSummary.message }}
            </p>
          </div>
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
            class="w-full p-3 bg-red-600 hover:bg-red-700 disabled:cursor-not-allowed disabled:opacity-60 rounded-lg transition-colors flex items-center justify-center"
            :disabled="deleteLoading"
            @click="showDeleteConfirm = true">
            <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
            </svg>
            {{ deleteLoading ? "删除中..." : "删除漫画" }}
          </button>
          <p
            v-if="deleteError"
            class="rounded-lg border border-red-500/30 bg-red-500/10 p-3 text-sm text-red-400 break-words"
            role="alert"
            aria-live="polite"
          >
            {{ deleteError }}
          </p>
        </div>
      </section>
    </div>

    <!-- 删除确认对话框 -->
    <ConfirmModal :show="showDeleteConfirm" title="确认删除" message="确定要删除这部漫画吗？此操作不可撤销。" type="danger" confirm-text="删除"
      @close="showDeleteConfirm = false" @confirm="handleDeleteArchive" />

    <!-- 标签添加模态框 -->
    <TagModal
      v-if="showTagModal"
      :archive="archiveInfo"
      @close="showTagModal = false"
      @submit="handleTagModalSubmit"
    />
  </BaseSidePanel>
</template>

<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { ArrowPathIcon } from "@heroicons/vue/24/outline";
import type { Archive, EhentaiCandidate, NhentaiCandidate } from "@/types/api";
import BaseSidePanel from "@/components/base/BaseSidePanel.vue";
import TagModal from "@/components/common/TagModal.vue";
import TagChip from "@/components/base/TagChip.vue";
import ConfirmModal from "@/components/common/ConfirmModal.vue";

interface ReaderPluginOption {
  id: string;
  name: string;
}

interface ReaderPluginExecutionSummary {
  status: "success" | "failure";
  message: string;
}

interface Props {
  show: boolean;
  archiveInfo?: Archive;
  currentPage: number;
  totalPages: number;
  displayModeLabel: string;
  readingModeLabel: string;
  pluginOptions: ReaderPluginOption[];
  pluginsLoading: boolean;
  pluginExecuting: boolean;
  pluginExecutionSummary: ReaderPluginExecutionSummary | null;
  deleteError: string | null;
  deleteLoading: boolean;
  translationRetrying: boolean;
  translationRetryMessage: string | null;
  ehentaiCandidates: EhentaiCandidate[];
  ehentaiSearching: boolean;
  ehentaiSearchError: string | null;
  nhentaiCandidates: NhentaiCandidate[];
  nhentaiSearching: boolean;
  nhentaiSearchError: string | null;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  close: [];
  "add-tag": [tag: { namespace: string; name: string }];
  "remove-tag": [tagId: string];
  "switch-display-mode": [];
  "switch-reading-mode": [];
  "delete-archive": [];
  "execute-plugin": [payload: { pluginId: string; oneshotParam?: string }];
  "retry-title-translation": [];
  "search-ehentai": [];
  "search-nhentai": [];
}>();

const showDeleteConfirm = ref(false);
const showTagModal = ref(false);
const selectedPluginId = ref("");
const oneshotParam = ref("");

const hasEhentaiPlugin = computed(() => props.pluginOptions.some((plugin) => plugin.id === "ehentai-metadata"));
const hasNhentaiPlugin = computed(() => props.pluginOptions.some((plugin) => plugin.id === "nhentai-metadata"));

const progressPercentage = computed(() => {
  if (props.totalPages === 0) return "0.0";
  return ((props.currentPage / props.totalPages) * 100).toFixed(1);
});

const canExecutePlugin = computed(() => {
  return !!selectedPluginId.value && !props.pluginExecuting && !props.pluginsLoading;
});

watch(
  () => props.pluginOptions,
  (options) => {
    if (!options.length) {
      selectedPluginId.value = "";
      return;
    }

    const exists = options.some((plugin) => plugin.id === selectedPluginId.value);
    if (!exists) {
      selectedPluginId.value = options[0].id;
    }
  },
  { immediate: true },
);

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

const handleTagModalSubmit = (tagName: string, namespace: string) => {
  emit("add-tag", { name: tagName, namespace });
  showTagModal.value = false;
};

const handleExecutePlugin = () => {
  if (!canExecutePlugin.value) return;

  emit("execute-plugin", {
    pluginId: selectedPluginId.value,
    oneshotParam: oneshotParam.value.trim() || undefined,
  });
};

const handleSearchEhentai = () => emit("search-ehentai");
const handleSearchNhentai = () => emit("search-nhentai");

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
