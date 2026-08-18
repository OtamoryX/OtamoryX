<template>
  <div class="space-y-6">
    <GlassCard size="md" radius="lg">
      <div class="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h2 class="text-lg font-medium text-[var(--text-primary)]">
            推荐洞察
          </h2>
          <p class="mt-1 text-sm text-[var(--text-secondary)]">
            只统计从书库“随机精选”进入阅读器后的行为，用来观察推荐是否更贴近你的阅读选择。
          </p>
        </div>
        <div class="inline-flex overflow-hidden rounded-lg border border-[var(--border)]" role="tablist" aria-label="统计周期">
          <button
            v-for="option in periods"
            :key="option.days"
            type="button"
            role="tab"
            :aria-selected="period === option.days"
            :class="[
              'min-w-14 border-l border-[var(--border)] px-3 py-1.5 text-sm transition-colors first:border-l-0',
              period === option.days
                ? 'bg-[var(--accent)] text-white'
                : 'bg-[var(--bg-tertiary)] text-[var(--text-secondary)] hover:text-[var(--text-primary)]',
            ]"
            @click="period = option.days"
          >
            {{ option.label }}
          </button>
        </div>
      </div>

      <div v-if="metricsQuery.isLoading.value" class="mt-6 grid grid-cols-1 divide-y divide-[var(--border)] border-y border-[var(--border)] sm:grid-cols-3 sm:divide-x sm:divide-y-0">
        <div v-for="index in 3" :key="index" class="h-20 animate-pulse bg-[var(--bg-tertiary)]/60" />
      </div>

      <div v-else-if="metricsQuery.isError.value" class="mt-6 flex items-center justify-between gap-3 border-y border-[var(--border)] py-4 text-sm text-[var(--text-secondary)]">
        <span>暂时无法读取推荐数据。</span>
        <button
          type="button"
          title="重新加载推荐数据"
          class="rounded-md p-2 text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] hover:text-[var(--text-primary)]"
          @click="metricsQuery.refetch()"
        >
          <ArrowPathIcon class="h-4 w-4" />
        </button>
      </div>

      <template v-else-if="metrics">
        <div v-if="!hasActivity" class="mt-6 border-y border-[var(--border)] py-5 text-sm text-[var(--text-secondary)]">
          暂无可展示的数据。开始从“随机精选”打开并阅读漫画后，这里会显示推荐与探索内容的表现。
        </div>

        <template v-else>
          <dl class="mt-6 grid grid-cols-1 divide-y divide-[var(--border)] border-y border-[var(--border)] sm:grid-cols-2 sm:divide-x sm:divide-y-0 lg:grid-cols-4">
            <div class="py-4 sm:px-4 sm:first:pl-0 sm:last:pr-0">
              <dt class="text-xs text-[var(--text-secondary)]">有效阅读率</dt>
              <dd class="mt-1 text-2xl font-semibold text-[var(--text-primary)]">
                {{ formatPercent(metrics.overall.effectiveReadRate) }}
              </dd>
              <p class="mt-1 text-xs text-[var(--text-tertiary)]">
                {{ metrics.overall.effectiveReads }} 次有效阅读 / {{ metrics.overall.opened }} 次打开
              </p>
            </div>
            <div class="py-4 sm:px-4">
              <dt class="text-xs text-[var(--text-secondary)]">已打开</dt>
              <dd class="mt-1 text-2xl font-semibold text-[var(--text-primary)]">
                {{ metrics.overall.opened }}
              </dd>
              <p class="mt-1 text-xs text-[var(--text-tertiary)]">
                共展示 {{ metrics.overall.exposed }} 本
              </p>
            </div>
            <div class="py-4 sm:px-4">
              <dt class="text-xs text-[var(--text-secondary)]">快速退出</dt>
              <dd class="mt-1 text-2xl font-semibold text-[var(--text-primary)]">
                {{ metrics.overall.quickExits }}
              </dd>
              <p class="mt-1 text-xs text-[var(--text-tertiary)]">打开后较快离开阅读器</p>
            </div>
            <div class="py-4 sm:px-4 sm:last:pr-0">
              <dt class="text-xs text-[var(--text-secondary)]">每百次打开的手动删除</dt>
              <dd class="mt-1 text-2xl font-semibold text-[var(--text-primary)]">
                {{ formatDecimal(metrics.overall.manualDeletesPer100Opens) }}
              </dd>
              <p class="mt-1 text-xs text-[var(--text-tertiary)]">
                {{ metrics.overall.manualDeletes }} 次手动删除
              </p>
            </div>
          </dl>

          <p v-if="!hasSufficientSample" class="mt-3 text-xs text-amber-700 dark:text-amber-400">
            当前只有 {{ metrics.overall.opened }} 次打开记录，数据仅供观察，暂不适合据此判断推荐效果。
          </p>

          <div class="mt-6">
            <div class="mb-3 flex items-center justify-between gap-3">
              <h3 class="text-sm font-medium text-[var(--text-primary)]">偏好与探索</h3>
              <span class="text-xs text-[var(--text-tertiary)]">按有效阅读率对比</span>
            </div>
            <div class="space-y-4">
              <div v-for="group in recommendationGroups" :key="group.id">
                <div class="mb-1.5 flex items-center justify-between gap-3 text-sm">
                  <div>
                    <span class="font-medium text-[var(--text-primary)]">{{ group.label }}</span>
                    <span class="ml-2 text-xs text-[var(--text-tertiary)]">{{ group.detail }}</span>
                  </div>
                  <span class="shrink-0 font-medium text-[var(--text-primary)]">{{ formatPercent(group.metric.effectiveReadRate) }}</span>
                </div>
                <div class="h-2 overflow-hidden rounded bg-[var(--bg-tertiary)]" :aria-label="`${group.label}有效阅读率 ${formatPercent(group.metric.effectiveReadRate)}`">
                  <div class="h-full rounded bg-[var(--accent)] transition-[width] duration-300" :style="{ width: percentWidth(group.metric.effectiveReadRate) }" />
                </div>
              </div>
            </div>
          </div>

          <dl class="mt-6 grid grid-cols-1 gap-4 border-t border-[var(--border)] pt-4 sm:grid-cols-2">
            <div>
              <dt class="text-xs text-[var(--text-secondary)]">已接触题材</dt>
              <dd class="mt-1 text-sm font-medium text-[var(--text-primary)]">
                {{ metrics.topics.exposedTopicCount }} / {{ metrics.topics.candidateTopicCount }}
              </dd>
              <p class="mt-1 text-xs text-[var(--text-tertiary)]">候选内容中已展示的不同题材数</p>
            </div>
            <div>
              <dt class="text-xs text-[var(--text-secondary)]">探索带来的新题材</dt>
              <dd class="mt-1 text-sm font-medium text-[var(--text-primary)]">
                {{ metrics.topics.explorationTopicCount }}
              </dd>
              <p class="mt-1 text-xs text-[var(--text-tertiary)]">来自未知或新组合的题材数</p>
            </div>
          </dl>
        </template>
      </template>
    </GlassCard>

    <GlassCard size="md" radius="lg">
      <h2 class="text-lg font-medium text-[var(--text-primary)]">推荐如何工作</h2>
      <dl class="mt-4 divide-y divide-[var(--border)] border-y border-[var(--border)]">
        <div class="grid grid-cols-[112px_minmax(0,1fr)] gap-4 py-4">
          <dt class="text-sm font-medium text-[var(--text-primary)]">偏好内容</dt>
          <dd class="text-sm text-[var(--text-secondary)]">系统会结合已完成的内容分析与阅读行为，为更可能合适的内容增加出现机会。</dd>
        </div>
        <div class="grid grid-cols-[112px_minmax(0,1fr)] gap-4 py-4">
          <dt class="text-sm font-medium text-[var(--text-primary)]">探索内容</dt>
          <dd class="text-sm text-[var(--text-secondary)]">随机精选会保留一部分未知内容，避免推荐范围越来越窄。</dd>
        </div>
        <div class="grid grid-cols-[112px_minmax(0,1fr)] gap-4 py-4">
          <dt class="text-sm font-medium text-[var(--text-primary)]">自动清理</dt>
          <dd class="text-sm text-[var(--text-secondary)]">只有内容证据和偏好规则都达到高置信度时才会移入回收站；恢复会作为纠正信号。</dd>
        </div>
      </dl>
    </GlassCard>

    <GlassCard size="md" radius="lg">
      <div class="flex items-start justify-between gap-4">
        <div>
          <h2 class="text-lg font-medium text-[var(--text-primary)]">偏好规则</h2>
          <p class="mt-1 text-sm text-[var(--text-secondary)]">你创建或系统学习出的规则会影响后续随机精选；系统级规则只读。</p>
        </div>
        <button
          type="button"
          title="重新加载偏好规则"
          class="rounded-md p-2 text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] hover:text-[var(--text-primary)]"
          @click="rulesQuery.refetch()"
        >
          <ArrowPathIcon class="h-4 w-4" />
        </button>
      </div>

      <div v-if="rulesQuery.isLoading.value" class="mt-5 h-16 animate-pulse border-y border-[var(--border)] bg-[var(--bg-tertiary)]/60" />
      <p v-else-if="rulesQuery.isError.value" class="mt-5 border-y border-[var(--border)] py-4 text-sm text-[var(--text-secondary)]">暂时无法读取偏好规则。</p>
      <p v-else-if="rules.length === 0" class="mt-5 border-y border-[var(--border)] py-4 text-sm text-[var(--text-secondary)]">系统还在积累阅读信号，尚未形成可展示的偏好规则。</p>
      <div v-else class="mt-5 divide-y divide-[var(--border)] border-y border-[var(--border)]">
        <div v-for="rule in rules" :key="rule.id" class="flex items-start justify-between gap-4 py-4">
          <div class="min-w-0">
            <div class="flex flex-wrap items-center gap-x-2 gap-y-1">
              <span class="text-sm font-medium text-[var(--text-primary)]">{{ rule.name }}</span>
              <span class="text-xs text-[var(--text-secondary)]">{{ actionLabel(rule.action) }}</span>
              <span class="text-xs" :class="ruleStatusClass(rule)">{{ ruleStatus(rule) }}</span>
            </div>
            <p class="mt-1 text-xs text-[var(--text-secondary)]">{{ describeConditions(rule.conditions) }} · 置信度阈值 {{ formatPercent(rule.confidenceThreshold) }}</p>
          </div>
          <button
            v-if="canToggleRule(rule)"
            type="button"
            :title="rule.enabled ? '暂停此规则' : '启用此规则'"
            :disabled="ruleMutation.isPending.value"
            class="shrink-0 rounded-md p-2 text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] hover:text-[var(--text-primary)] disabled:cursor-not-allowed disabled:opacity-50"
            @click="toggleRule(rule)"
          >
            <PauseIcon v-if="rule.enabled" class="h-4 w-4" />
            <PlayIcon v-else class="h-4 w-4" />
          </button>
        </div>
      </div>
    </GlassCard>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { useMutation, useQuery, useQueryClient } from "@tanstack/vue-query";
import { ArrowPathIcon, PauseIcon, PlayIcon } from "@heroicons/vue/24/outline";
import GlassCard from "@/components/base/GlassCard.vue";
import { useAuthStore } from "@/stores/auth";
import {
  getPreferenceRules,
  getRandomRecommendationMetrics,
  setPreferenceRuleEnabled,
} from "@/utils/api";
import type { PreferenceRule, RandomRecommendationMetric } from "@/types/api";

type MetricPeriod = 7 | 30 | 90;

const periods: Array<{ days: MetricPeriod; label: string }> = [
  { days: 7, label: "7 天" },
  { days: 30, label: "30 天" },
  { days: 90, label: "90 天" },
];

const period = ref<MetricPeriod>(30);
const queryClient = useQueryClient();
const authStore = useAuthStore();

const metricsQuery = useQuery({
  queryKey: computed(() => ["random-recommendation-metrics", period.value]),
  queryFn: () => getRandomRecommendationMetrics(period.value),
  staleTime: 60_000,
});

const rulesQuery = useQuery({
  queryKey: ["preference-rules"],
  queryFn: getPreferenceRules,
  staleTime: 60_000,
});

const ruleMutation = useMutation({
  mutationFn: ({ rule, enabled }: { rule: PreferenceRule; enabled: boolean }) =>
    setPreferenceRuleEnabled(rule.id, enabled),
  onSuccess: () => queryClient.invalidateQueries({ queryKey: ["preference-rules"] }),
});

const metrics = computed(() => metricsQuery.data.value);
const rules = computed(() => rulesQuery.data.value ?? []);
const hasActivity = computed(() => (metrics.value?.overall.exposed ?? 0) > 0);
const hasSufficientSample = computed(() => (metrics.value?.overall.opened ?? 0) >= 20);
const recommendationGroups = computed(() => {
  if (!metrics.value) return [] as Array<{ id: string; label: string; detail: string; metric: RandomRecommendationMetric }>;
  return [
    {
      id: "preferred",
      label: "偏好内容",
      detail: `${metrics.value.preferred.opened} 次打开`,
      metric: metrics.value.preferred,
    },
    {
      id: "exploration",
      label: "探索内容",
      detail: `${metrics.value.exploration.opened} 次打开`,
      metric: metrics.value.exploration,
    },
  ];
});

const formatPercent = (value: number) => `${Math.round(value * 100)}%`;
const formatDecimal = (value: number) => Number(value).toFixed(1);
const percentWidth = (value: number) => `${Math.round(Math.max(0, Math.min(1, value)) * 100)}%`;

const actionLabel = (action: PreferenceRule["action"]) => ({
  keep: "优先推荐",
  downrank: "降低推荐",
  auto_delete: "自动清理",
}[action] ?? action);

const ruleStatus = (rule: PreferenceRule) => {
  if (rule.autoPaused) return "已自动暂停";
  return rule.enabled ? "生效中" : "暂未启用";
};

const ruleStatusClass = (rule: PreferenceRule) =>
  rule.autoPaused
    ? "text-amber-700 dark:text-amber-400"
    : rule.enabled
      ? "text-green-700 dark:text-green-400"
      : "text-[var(--text-tertiary)]";

const canToggleRule = (rule: PreferenceRule) => rule.userId === authStore.user?.id && !rule.autoPaused;

const toggleRule = (rule: PreferenceRule) => {
  ruleMutation.mutate({ rule, enabled: !rule.enabled });
};

const describeConditions = (conditions: Record<string, unknown>): string => {
  const value = conditions as Record<string, unknown>;
  const join = (key: "all" | "any", separator: string) => {
    const items = value[key];
    return Array.isArray(items) ? items.map(describeCondition).filter(Boolean).join(separator) : "";
  };
  if (value.all) return join("all", " 且 ") || "组合条件";
  if (value.any) return join("any", " 或 ") || "任一条件";
  if (value.not && typeof value.not === "object") return `不包含 ${describeCondition(value.not as Record<string, unknown>)}`;
  return describeCondition(value) || "内容条件";
};

const describeCondition = (condition: Record<string, unknown>): string => {
  const concept = typeof condition.concept === "string" ? condition.concept : null;
  const theme = typeof condition.theme === "string" ? condition.theme : null;
  const label = concept ?? theme;
  if (!label) return "";
  const confidence = Number(condition.minConfidence);
  return Number.isFinite(confidence) ? `${label}（≥${formatPercent(confidence)}）` : label;
};
</script>
