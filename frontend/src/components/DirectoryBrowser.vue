<template>
  <div class="directory-browser">
    <!-- 模态框遮罩 -->
    <div v-if="isOpen" class="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50"
      @click="closeModal">
      <!-- 模态框内容 -->
      <div
        class="relative bg-white/10 backdrop-blur-xl border border-white/20 rounded-xl shadow-2xl w-full max-w-2xl max-h-[85vh] flex flex-col"
        @click.stop>
        <!-- 背景装饰 -->
        <div
          class="absolute inset-0 bg-linear-to-br from-white/5 via-transparent to-transparent pointer-events-none" />
        <div class="absolute top-0 left-0 w-32 h-32 bg-blue-500/10 rounded-full blur-xl" />
        <div class="absolute bottom-0 right-0 w-24 h-24 bg-purple-500/10 rounded-full blur-lg" />
        <!-- 标题栏 -->
        <div class="relative z-10 flex items-center justify-between p-4 border-b border-white/20">
          <h2 class="text-lg font-semibold text-white flex items-center">
            <svg class="w-5 h-5 mr-2 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2H5a2 2 0 00-2-2v0" />
            </svg>
            选择目录
          </h2>
          <button class="text-white/60 hover:text-white hover:bg-white/10 p-2 rounded-lg transition-all duration-200"
            @click="closeModal">
            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <!-- 当前路径显示 -->
        <div class="relative z-10 px-4 py-3 bg-white/5 border-b border-white/20">
          <div class="flex items-center space-x-2 text-sm text-white/80">
            <svg class="w-4 h-4 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2H5a2 2 0 00-2-2v0" />
            </svg>
            <span class="font-medium">当前路径:</span>
            <span
              class="font-mono bg-white/10 backdrop-blur-sm px-3 py-1 rounded-lg border border-white/20 text-white">{{
                currentPath || "/" }}</span>
          </div>
        </div>

        <!-- 目录列表 -->
        <div class="relative z-10 flex-1 overflow-y-auto p-4 min-h-0">
          <!-- 加载状态 -->
          <div v-if="loading" class="flex items-center justify-center py-8">
            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-400" />
            <span class="ml-2 text-white/80">加载中...</span>
          </div>

          <!-- 错误状态 -->
          <div v-else-if="error" class="text-center py-8">
            <div class="text-red-400 mb-2">
              <svg class="w-12 h-12 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 15.5c-.77.833.192 2.5 1.732 2.5z" />
              </svg>
            </div>
            <p class="text-red-400 font-medium">加载失败</p>
            <p class="text-white/60 text-sm mt-1">
              {{ error }}
            </p>
            <button
              class="mt-3 px-4 py-2 bg-blue-500/20 hover:bg-blue-500/30 backdrop-blur-sm border border-blue-400/30 text-blue-200 rounded-lg transition-all duration-200"
              @click="refreshDirectory">
              重试
            </button>
          </div>

          <!-- 目录列表 -->
          <div v-else class="space-y-1">
            <!-- 返回上级目录 -->
            <button v-if="parentPath"
              class="w-full flex items-center p-3 text-left hover:bg-white/10 rounded-lg transition-all duration-200 group backdrop-blur-sm border border-transparent hover:border-white/20"
              @click="navigateToParent">
              <svg class="w-5 h-5 text-white/60 group-hover:text-blue-400 mr-3" fill="none" stroke="currentColor"
                viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M11 19l-7-7 7-7m8 14l-7-7 7-7" />
              </svg>
              <span class="text-white group-hover:text-blue-200 font-medium">..</span>
              <span class="text-white/60 ml-2">(返回上级)</span>
            </button>

            <!-- 目录项 -->
            <div v-for="directory in directories" :key="directory.path" class="directory-item">
              <button :disabled="!directory.is_accessible"
                class="w-full flex items-center p-3 text-left rounded-lg transition-all duration-200 backdrop-blur-sm border"
                :class="{
                  'hover:bg-white/10 cursor-pointer border-transparent hover:border-white/20':
                    directory.is_accessible,
                  'opacity-50 cursor-not-allowed border-white/10':
                    !directory.is_accessible,
                  'bg-blue-500/20 border-blue-400/30':
                    selectedPath === directory.path,
                }" @click="navigateToDirectory(directory.path)">
                <svg class="w-5 h-5 mr-3" :class="{
                  'text-blue-400': directory.is_accessible,
                  'text-white/40': !directory.is_accessible,
                }" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                    d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2H5a2 2 0 00-2-2v0" />
                </svg>
                <span :class="{
                  'text-white': directory.is_accessible,
                  'text-white/40': !directory.is_accessible,
                }">
                  {{ directory.name }}
                </span>
                <span v-if="!directory.is_accessible" class="ml-auto text-xs text-white/40">
                  无权限
                </span>
              </button>
            </div>

            <!-- 空目录提示 -->
            <div v-if="directories.length === 0" class="text-center py-8 text-white/60">
              <svg class="w-12 h-12 mx-auto mb-2 text-white/40" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2H5a2 2 0 00-2-2v0" />
              </svg>
              <p>此目录中没有子目录</p>
            </div>
          </div>
        </div>

        <!-- 底部操作栏 -->
        <div
          class="relative z-10 flex items-center justify-between p-4 border-t border-white/20 bg-white/5 backdrop-blur-sm">
          <div class="flex items-center space-x-2 text-sm text-white/80">
            <span>已选择:</span>
            <span
              class="font-mono bg-white/10 backdrop-blur-sm px-3 py-1 rounded-lg border border-white/20 max-w-md truncate text-white">
              {{ selectedPath || "未选择" }}
            </span>
          </div>
          <div class="flex space-x-2">
            <button
              class="px-4 py-2 text-white/80 hover:text-white hover:bg-white/10 rounded-lg transition-all duration-200"
              @click="closeModal">
              取消
            </button>
            <button :disabled="!currentPath"
              class="px-4 py-2 bg-blue-500/20 hover:bg-blue-500/30 backdrop-blur-sm border border-blue-400/30 text-blue-200 rounded-lg disabled:opacity-50 disabled:cursor-not-allowed transition-all duration-200"
              @click="selectCurrentPath">
              选择当前目录
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import { getDirectories } from "@/utils/api";

interface DirectoryInfo {
  name: string;
  path: string;
  is_accessible: boolean;
}

interface DirectoryListResponse {
  current_path: string;
  parent_path: string | null;
  directories: DirectoryInfo[];
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
  } catch (err: any) {
    console.error("Failed to load directory:", err);
    error.value = err.response?.data?.message || err.message || "加载目录失败";
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
.directory-browser {
  /* 确保模态框在最顶层 */
  z-index: 10;
}

.directory-item:hover {
  transform: translateX(2px);
}

/* 滚动条样式 */
.overflow-y-auto::-webkit-scrollbar {
  width: 6px;
}

.overflow-y-auto::-webkit-scrollbar-track {
  background: rgba(255, 255, 255, 0.1);
  border-radius: 3px;
}

.overflow-y-auto::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.3);
  border-radius: 3px;
}

.overflow-y-auto::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.5);
}
</style>
