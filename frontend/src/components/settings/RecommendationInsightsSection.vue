<template>
  <div class="space-y-6">
    <GlassCard size="md" radius="lg">
      <div class="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h2 class="text-lg font-medium text-[var(--text-primary)]">
            {{ t("recommendationInsights.title") }}
          </h2>
          <p class="mt-1 text-sm text-[var(--text-secondary)]">
            {{ t("recommendationInsights.description") }}
          </p>
        </div>
        <div
          class="inline-flex overflow-hidden rounded-lg border border-[var(--border)]"
          role="tablist"
          :aria-label="t('recommendationInsights.aria.period')"
        >
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
            {{ t("recommendationInsights.period", { days: option.days }) }}
          </button>
        </div>
      </div>

      <div
        v-if="metricsQuery.isLoading.value"
        class="mt-6 grid grid-cols-1 divide-y divide-[var(--border)] border-y border-[var(--border)] sm:grid-cols-3 sm:divide-x sm:divide-y-0"
      >
        <div
          v-for="index in 3"
          :key="index"
          class="h-20 animate-pulse bg-[var(--bg-tertiary)]/60"
        />
      </div>

      <div
        v-else-if="metricsQuery.isError.value"
        class="mt-6 flex items-center justify-between gap-3 border-y border-[var(--border)] py-4 text-sm text-[var(--text-secondary)]"
      >
        <span>{{ t("recommendationInsights.error.recommendations") }}</span>
        <button
          type="button"
          :title="t('recommendationInsights.reload.recommendations')"
          class="rounded-md p-2 text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] hover:text-[var(--text-primary)]"
          @click="metricsQuery.refetch()"
        >
          <ArrowPathIcon class="h-4 w-4" />
        </button>
      </div>

      <template v-else-if="metrics">
        <div
          v-if="!hasActivity"
          class="mt-6 border-y border-[var(--border)] py-5 text-sm text-[var(--text-secondary)]"
        >
          {{ t("recommendationInsights.empty.recommendations") }}
        </div>

        <template v-else>
          <dl
            class="mt-6 grid grid-cols-1 divide-y divide-[var(--border)] border-y border-[var(--border)] sm:grid-cols-2 sm:divide-x sm:divide-y-0 lg:grid-cols-4"
          >
            <div class="py-4 sm:px-4 sm:first:pl-0 sm:last:pr-0">
              <dt class="text-xs text-[var(--text-secondary)]">
                {{ t("recommendationInsights.metrics.effectiveReadRate") }}
              </dt>
              <dd
                class="mt-1 text-2xl font-semibold text-[var(--text-primary)]"
              >
                {{ formatPercent(metrics.overall.effectiveReadRate) }}
              </dd>
              <p class="mt-1 text-xs text-[var(--text-tertiary)]">
                {{
                  t("recommendationInsights.metrics.effectiveReadSummary", {
                    effectiveReads: metrics.overall.effectiveReads,
                    opened: metrics.overall.opened,
                  })
                }}
              </p>
            </div>
            <div class="py-4 sm:px-4">
              <dt class="text-xs text-[var(--text-secondary)]">
                {{ t("recommendationInsights.metrics.opened") }}
              </dt>
              <dd
                class="mt-1 text-2xl font-semibold text-[var(--text-primary)]"
              >
                {{ metrics.overall.opened }}
              </dd>
              <p class="mt-1 text-xs text-[var(--text-tertiary)]">
                {{
                  t("recommendationInsights.metrics.exposed", {
                    count: metrics.overall.exposed,
                  })
                }}
              </p>
            </div>
            <div class="py-4 sm:px-4">
              <dt class="text-xs text-[var(--text-secondary)]">
                {{ t("recommendationInsights.metrics.quickExits") }}
              </dt>
              <dd
                class="mt-1 text-2xl font-semibold text-[var(--text-primary)]"
              >
                {{ metrics.overall.quickExits }}
              </dd>
              <p class="mt-1 text-xs text-[var(--text-tertiary)]">
                {{ t("recommendationInsights.metrics.quickExitDetail") }}
              </p>
            </div>
            <div class="py-4 sm:px-4 sm:last:pr-0">
              <dt class="text-xs text-[var(--text-secondary)]">
                {{
                  t("recommendationInsights.metrics.manualDeletesPer100Opens")
                }}
              </dt>
              <dd
                class="mt-1 text-2xl font-semibold text-[var(--text-primary)]"
              >
                {{ formatDecimal(metrics.overall.manualDeletesPer100Opens) }}
              </dd>
              <p class="mt-1 text-xs text-[var(--text-tertiary)]">
                {{
                  t("recommendationInsights.metrics.manualDeleteDetail", {
                    count: metrics.overall.manualDeletes,
                  })
                }}
              </p>
            </div>
          </dl>

          <p
            v-if="!hasSufficientSample"
            class="mt-3 text-xs text-amber-700 dark:text-amber-400"
          >
            {{
              t("recommendationInsights.metrics.sampleWarning", {
                count: metrics.overall.opened,
              })
            }}
          </p>

          <div class="mt-6">
            <div class="mb-3 flex items-center justify-between gap-3">
              <h3 class="text-sm font-medium text-[var(--text-primary)]">
                {{ t("recommendationInsights.comparison.title") }}
              </h3>
              <span class="text-xs text-[var(--text-tertiary)]">{{
                t("recommendationInsights.comparison.detail")
              }}</span>
            </div>
            <div class="space-y-4">
              <div v-for="group in recommendationGroups" :key="group.id">
                <div
                  class="mb-1.5 flex items-center justify-between gap-3 text-sm"
                >
                  <div>
                    <span class="font-medium text-[var(--text-primary)]">{{
                      group.label
                    }}</span>
                    <span class="ml-2 text-xs text-[var(--text-tertiary)]">{{
                      group.detail
                    }}</span>
                  </div>
                  <span
                    class="shrink-0 font-medium text-[var(--text-primary)]"
                    >{{ formatPercent(group.metric.effectiveReadRate) }}</span
                  >
                </div>
                <div
                  class="h-2 overflow-hidden rounded bg-[var(--bg-tertiary)]"
                  :aria-label="
                    t('recommendationInsights.aria.recommendationRate', {
                      label: group.label,
                      rate: formatPercent(group.metric.effectiveReadRate),
                    })
                  "
                >
                  <div
                    class="h-full rounded bg-[var(--accent)] transition-[width] duration-300"
                    :style="{
                      width: percentWidth(group.metric.effectiveReadRate),
                    }"
                  />
                </div>
              </div>
            </div>
          </div>

          <dl
            class="mt-6 grid grid-cols-1 gap-4 border-t border-[var(--border)] pt-4 sm:grid-cols-2"
          >
            <div>
              <dt class="text-xs text-[var(--text-secondary)]">
                {{ t("recommendationInsights.topics.exposed") }}
              </dt>
              <dd class="mt-1 text-sm font-medium text-[var(--text-primary)]">
                {{ metrics.topics.exposedTopicCount }} /
                {{ metrics.topics.candidateTopicCount }}
              </dd>
              <p class="mt-1 text-xs text-[var(--text-tertiary)]">
                {{ t("recommendationInsights.topics.exposedDetail") }}
              </p>
            </div>
            <div>
              <dt class="text-xs text-[var(--text-secondary)]">
                {{ t("recommendationInsights.topics.exploration") }}
              </dt>
              <dd class="mt-1 text-sm font-medium text-[var(--text-primary)]">
                {{ metrics.topics.explorationTopicCount }}
              </dd>
              <p class="mt-1 text-xs text-[var(--text-tertiary)]">
                {{ t("recommendationInsights.topics.explorationDetail") }}
              </p>
            </div>
          </dl>
        </template>
      </template>
    </GlassCard>

    <GlassCard size="md" radius="lg">
      <h2 class="text-lg font-medium text-[var(--text-primary)]">
        {{ t("recommendationInsights.howItWorks.title") }}
      </h2>
      <dl
        class="mt-4 divide-y divide-[var(--border)] border-y border-[var(--border)]"
      >
        <div class="grid grid-cols-[112px_minmax(0,1fr)] gap-4 py-4">
          <dt class="text-sm font-medium text-[var(--text-primary)]">
            {{ t("recommendationInsights.howItWorks.preferredTitle") }}
          </dt>
          <dd class="text-sm text-[var(--text-secondary)]">
            {{ t("recommendationInsights.howItWorks.preferredDescription") }}
          </dd>
        </div>
        <div class="grid grid-cols-[112px_minmax(0,1fr)] gap-4 py-4">
          <dt class="text-sm font-medium text-[var(--text-primary)]">
            {{ t("recommendationInsights.howItWorks.explorationTitle") }}
          </dt>
          <dd class="text-sm text-[var(--text-secondary)]">
            {{ t("recommendationInsights.howItWorks.explorationDescription") }}
          </dd>
        </div>
        <div class="grid grid-cols-[112px_minmax(0,1fr)] gap-4 py-4">
          <dt class="text-sm font-medium text-[var(--text-primary)]">
            {{ t("recommendationInsights.howItWorks.cleanupTitle") }}
          </dt>
          <dd class="text-sm text-[var(--text-secondary)]">
            {{ t("recommendationInsights.howItWorks.cleanupDescription") }}
          </dd>
        </div>
      </dl>
    </GlassCard>

    <GlassCard size="md" radius="lg">
      <div class="flex items-start justify-between gap-4">
        <div>
          <h2 class="text-lg font-medium text-[var(--text-primary)]">
            {{ t("recommendationInsights.rules.title") }}
          </h2>
          <p class="mt-1 text-sm text-[var(--text-secondary)]">
            {{ t("recommendationInsights.rules.description") }}
          </p>
        </div>
        <button
          type="button"
          :title="t('recommendationInsights.reload.rules')"
          class="rounded-md p-2 text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] hover:text-[var(--text-primary)]"
          @click="rulesQuery.refetch()"
        >
          <ArrowPathIcon class="h-4 w-4" />
        </button>
      </div>

      <div
        v-if="rulesQuery.isLoading.value"
        class="mt-5 h-16 animate-pulse border-y border-[var(--border)] bg-[var(--bg-tertiary)]/60"
      />
      <p
        v-else-if="rulesQuery.isError.value"
        class="mt-5 border-y border-[var(--border)] py-4 text-sm text-[var(--text-secondary)]"
      >
        {{ t("recommendationInsights.error.rules") }}
      </p>
      <p
        v-else-if="rules.length === 0"
        class="mt-5 border-y border-[var(--border)] py-4 text-sm text-[var(--text-secondary)]"
      >
        {{ t("recommendationInsights.empty.rules") }}
      </p>
      <div
        v-else
        class="mt-5 divide-y divide-[var(--border)] border-y border-[var(--border)]"
      >
        <div
          v-for="rule in rules"
          :key="rule.id"
          class="flex items-start justify-between gap-4 py-4"
        >
          <div class="min-w-0">
            <div class="flex flex-wrap items-center gap-x-2 gap-y-1">
              <span class="text-sm font-medium text-[var(--text-primary)]">{{
                displayRuleName(rule)
              }}</span>
              <span class="text-xs text-[var(--text-secondary)]">{{
                actionLabel(rule.action)
              }}</span>
              <span class="text-xs" :class="ruleStatusClass(rule)">{{
                ruleStatus(rule)
              }}</span>
            </div>
            <p class="mt-1 text-xs text-[var(--text-secondary)]">
              {{ describeConditions(rule.conditions) }} ·
              {{
                t("recommendationInsights.rules.threshold", {
                  value: formatPercent(rule.confidenceThreshold),
                })
              }}
            </p>
          </div>
          <button
            v-if="canToggleRule(rule)"
            type="button"
            :title="
              rule.enabled
                ? t('recommendationInsights.rules.pause')
                : t('recommendationInsights.rules.enable')
            "
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
import { useI18n } from "vue-i18n";
import GlassCard from "@/components/base/GlassCard.vue";
import { useAuthStore } from "@/stores/auth";
import {
  parsePreferenceRuleDisplayName,
  type ParsedPreferenceRule,
} from "@/utils/preferenceRuleDisplay";
import {
  getPreferenceRules,
  getRandomRecommendationMetrics,
  setPreferenceRuleEnabled,
} from "@/utils/api";
import type { PreferenceRule, RandomRecommendationMetric } from "@/types/api";

type MetricPeriod = 7 | 30 | 90;

const periods: Array<{ days: MetricPeriod }> = [
  { days: 7 },
  { days: 30 },
  { days: 90 },
];

const { t } = useI18n();
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
  onSuccess: () =>
    queryClient.invalidateQueries({ queryKey: ["preference-rules"] }),
});

const metrics = computed(() => metricsQuery.data.value);
const rules = computed(() => rulesQuery.data.value ?? []);
const hasActivity = computed(() => (metrics.value?.overall.exposed ?? 0) > 0);
const hasSufficientSample = computed(
  () => (metrics.value?.overall.opened ?? 0) >= 20,
);
const recommendationGroups = computed(() => {
  if (!metrics.value)
    return [] as Array<{
      id: string;
      label: string;
      detail: string;
      metric: RandomRecommendationMetric;
    }>;
  return [
    {
      id: "preferred",
      label: t("recommendationInsights.comparison.preferred"),
      detail: t("recommendationInsights.comparison.opened", {
        count: metrics.value.preferred.opened,
      }),
      metric: metrics.value.preferred,
    },
    {
      id: "exploration",
      label: t("recommendationInsights.comparison.exploration"),
      detail: t("recommendationInsights.comparison.opened", {
        count: metrics.value.exploration.opened,
      }),
      metric: metrics.value.exploration,
    },
  ];
});

const formatPercent = (value: number) => `${Math.round(value * 100)}%`;
const formatDecimal = (value: number) => Number(value).toFixed(1);
const percentWidth = (value: number) =>
  `${Math.round(Math.max(0, Math.min(1, value)) * 100)}%`;

const actionLabel = (action: PreferenceRule["action"]) =>
  ({
    keep: t("recommendationInsights.actions.keep"),
    downrank: t("recommendationInsights.actions.downrank"),
    auto_delete: t("recommendationInsights.actions.autoDelete"),
  })[action] ?? action;

const ruleStatus = (rule: PreferenceRule) => {
  if (rule.autoPaused) return t("recommendationInsights.statuses.autoPaused");
  return rule.enabled
    ? t("recommendationInsights.statuses.enabled")
    : t("recommendationInsights.statuses.disabled");
};

const displayRuleName = (rule: PreferenceRule) => {
  const parsedName = parsePreferenceRuleDisplayName(rule.name);
  if (rule.source !== "learned_cold_start" && !parsedName) return rule.name;

  const condition = parsedName
    ? describeCondition(parsedNameToCondition(parsedName))
    : describeConditions(rule.conditions);
  return t("recommendationInsights.rules.learnedName", {
    condition: condition || t("recommendationInsights.conditions.content"),
  });
};

const parsedNameToCondition = (parsed: ParsedPreferenceRule) =>
  parsed.operator === "between"
    ? {
        feature: parsed.feature,
        operator: parsed.operator,
        min: parsed.min,
        max: parsed.max,
      }
    : {
        feature: parsed.feature,
        operator: parsed.operator,
        value: parsed.value,
      };

const ruleStatusClass = (rule: PreferenceRule) =>
  rule.autoPaused
    ? "text-amber-700 dark:text-amber-400"
    : rule.enabled
      ? "text-green-700 dark:text-green-400"
      : "text-[var(--text-tertiary)]";

const canToggleRule = (rule: PreferenceRule) =>
  rule.userId === authStore.user?.id && !rule.autoPaused;

const toggleRule = (rule: PreferenceRule) => {
  ruleMutation.mutate({ rule, enabled: !rule.enabled });
};

const describeConditions = (conditions: Record<string, unknown>): string => {
  const value = conditions as Record<string, unknown>;
  const join = (key: "all" | "any", separator: string) => {
    const items = value[key];
    return Array.isArray(items)
      ? items.map(describeCondition).filter(Boolean).join(separator)
      : "";
  };
  if (value.all)
    return (
      join("all", t("recommendationInsights.conditions.all")) ||
      t("recommendationInsights.conditions.combination")
    );
  if (value.any)
    return (
      join("any", t("recommendationInsights.conditions.any")) ||
      t("recommendationInsights.conditions.anyCondition")
    );
  if (value.not && typeof value.not === "object") {
    return t("recommendationInsights.conditions.not", {
      condition: describeCondition(value.not as Record<string, unknown>),
    });
  }
  return (
    describeCondition(value) || t("recommendationInsights.conditions.content")
  );
};

const describeCondition = (condition: Record<string, unknown>): string => {
  const concept =
    typeof condition.concept === "string" ? condition.concept : null;
  const theme = typeof condition.theme === "string" ? condition.theme : null;
  const label = concept ?? theme;
  if (label) {
    const confidence = Number(condition.minConfidence);
    return Number.isFinite(confidence)
      ? `${label}${t("recommendationInsights.conditions.confidence", { value: formatPercent(confidence) })}`
      : label;
  }

  const feature =
    typeof condition.feature === "string" ? condition.feature : null;
  if (!feature) return "";
  const featureLabel = describeFeature(feature);
  const operator =
    typeof condition.operator === "string" ? condition.operator : null;
  if (operator === "between") {
    const min = Number(condition.min);
    const max = Number(condition.max);
    if (Number.isFinite(min) && Number.isFinite(max)) {
      return t("recommendationInsights.conditions.between", {
        feature: featureLabel,
        min: formatConditionValue(min),
        max: formatConditionValue(max),
      });
    }
  }
  if (operator === "eq") {
    const value = Number(condition.value);
    if (value === 1 && /^(tag|theme):/.test(feature)) {
      return t("recommendationInsights.conditions.matches", {
        feature: featureLabel,
      });
    }
    if (Number.isFinite(value)) {
      return t("recommendationInsights.conditions.equals", {
        feature: featureLabel,
        value: formatConditionValue(value),
      });
    }
  }
  return featureLabel;
};

const describeFeature = (feature: string): string => {
  const tag = /^tag:[^:]+:(.+)$/.exec(feature);
  if (tag) return t("recommendationInsights.conditions.tag", { name: tag[1] });
  const theme = /^theme:(.+)$/.exec(feature);
  if (theme)
    return t("recommendationInsights.conditions.theme", { id: theme[1] });
  const key = `recommendationInsights.conditions.features.${feature}`;
  const translated = t(key);
  return translated === key
    ? t("recommendationInsights.conditions.unknownFeature", { feature })
    : translated;
};

const formatConditionValue = (value: number): string => {
  if (Number.isInteger(value)) return String(value);
  return value.toFixed(2).replace(/\.?(0+)$/, "");
};
</script>
