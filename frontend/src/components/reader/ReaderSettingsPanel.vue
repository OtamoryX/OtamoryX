<template>
  <BaseSidePanel
    :show="show"
    title="阅读设置"
    width="narrow"
    @close="$emit('close')"
  >
    <div class="space-y-6">
      <!-- 显示设置 -->
      <section class="space-y-4">
        <h3 class="text-base font-medium text-blue-400 flex items-center">
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
              d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
            />
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"
            />
          </svg>
          显示设置
        </h3>

        <!-- 图片显示模式 -->
        <div class="space-y-2">
          <label class="text-sm text-gray-300">图片显示模式</label>
          <div class="grid grid-cols-3 gap-2">
            <button
              v-for="mode in displayModes"
              :key="mode.value"
              :class="[
                'p-2 rounded-lg text-xs border transition-all',
                props.imageDisplayMode === mode.value
                  ? 'bg-blue-600 border-blue-400 text-white'
                  : 'bg-white/10 border-white/20 text-gray-300 hover:bg-white/20',
              ]"
              @click="$emit('set-display-mode', mode.value)"
            >
              {{ mode.label }}
            </button>
          </div>
        </div>

        <!-- 阅读模式 -->
        <div class="space-y-2">
          <label class="text-sm text-gray-300">阅读模式</label>
          <div class="grid grid-cols-2 gap-2">
            <button
              v-for="mode in readingModes"
              :key="mode.value"
              :class="[
                'p-3 rounded-lg text-sm border transition-all flex items-center justify-center',
                props.readingMode === mode.value
                  ? 'bg-blue-600 border-blue-400 text-white'
                  : 'bg-white/10 border-white/20 text-gray-300 hover:bg-white/20',
              ]"
              @click="$emit('set-reading-mode', mode.value)"
            >
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
                  :d="mode.icon"
                />
              </svg>
              {{ mode.label }}
            </button>
          </div>
        </div>

        <!-- 翻页方向 -->
        <div class="space-y-2">
          <label class="text-sm text-gray-300">翻页方向</label>
          <div class="grid grid-cols-2 gap-2">
            <button
              v-for="direction in pageDirections"
              :key="direction.value"
              :class="[
                'p-3 rounded-lg text-sm border transition-all flex items-center justify-center',
                props.pageDirection === direction.value
                  ? 'bg-purple-600 border-purple-400 text-white'
                  : 'bg-white/10 border-white/20 text-gray-300 hover:bg-white/20',
              ]"
              @click="$emit('switch-page-direction')"
            >
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
                  :d="direction.icon"
                />
              </svg>
              {{ direction.label }}
            </button>
          </div>
        </div>

        <!-- 动画和全屏设置 -->
        <div class="space-y-2">
          <label class="text-sm text-gray-300">显示选项</label>
          <div class="grid grid-cols-2 gap-2">
            <button
              :class="[
                'p-2 rounded-lg text-xs border transition-all',
                props.pageAnimationEnabled
                  ? 'bg-green-600 border-green-400 text-white'
                  : 'bg-white/10 border-white/20 text-gray-300 hover:bg-white/20',
              ]"
              @click="$emit('toggle-animation')"
            >
              {{ props.pageAnimationEnabled ? "动画已开启" : "动画已关闭" }}
            </button>
            <button
              :class="[
                'p-2 rounded-lg text-xs border transition-all',
                props.isFullscreen
                  ? 'bg-purple-600 border-purple-400 text-white'
                  : 'bg-white/10 border-white/20 text-gray-300 hover:bg-white/20',
              ]"
              @click="$emit('toggle-fullscreen')"
            >
              {{ props.isFullscreen ? "退出全屏" : "全屏阅读" }}
            </button>
          </div>
        </div>
      </section>

      <!-- 界面设置 -->
      <section class="space-y-4">
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
              d="M7 21a4 4 0 01-4-4V5a2 2 0 012-2h4a2 2 0 012 2v12a4 4 0 01-4 4zM21 5a2 2 0 00-2-2h-4a2 2 0 00-2 2v12a4 4 0 004 4h4a2 2 0 002-2V5z"
            />
          </svg>
          界面设置
        </h3>

        <!-- 界面选项 -->
        <div class="space-y-3">
          <label class="flex items-center cursor-pointer">
            <input
              :checked="props.autoHideUI"
              type="checkbox"
              class="mr-3 rounded cursor-pointer"
              @change="handleAutoHideChange"
            />
            <span class="text-sm">自动隐藏工具栏</span>
          </label>
          <label class="flex items-center cursor-pointer">
            <input
              :checked="props.showPageNumbers"
              type="checkbox"
              class="mr-3 rounded cursor-pointer"
              @change="$emit('toggle-page-numbers')"
            />
            <span class="text-sm">显示页码信息</span>
          </label>
        </div>
      </section>

      <!-- 快捷键说明 -->
      <section class="space-y-4">
        <h3 class="text-base font-medium text-yellow-400 flex items-center">
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
              d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
            />
          </svg>
          快捷键
        </h3>
        <div class="text-xs text-gray-400 space-y-1">
          <div
            v-for="shortcut in shortcuts"
            :key="shortcut.key"
            class="flex justify-between"
          >
            <span>{{ shortcut.key }}</span>
            <span>{{ shortcut.description }}</span>
          </div>
        </div>
      </section>
    </div>
  </BaseSidePanel>
</template>

<script setup lang="ts">
import BaseSidePanel from "@/components/base/BaseSidePanel.vue";

interface Props {
  show: boolean;
  imageDisplayMode: string;
  readingMode: string;
  pageDirection: string;
  pageAnimationEnabled: boolean;
  isFullscreen: boolean;
  autoHideUI: boolean;
  showPageNumbers: boolean;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  close: [];
  "set-display-mode": [mode: string];
  "set-reading-mode": [mode: string];
  "switch-page-direction": [];
  "toggle-animation": [];
  "toggle-fullscreen": [];
  "toggle-auto-hide": [];
  "toggle-page-numbers": [];
}>();

const displayModes = [
  { value: "fit", label: "适应屏幕" },
  { value: "fill", label: "填充屏幕" },
  { value: "original", label: "原始尺寸" },
];

const readingModes = [
  {
    value: "single",
    label: "单页模式",
    icon: "M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z",
  },
  {
    value: "double",
    label: "双页模式",
    icon: "M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.746 0 3.332.477 4.5 1.253v13C19.832 18.477 18.246 18 16.5 18c-1.746 0-3.332.477-4.5 1.253",
  },
];

const pageDirections = [
  {
    value: "ltr",
    label: "从左到右",
    icon: "M17 8l4 4m0 0l-4 4m4-4H3",
  },
  {
    value: "rtl",
    label: "从右到左",
    icon: "M7 16l-4-4m0 0l4-4m-4 4h18",
  },
];

const shortcuts = [
  { key: "← →", description: "翻页" },
  { key: "Space", description: "显示/隐藏详情" },
  { key: "V", description: "切换显示模式" },
  { key: "D", description: "切换阅读模式" },
  { key: "Esc", description: "返回书库" },
];

const handleAutoHideChange = () => {
  emit("toggle-auto-hide");
};
</script>

<style scoped>
/* 样式已在Tailwind中定义，无需额外样式 */
</style>
