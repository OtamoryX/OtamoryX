// @vitest-environment jsdom

import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { defineComponent, h } from "vue";
import AISettingsSection from "@/components/settings/AISettingsSection.vue";
import { i18n } from "@/i18n";
import type {
  AISettings,
  AIStatus,
  AITaskQueueStatus,
  WeightedTagGraphStatus,
} from "@/types/api";

const GlassCardStub = defineComponent({
  setup(_, { slots }) {
    return () => h("div", slots.default?.());
  },
});

const GlassButtonStub = defineComponent({
  props: {
    disabled: Boolean,
    loading: Boolean,
  },
  emits: ["click"],
  setup(props, { emit, slots }) {
    return () =>
      h(
        "button",
        {
          disabled: props.disabled || props.loading,
          onClick: () => emit("click"),
        },
        slots.default?.(),
      );
  },
});

const TaskExecutionSettingsStub = defineComponent({
  setup(_, { slots }) {
    return () => h("div", slots.default?.());
  },
});

const taskQueue = (
  overrides: Partial<AITaskQueueStatus>,
): AITaskQueueStatus => ({
  jobType: "content_analysis_reconcile",
  pendingCount: 0,
  processingCount: 0,
  waitingForModelCount: 0,
  waitingForDependencyCount: 0,
  retryWaitingCount: 0,
  manuallyPaused: false,
  state: "idle",
  blockedUntil: null,
  nextRunAt: null,
  lastError: null,
  requiresModel: false,
  blockingScope: null,
  blockingReason: null,
  availableActions: [],
  ...overrides,
});

const aiSettings = (): AISettings => ({
  settingsVersion: 1,
  connection: {
    provider: "ollama",
    baseUrl: "http://localhost:11434",
    model: "test-model",
    streamResponse: false,
    firstTokenTimeoutSeconds: 30,
    requestIntervalSeconds: 0,
    ollamaUseGpu: false,
    ollamaMaxNumCtx: 256,
    contextWindowTokens: 256,
    ollamaThinking: false,
    ollamaRepeatPenalty: 1,
    ollamaRepeatLastN: 1,
    visionCapable: false,
    authMode: "none",
    apiKeyConfigured: false,
  },
  profiles: [],
  activeProfileId: "",
  execution: {
    lanes: { llm: 1, ocr: 1, plugin: 1, orchestration: 1 },
    timeoutSeconds: 30,
    maxRetries: 1,
    maxImagesPerTask: 1,
    imageTokenBudget: 1,
    outputTokenLimit: 1,
    thinkingOutputTokenLimit: 1,
    promptSafetyMargin: 1,
    adaptiveContextRetries: 0,
    ocrMaxPages: 1,
    ocrCharsPerPage: 1,
  },
  features: {
    titleTranslation: {
      enabled: false,
      targetLanguage: "zh-CN",
      skipIfTargetLanguage: false,
      retranslateOnTitleChange: false,
      displayTranslatedTitle: false,
      execution: {} as AISettings["features"]["titleTranslation"]["execution"],
    },
    tagLocalization: {
      enabled: false,
      execution: {} as AISettings["features"]["tagLocalization"]["execution"],
    },
    contentUnderstanding: {
      execution:
        {} as AISettings["features"]["contentUnderstanding"]["execution"],
    },
    autoTagging: {
      enabled: false,
      mode: "suggestions",
      autoProcessNewArchives: false,
      execution: {} as AISettings["features"]["autoTagging"]["execution"],
    },
    recommendations: {
      tagGraphEnabled: true,
      multiUserExperimentEnabled: false,
      analysisRefreshAfterDays: 1,
      tagRelation: {
        transport: "openrouterAlphaDecisions",
        endpoint: "https://openrouter.ai/api/alpha/decisions",
        gpuGateEndpoint: "http://gpu-gate:8090/v1/jev/alpha/decisions",
        model: "~typesafe/jev-latest",
        apiKey: "",
        apiKeyConfigured: false,
        batchSize: 4,
        maxPairsPerTrigger: 100,
        candidateAlgorithmVersion: "tag-cooccurrence-candidates-v1",
        protocolVersion: "openrouter-alpha-decisions-v1",
        promptVersion: "jev-tag-relation-choice-alpha-v1",
        schemaVersion: "jev-alpha-choice-relation-v1",
        minConfidence: 0.7,
        execution: {} as AISettings["features"]["autoTagging"]["execution"],
      },
    },
  },
});

const aiStatus = (): AIStatus => ({
  queueSize: 3,
  processingCount: 1,
  completedToday: 0,
  failedToday: 0,
  languageDetectionPending: 0,
  retryWaiting: 0,
  unresolvedFailureCount: 0,
  providerBlockedUntil: null,
  averageProcessingTime: 0,
  activeModels: [],
  queueByLane: {},
  executorLanes: [],
  modelStates: [],
  taskQueues: [
    taskQueue({
      jobType: "content_analysis_reconcile",
      waitingForDependencyCount: 1,
      state: "waiting_for_dependency",
      blockingScope: "task",
      blockingReason: "dependency_wait",
      availableActions: ["forceContinue", "pause"],
    }),
    taskQueue({
      jobType: "content_analysis_synthesize",
      processingCount: 1,
      state: "running",
      requiresModel: true,
      availableActions: ["pause"],
    }),
    taskQueue({
      jobType: "content_analysis_canonicalize",
      pendingCount: 1,
      state: "queued",
      requiresModel: true,
      availableActions: ["pause"],
    }),
  ],
});

const tagGraphStatus = (
  overrides: Partial<WeightedTagGraphStatus> = {},
): WeightedTagGraphStatus => ({
  enabled: true,
  configured: true,
  state: "ready",
  activeRelationCount: 4,
  queuedTaskCount: 2,
  processingTaskCount: 1,
  retryWaitingTaskCount: 0,
  paused: false,
  nextRetryAt: null,
  lastError: null,
  ...overrides,
});

const mountSection = (
  section: "overview" | "models" | "tasks" = "overview",
  status: AIStatus = aiStatus(),
  graphStatus: WeightedTagGraphStatus | null = null,
  graphStatusError = false,
) =>
  mount(AISettingsSection, {
    props: {
      section,
      aiSettings: aiSettings(),
      aiStatus: status,
      weightedTagGraphStatus: graphStatus,
      weightedTagGraphStatusLoading: false,
      weightedTagGraphStatusError: graphStatusError,
      aiLoading: false,
      aiDirty: false,
      savedMessage: null,
      saveError: null,
      testingConnection: false,
      previewingTitleTranslation: false,
      titleTranslationPreview: null,
      backfillingTranslations: false,
      repairingTranslations: false,
      retranslatingTranslations: false,
      backfillingTagging: false,
      backfillingTagLocalizations: false,
      loadingTagSuggestions: false,
      tagSuggestions: [],
      tagSuggestionsTotal: 0,
      tagSuggestionsPage: 1,
      tagSuggestionsPageCount: 1,
      reviewingTagSuggestionId: null,
      undoingTaggingRunId: null,
      recentTaggingRunIds: [],
      controllingTaskQueue: null,
      controllingModel: null,
    },
    global: {
      plugins: [i18n],
      stubs: {
        GlassCard: GlassCardStub,
        GlassButton: GlassButtonStub,
        TaskExecutionSettings: TaskExecutionSettingsStub,
      },
    },
  });

describe("AISettingsSection task queue controls", () => {
  it("labels tag relation scoring and explains paused cache behavior", () => {
    const status = aiStatus();
    status.taskQueues.push(
      taskQueue({
        jobType: "tag_relation_jev",
        manuallyPaused: true,
        state: "manually_paused",
      }),
    );
    const wrapper = mountSection("overview", status);

    expect(wrapper.text()).toContain("标签关联评分");
    expect(wrapper.text()).toContain("已有缓存继续参与推荐");
  });

  it("shows pause alongside force continue for mixed tagging queues", () => {
    const wrapper = mountSection();
    const buttonTexts = wrapper
      .findAll("button")
      .map((button) => button.text().trim());

    expect(buttonTexts).toContain("暂停");
    expect(buttonTexts).toContain("强制继续");
  });

  it("emits pause for all shared tagging queue stages", async () => {
    const wrapper = mountSection();
    const pauseButton = wrapper
      .findAll("button")
      .find((button) => button.text().trim() === "暂停");

    expect(pauseButton).toBeDefined();
    await pauseButton?.trigger("click");

    expect(wrapper.emitted("control-task-queue")).toEqual([
      [
        [
          "content_analysis_reconcile",
          "content_analysis_synthesize",
          "content_analysis_canonicalize",
        ],
        "pause",
      ],
    ]);
  });
});

describe("AISettingsSection task settings", () => {
  it("keeps recommendation mode while removing content understanding settings", () => {
    const wrapper = mountSection("tasks");
    const text = wrapper.text();

    expect(text).toContain("推荐方式");
    expect(text).toContain("批量生成自动标签");
    expect(text).not.toContain("关系判断模型");
    expect(text).not.toContain("内容理解高级配置");
    expect(text).not.toContain("内容理解更新间隔");
    expect(text).not.toContain("批量分析并打标签");
  });

  it("keeps the single tag relation control checked by default", () => {
    const wrapper = mountSection("tasks", aiStatus(), tagGraphStatus());
    const toggles = wrapper.findAll('input[data-testid="tag-graph-toggle"]');

    expect(toggles).toHaveLength(1);
    expect((toggles[0].element as HTMLInputElement).checked).toBe(true);
    expect(wrapper.text()).toContain("标签关联推荐");
    expect(wrapper.text()).toContain("待处理标签对上限");
    expect(wrapper.text()).not.toContain("观察标签语义关系");
  });

  it("uses operational copy for graph scanning and readiness states", () => {
    const cases = [
      { state: "waiting_tags", label: "等待标签" },
      { state: "updating", label: "正在更新" },
      { state: "ready", label: "已启用" },
    ] as const;

    for (const { state, label } of cases) {
      const wrapper = mountSection(
        "tasks",
        aiStatus(),
        tagGraphStatus({ state }),
      );

      expect(wrapper.get('[data-testid="tag-graph-status"]').text()).toContain(
        label,
      );
      expect(
        wrapper.findAll('input[data-testid="tag-graph-toggle"]'),
      ).toHaveLength(1);
      wrapper.unmount();
    }
  });

  it("updates only the product intent when switched off", async () => {
    const wrapper = mountSection("tasks", aiStatus(), tagGraphStatus());
    const toggle = wrapper.get('input[data-testid="tag-graph-toggle"]');

    await toggle.setValue(false);

    expect((toggle.element as HTMLInputElement).checked).toBe(false);
    expect(
      wrapper.props().aiSettings.features.recommendations.tagGraphEnabled,
    ).toBe(false);
  });

  it("shows setup and task details without treating task counts as relation counts", async () => {
    const wrapper = mountSection(
      "tasks",
      aiStatus(),
      tagGraphStatus({
        configured: false,
        state: "unconfigured",
        activeRelationCount: 0,
      }),
    );

    expect(wrapper.text()).toContain("待配置");
    expect(wrapper.text()).toContain("可用关联 0");
    expect(wrapper.text()).toContain("待处理任务 2");

    const configure = wrapper
      .findAll("button")
      .find((button) => button.text().trim() === "配置服务");
    const details = wrapper
      .findAll("button")
      .find((button) => button.text().trim() === "任务详情");
    expect(configure).toBeDefined();
    expect(details).toBeDefined();
    await configure?.trigger("click");
    await details?.trigger("click");
    expect(wrapper.emitted("open-jev-settings")).toHaveLength(1);
    expect(wrapper.emitted("view-task-queue")).toHaveLength(1);
  });

  it("explains that cached relations remain usable while scoring is paused", () => {
    const wrapper = mountSection(
      "tasks",
      aiStatus(),
      tagGraphStatus({ state: "paused", paused: true }),
    );

    expect(wrapper.text()).toContain("评分已暂停");
    expect(wrapper.text()).toContain("已有缓存继续参与推荐");
  });

  it("shows retry time and provider errors for scoring attention states", () => {
    const wrapper = mountSection(
      "tasks",
      aiStatus(),
      tagGraphStatus({
        state: "needs_attention",
        nextRetryAt: "2030-01-01T00:00:00Z",
        lastError: "sample provider error",
      }),
    );

    expect(wrapper.text()).toContain("需处理");
    expect(wrapper.text()).toContain("下次重试：");
    expect(wrapper.text()).toContain("原因：sample provider error");
  });

  it("shows status request failures without hiding the switch", () => {
    const wrapper = mountSection("tasks", aiStatus(), null, true);

    expect(wrapper.text()).toContain("状态暂不可用");
    expect(wrapper.find('[data-testid="tag-graph-toggle"]').exists()).toBe(
      true,
    );
  });
});

describe("AISettingsSection JEV settings", () => {
  it("shows an independent endpoint, model, and write-only key field", () => {
    const wrapper = mountSection("models");

    expect(wrapper.text()).toContain("JEV Alpha Decisions");
    const endpoint = wrapper.find('input[type="url"]')
      .element as HTMLInputElement;
    expect(endpoint.value).toBe("https://openrouter.ai/api/alpha/decisions");
    expect(wrapper.find('input[type="password"]').exists()).toBe(true);
    expect(wrapper.text()).toContain("未配置密钥");
  });

  it("selects the GPU Gate relay endpoint independently from direct JEV", async () => {
    const wrapper = mountSection("models");

    await wrapper
      .find('[data-testid="jev-transport"]')
      .setValue("gpuGateAlphaDecisions");

    const endpoint = wrapper.find('[data-testid="jev-gateway-endpoint"]')
      .element as HTMLInputElement;
    expect(endpoint.value).toBe("http://gpu-gate:8090/v1/jev/alpha/decisions");
    expect(wrapper.find('[data-testid="jev-direct-endpoint"]').exists()).toBe(
      false,
    );
  });

  it("uses a localized JEV queue label without exposing its job type", () => {
    const status = aiStatus();
    status.taskQueues.push(taskQueue({ jobType: "tag_relation_jev" }));
    const wrapper = mountSection("overview", status);

    expect(wrapper.text()).toContain("标签关联评分");
    expect(wrapper.text()).not.toContain("tag_relation_jev");
  });
});
