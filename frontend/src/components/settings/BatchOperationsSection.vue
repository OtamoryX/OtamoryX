<template>
  <div class="space-y-6">
    <GlassCard size="md" radius="lg">
      <h2 class="text-lg font-medium text-[var(--text-primary)]">批量维护</h2>
      <p class="mt-1 text-sm text-[var(--text-secondary)]">用于执行清理类和批量删除类维护任务。</p>
    </GlassCard>

    <GlassCard size="md" radius="lg" class="border-red-400/30 bg-red-500/5">
      <div class="mb-3 flex items-center justify-between">
        <h3 class="text-base font-medium text-red-500">Danger Zone</h3>
        <span class="text-xs text-red-400">不可撤销</span>
      </div>

      <div class="grid grid-cols-1 gap-4 lg:grid-cols-3">
        <div class="rounded-lg border border-red-400/25 bg-[var(--bg-tertiary)] p-4">
          <div class="mb-2 text-sm font-medium text-[var(--text-primary)]">按 ID 批量删除</div>
          <p class="mb-3 text-xs text-[var(--text-secondary)]">输入逗号分隔 ID，留空表示删除全部（高风险）。</p>
          <input
            v-model="batchDeleteForm.archiveIds"
            type="text"
            placeholder="1,2,3"
            class="mb-3 w-full rounded-lg border border-[var(--border)] bg-[var(--bg-card)] px-3 py-2 text-sm text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-red-400"
          />
          <GlassButton :disabled="batchOperationLoading" variant="danger" size="sm" full-width @click="emit('delete-archives')">
            {{ batchOperationLoading ? "处理中..." : "批量删除漫画" }}
          </GlassButton>
        </div>

        <div class="rounded-lg border border-red-400/25 bg-[var(--bg-tertiary)] p-4">
          <div class="mb-2 text-sm font-medium text-[var(--text-primary)]">按分类删除</div>
          <p class="mb-3 text-xs text-[var(--text-secondary)]">删除指定分类下的所有漫画。</p>
          <select
            v-model="batchDeleteForm.categoryId"
            class="mb-3 w-full rounded-lg border border-[var(--border)] bg-[var(--bg-card)] px-3 py-2 text-sm text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-red-400"
          >
            <option value="">选择分类</option>
            <option v-for="category in categories" :key="category.id" :value="category.id">{{ category.name }}</option>
          </select>
          <GlassButton
            :disabled="batchOperationLoading || !batchDeleteForm.categoryId"
            variant="danger"
            size="sm"
            full-width
            @click="emit('delete-category')"
          >
            {{ batchOperationLoading ? "处理中..." : "删除分类漫画" }}
          </GlassButton>
        </div>

        <div class="rounded-lg border border-red-400/25 bg-[var(--bg-tertiary)] p-4">
          <div class="mb-2 text-sm font-medium text-[var(--text-primary)]">按标签删除</div>
          <p class="mb-3 text-xs text-[var(--text-secondary)]">删除指定标签下的所有漫画。</p>
          <select
            v-model="batchDeleteForm.tagId"
            class="mb-3 w-full rounded-lg border border-[var(--border)] bg-[var(--bg-card)] px-3 py-2 text-sm text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-red-400"
          >
            <option value="">选择标签</option>
            <option v-for="tag in tags" :key="tag.id" :value="tag.id">{{ tag.namespace }}:{{ tag.name }}</option>
          </select>
          <GlassButton
            :disabled="batchOperationLoading || !batchDeleteForm.tagId"
            variant="danger"
            size="sm"
            full-width
            @click="emit('delete-tag')"
          >
            {{ batchOperationLoading ? "处理中..." : "删除标签漫画" }}
          </GlassButton>
        </div>
      </div>
    </GlassCard>

    <GlassCard size="md" radius="lg">
      <h3 class="mb-3 text-base font-medium text-[var(--text-primary)]">数据清理</h3>
      <div class="grid grid-cols-1 gap-4 md:grid-cols-2">
        <div class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-4">
          <div class="mb-2 text-sm font-medium text-[var(--text-primary)]">清理无用标签</div>
          <p class="mb-3 text-xs text-[var(--text-secondary)]">删除没有关联漫画的标签（系统标签除外）。</p>
          <GlassButton :disabled="batchOperationLoading" variant="warning" size="sm" full-width @click="emit('prune-tags')">
            {{ batchOperationLoading ? "处理中..." : "清理无用标签" }}
          </GlassButton>
        </div>

        <div class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-4">
          <div class="mb-2 text-sm font-medium text-[var(--text-primary)]">清理空分类</div>
          <p class="mb-3 text-xs text-[var(--text-secondary)]">删除不包含漫画的分类。</p>
          <GlassButton
            :disabled="batchOperationLoading"
            variant="warning"
            size="sm"
            full-width
            @click="emit('prune-categories')"
          >
            {{ batchOperationLoading ? "处理中..." : "清理空分类" }}
          </GlassButton>
        </div>
      </div>
    </GlassCard>

    <GlassCard v-if="batchOperationHistory.length > 0" size="md" radius="lg">
      <div class="mb-3 flex items-center justify-between">
        <h3 class="text-base font-medium text-[var(--text-primary)]">操作历史</h3>
        <GlassButton size="xs" variant="ghost" @click="emit('clear-history')">清空历史</GlassButton>
      </div>

      <div class="space-y-2">
        <div
          v-for="(record, index) in batchOperationHistory"
          :key="index"
          class="flex items-center justify-between rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3"
        >
          <div>
            <div class="text-sm font-medium text-[var(--text-primary)]">{{ record.operation }}</div>
            <div class="text-xs text-[var(--text-secondary)]">{{ record.timestamp }}</div>
          </div>
          <div class="text-right">
            <div :class="record.success ? 'text-green-500' : 'text-red-500'" class="text-sm font-medium">
              {{ record.success ? "成功" : "失败" }}
            </div>
            <div class="text-xs text-[var(--text-secondary)]">{{ record.result }}</div>
          </div>
        </div>
      </div>
    </GlassCard>
  </div>
</template>

<script setup lang="ts">
import GlassButton from "@/components/base/GlassButton.vue";
import GlassCard from "@/components/base/GlassCard.vue";
import type { Category, Tag } from "@/types/api";
import type { BatchDeleteForm, BatchOperationRecord } from "@/types/settings";

interface Props {
  categories?: Category[];
  tags?: Tag[];
  batchDeleteForm: BatchDeleteForm;
  batchOperationLoading: boolean;
  batchOperationHistory: BatchOperationRecord[];
}

defineProps<Props>();

const emit = defineEmits<{
  "delete-archives": [];
  "delete-category": [];
  "delete-tag": [];
  "prune-tags": [];
  "prune-categories": [];
  "clear-history": [];
}>();
</script>
