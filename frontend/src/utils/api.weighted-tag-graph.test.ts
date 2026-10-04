import { afterEach, describe, expect, it, vi } from "vitest";
import { getWeightedTagGraphStatus, updateAISettings } from "@/utils/api";
import type { AISettings, WeightedTagGraphStatus } from "@/types/api";

const apiClient = vi.hoisted(() => ({
  get: vi.fn(),
  put: vi.fn(),
  interceptors: {
    request: { use: vi.fn() },
    response: { use: vi.fn() },
  },
}));

vi.mock("axios", () => ({
  default: {
    create: vi.fn(() => apiClient),
  },
}));

const aiSettings = (tagGraphEnabled: boolean, apiKey = ""): AISettings =>
  ({
    settingsVersion: 1,
    connection: {},
    profiles: [],
    activeProfileId: "",
    execution: {},
    features: {
      titleTranslation: {},
      tagLocalization: {},
      contentUnderstanding: {},
      autoTagging: {},
      recommendations: {
        tagGraphEnabled,
        multiUserExperimentEnabled: false,
        analysisRefreshAfterDays: 180,
        tagRelation: {
          transport: "openrouterAlphaDecisions",
          endpoint: "https://example.invalid/decisions",
          gpuGateEndpoint: "http://example.invalid/jev",
          ollamaEndpoint: "http://example.invalid/systemone",
          model: "sample-jev-model",
          apiKey,
          apiKeyConfigured: apiKey.length > 0,
          batchSize: 4,
          maxPairsPerTrigger: 100,
          candidateAlgorithmVersion: "sample-candidates-v1",
          protocolVersion: "sample-protocol-v1",
          promptVersion: "sample-prompt-v1",
          schemaVersion: "sample-schema-v1",
          minConfidence: 0.7,
          execution: {},
        },
      },
    },
  }) as unknown as AISettings;

const graphStatus = (): WeightedTagGraphStatus => ({
  enabled: true,
  configured: false,
  state: "unconfigured",
  activeRelationCount: 0,
  queuedTaskCount: 0,
  processingTaskCount: 0,
  retryWaitingTaskCount: 0,
  paused: false,
  nextRetryAt: null,
  lastError: null,
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("weighted tag graph settings API", () => {
  it("reads status from the dedicated admin endpoint", async () => {
    const status = graphStatus();
    apiClient.get.mockResolvedValue({ data: status });

    await expect(getWeightedTagGraphStatus()).resolves.toEqual(status);
    expect(apiClient.get).toHaveBeenCalledWith(
      "/admin/recommendations/weighted-tag-graph/status",
    );
  });

  it("allows an unconfigured default-on setting to save through AI settings", async () => {
    const settings = aiSettings(true);
    apiClient.put.mockResolvedValue({});

    await updateAISettings(settings);

    expect(apiClient.put).toHaveBeenCalledTimes(1);
    expect(apiClient.put).toHaveBeenCalledWith(
      "/settings/ai",
      expect.objectContaining({
        features: expect.objectContaining({
          recommendations: expect.objectContaining({ tagGraphEnabled: true }),
        }),
      }),
    );
  });

  it("preserves explicit OFF when JEV model and key are changed", async () => {
    const settings = aiSettings(false, "sample-key-old");
    settings.features.recommendations.tagRelation.model = "sample-jev-model-v2";
    settings.features.recommendations.tagRelation.apiKey = "sample-key-new";
    apiClient.put.mockResolvedValue({});

    await updateAISettings(settings);

    expect(apiClient.put).toHaveBeenCalledTimes(1);
    const [, payload] = apiClient.put.mock.calls[0];
    expect(payload.features.recommendations.tagGraphEnabled).toBe(false);
    expect(payload.features.recommendations.tagRelation.model).toBe(
      "sample-jev-model-v2",
    );
    expect(payload.features.recommendations.tagRelation.apiKey).toBe(
      "sample-key-new",
    );
    expect(payload.features.recommendations.tagRelation).not.toHaveProperty(
      "enabled",
    );
    expect(apiClient.put).not.toHaveBeenCalledWith(
      "/admin/recommendations/weighted-tag-graph/policy",
      expect.anything(),
    );
  });
});
