<template>
  <BaseModal
    :show="isOpen"
    title="选择目录"
    width="2xl"
    max-height="full"
    :z-index="9999"
    :content-padding="false"
    @close="closeModal"
  >
    <div class="relative flex min-h-[24rem] flex-1 flex-col">
      <div class="pointer-events-none absolute inset-0 bg-linear-to-br from-[var(--bg-tertiary)] via-transparent to-transparent opacity-60" />

      <div class="relative z-10 border-b border-[var(--border)] bg-[var(--bg-secondary)] px-5 py-4">
        <div class="flex items-center gap-2 text-sm text-[var(--text-primary)]">
          <svg class="h-4 w-4 text-[var(--accent)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2H5a2 2 0 00-2-2v0"
            />
          </svg>
          <span class="font-medium">当前路径:</span>
          <span class="max-w-[70vw] truncate rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-1 font-mono text-[var(--text-primary)]">
            {{ currentPath || "/" }}
          </span>
        </div>
      </div>

      <div class="directory-scroll relative z-10 flex-1 overflow-y-auto p-4 min-h-0">
        <div v-if="loading" class="flex items-center justify-center py-8">
          <div class="h-8 w-8 animate-spin rounded-full border-b-2 border-[var(--accent)]" />
          <span class="ml-2 text-[var(--text-primary)]">加载中...</span>
        </div>

        <div v-else-if="error" class="py-8 text-center">
          <div class="mb-2 text-red-400">
            <svg class="mx-auto h-12 w-12" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 15.5c-.77.833.192 2.5 1.732 2.5z"
              />
            </svg>
          </div>
          <p class="font-medium text-red-400">加载失败</p>
          <p class="mt-1 text-sm text-[var(--text-secondary)]">
            {{ error }}
          </p>
          <button
            class="mt-3 rounded-lg bg-[var(--accent)] px-4 py-2 text-white transition-all duration-200 hover:bg-[var(--accent-hover)]"
            @click="refreshDirectory"
          >
            重试
          </button>
        </div>

        <div v-else class="space-y-1">
          <button
            v-if="parentPath"
            class="group w-full rounded-lg border border-transparent p-3 text-left transition-all duration-200 hover:border-[var(--border)] hover:bg-[var(--bg-tertiary)]"
            @click="navigateToParent"
          >
            <div class="flex items-center">
              <svg class="mr-3 h-5 w-5 text-[var(--text-secondary)] group-hover:text-[var(--accent)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 19l-7-7 7-7m8 14l-7-7 7-7" />
              </svg>
              <span class="font-medium text-[var(--text-primary)] group-hover:text-[var(--accent)]">..</span>
              <span class="ml-2 text-[var(--text-secondary)]">(返回上级)</span>
            </div>
          </button>

          <div v-for="directory in directories" :key="directory.path" class="directory-item">
            <button
              :disabled="!directory.is_accessible"
              class="w-full rounded-lg border p-3 text-left transition-all duration-200"
              :class="{
                'cursor-pointer border-transparent hover:border-[var(--border)] hover:bg-[var(--bg-tertiary)]': directory.is_accessible,
                'cursor-not-allowed border-[var(--border)] opacity-50': !directory.is_accessible,
                'border-[var(--accent)]/30 bg-[var(--accent)]/20': selectedPath === directory.path,
              }"
              @click="navigateToDirectory(directory.path)"
            >
              <div class="flex items-center">
                <svg
                  class="mr-3 h-5 w-5"
                  :class="{
                    'text-[var(--accent)]': directory.is_accessible,
                    'text-[var(--text-tertiary)]': !directory.is_accessible,
                  }"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2H5a2 2 0 00-2-2v0"
                  />
                </svg>
                <span
                  :class="{
                    'text-[var(--text-primary)]': directory.is_accessible,
                    'text-[var(--text-tertiary)]': !directory.is_accessible,
                  }"
                >
                  {{ directory.name }}
                </span>
                <span v-if="!directory.is_accessible" class="ml-auto text-xs text-[var(--text-tertiary)]">
                  无权限
                </span>
              </div>
            </button>
          </div>

          <div v-if="directories.length === 0" class="py-8 text-center text-[var(--text-secondary)]">
            <svg class="mx-auto mb-2 h-12 w-12 text-[var(--text-tertiary)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2H5a2 2 0 00-2-2v0"
              />
            </svg>
            <p>此目录中没有子目录</p>
          </div>
        </div>
      </div>
    </div>

    <template #footer>
      <div class="flex items-center justify-between gap-4">
        <div class="flex min-w-0 items-center gap-2 text-sm text-[var(--text-primary)]">
          <span class="shrink-0">已选择:</span>
          <span class="max-w-[52vw] truncate rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-1 font-mono text-[var(--text-primary)]">
            {{ selectedPath || "未选择" }}
          </span>
        </div>
        <div class="flex shrink-0 space-x-2">
          <button
            class="rounded-lg px-4 py-2 text-[var(--text-primary)] transition-all duration-200 hover:bg-[var(--bg-tertiary)]"
            @click="closeModal"
          >
            取消
          </button>
          <button
            :disabled="!currentPath"
            class="rounded-lg bg-[var(--accent)] px-4 py-2 text-white transition-all duration-200 hover:bg-[var(--accent-hover)] disabled:cursor-not-allowed disabled:opacity-50"
            @click="selectCurrentPath"
          >
            选择当前目录
          </button>
        </div>
      </div>
    </template>
  </BaseModal>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import BaseModal from "@/components/base/BaseModal.vue";
import { getDirectories } from "@/utils/api";

interface DirectoryInfo {
  name: string;
  path: string;
  is_accessible: boolean;
}

interface Props {
  isOpen: boolean;
  initialPath?: string;
}

interface Emits {
  (e: "close"): void;
  (e: "select", path: string): void;
}

const props = withDefaults(defineProps<Props>(), {
  initialPath: "",
});

const emit = defineEmits<Emits>();

// 响应式数据
const loading = ref(false);
const error = ref<string | null>(null);
const currentPath = ref("");
const parentPath = ref<string | null>(null);
const directories = ref<DirectoryInfo[]>([]);
const selectedPath = ref("");

const getErrorMessage = (err: unknown): string => {
  if (typeof err !== "object" || err === null) return "加载目录失败";
  const maybeError = err as {
    response?: { data?: { message?: string } };
    message?: string;
  };
  return maybeError.response?.data?.message || maybeError.message || "加载目录失败";
};

// 监听模态框打开状态
watch(
  () => props.isOpen,
  (isOpen) => {
    if (isOpen) {
      // 只有绝对路径才传递给后端，相对路径或空路径使用默认行为
      const pathToLoad =
        props.initialPath && props.initialPath.startsWith("/")
          ? props.initialPath
          : "";
      loadDirectory(pathToLoad);
      selectedPath.value = props.initialPath;
    }
  },
);

// 加载目录
const loadDirectory = async (path: string = "") => {
  loading.value = true;
  error.value = null;

  try {
    const response = await getDirectories(path || undefined);

    currentPath.value = response.current_path;
    parentPath.value = response.parent_path;
    directories.value = response.directories;
  } catch (err: unknown) {
    console.error("Failed to load directory:", err);
    error.value = getErrorMessage(err);
  } finally {
    loading.value = false;
  }
};

// 导航到父目录
const navigateToParent = () => {
  if (parentPath.value) {
    loadDirectory(parentPath.value);
  }
};

// 导航到指定目录
const navigateToDirectory = (path: string) => {
  loadDirectory(path);
};

// 刷新当前目录
const refreshDirectory = () => {
  loadDirectory(currentPath.value);
};

// 选择当前路径
const selectCurrentPath = () => {
  if (currentPath.value) {
    emit("select", currentPath.value);
    closeModal();
  }
};

// 关闭模态框
const closeModal = () => {
  emit("close");
};
</script>

<style scoped>
.directory-item:hover {
  transform: translateX(2px);
}

/* 滚动条样式 */
.directory-scroll::-webkit-scrollbar {
  width: 6px;
}

.directory-scroll::-webkit-scrollbar-track {
  background: var(--bg-tertiary);
  border-radius: 3px;
}

.directory-scroll::-webkit-scrollbar-thumb {
  background: var(--border);
  border-radius: 3px;
}

.directory-scroll::-webkit-scrollbar-thumb:hover {
  background: var(--text-tertiary);
}
</style>
