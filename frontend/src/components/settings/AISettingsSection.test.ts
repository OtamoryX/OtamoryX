// @vitest-environment jsdom

import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { defineComponent, h } from "vue";
import AISettingsSection from "@/components/settings/AISettingsSection.vue";
import type { AISettings, AIStatus, AITaskQueueStatus } from "@/types/api";

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
      multiUserExperimentEnabled: false,
      analysisRefreshAfterDays: 1,
      tagRelation: {
        enabled: false,
        profileId: "auto",
        transport: "openrouterAlphaDecisions",
        endpoint: "https://openrouter.ai/api/alpha/decisions",
        model: "~typesafe/jev-latest",
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

const mountSection = (section: "overview" | "tasks" = "overview") =>
  mount(AISettingsSection, {
    props: {
      section,
      aiSettings: aiSettings(),
      aiStatus: aiStatus(),
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
      stubs: {
        GlassCard: GlassCardStub,
        GlassButton: GlassButtonStub,
        TaskExecutionSettings: TaskExecutionSettingsStub,
      },
    },
  });

describe("AISettingsSection task queue controls", () => {
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
    expect(text).not.toContain("内容理解高级配置");
    expect(text).not.toContain("内容理解更新间隔");
    expect(text).not.toContain("批量分析并打标签");
  });
});
