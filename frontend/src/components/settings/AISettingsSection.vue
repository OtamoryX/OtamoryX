<template>
  <div class="space-y-6">
    <GlassCard size="md" radius="lg">
      <div class="mb-4 flex items-center justify-between">
        <h2 class="text-lg font-medium text-[var(--text-primary)]">AI 自动标签配置</h2>
        <GlassButton :disabled="aiLoading" variant="primary" size="sm" @click="emit('save')">
          {{ aiLoading ? "保存中..." : "保存 AI 设置" }}
        </GlassButton>
      </div>

      <div class="space-y-4">
        <label class="flex items-center gap-2 text-sm text-[var(--text-primary)]">
          <input v-model="aiSettings.enabled" type="checkbox" class="rounded" />
          启用 AI 自动标签
        </label>

        <div>
          <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]">自动应用阈值</label>
          <div class="flex items-center gap-3">
            <input v-model.number="aiSettings.autoApplyThreshold" type="range" min="0.1" max="1.0" step="0.1" class="flex-1" />
            <span class="w-12 text-sm text-[var(--text-primary)]">{{ (aiSettings.autoApplyThreshold * 100).toFixed(0) }}%</span>
          </div>
          <p class="mt-1 text-xs text-[var(--text-secondary)]">置信度达到阈值将自动应用标签。</p>
        </div>

        <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <div>
            <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]">处理调度</label>
            <select
              v-model="aiSettings.processingSchedule"
              class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
            >
              <option value="immediate">立即处理</option>
              <option value="batch">批量处理</option>
              <option value="off-peak">非高峰时段</option>
            </select>
          </div>

          <div>
            <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]">最大并发任务数</label>
            <input
              v-model.number="aiSettings.maxConcurrentTasks"
              type="number"
              min="1"
              max="10"
              class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
            />
          </div>
        </div>
      </div>
    </GlassCard>

    <GlassCard v-if="aiStatus" size="md" radius="lg">
      <h3 class="mb-3 text-base font-medium text-[var(--text-primary)]">运行状态</h3>
      <div class="grid grid-cols-2 gap-3 md:grid-cols-4">
        <div class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3 text-center">
          <div class="text-xl font-semibold text-[var(--accent)]">{{ aiStatus.queueSize }}</div>
          <div class="text-xs text-[var(--text-secondary)]">队列中</div>
        </div>
        <div class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3 text-center">
          <div class="text-xl font-semibold text-green-500">{{ aiStatus.processingCount }}</div>
          <div class="text-xs text-[var(--text-secondary)]">处理中</div>
        </div>
        <div class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3 text-center">
          <div class="text-xl font-semibold text-[var(--accent)]">{{ aiStatus.completedToday }}</div>
          <div class="text-xs text-[var(--text-secondary)]">今日完成</div>
        </div>
        <div class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3 text-center">
          <div class="text-xl font-semibold text-red-500">{{ aiStatus.failedToday }}</div>
          <div class="text-xs text-[var(--text-secondary)]">今日失败</div>
        </div>
      </div>
    </GlassCard>
  </div>
</template>

<script setup lang="ts">
import GlassButton from "@/components/base/GlassButton.vue";
import GlassCard from "@/components/base/GlassCard.vue";
import type { AISettings, AIStatus } from "@/types/api";

interface Props {
  aiSettings: AISettings;
  aiStatus?: AIStatus;
  aiLoading: boolean;
}

defineProps<Props>();

const emit = defineEmits<{
  save: [];
}>();
</script>
