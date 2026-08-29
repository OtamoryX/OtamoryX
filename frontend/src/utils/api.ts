import axios from "axios";
import type {
  Archive,
  PaginatedResponse,
  SearchParams,
  HealthResponse,
  SystemSettings,
  ScanSettings,
  Tag,
  ReadingProgress,
  UpdateProgressRequest,
  BatchProgressRequest,
  BatchProgressResponse,
  BehaviorEventRequest,
  BehaviorEventResponse,
  TrashEntry,
  AuthResponse,
  LoginRequest,
  CreateUserRequest,
  SystemStatus,
  InitializeSystemRequest,
  Category,
  DynamicCategory,
  CreateCategoryRequest,
  CreateDynamicCategoryRequest,
  UpdateCategoryRequest,
  AddArchivesToCategoryRequest,
  CategoryDeletePreview,
  CategoryBatchDeleteResult,
  User,
  Plugin,
  PluginDetail,
  PluginConfigSchemaResponse,
  PluginExecuteRequest,
  PluginExecuteResponse,
  EhentaiCandidateSearchResponse,
  NhentaiCandidateSearchResponse,
  PluginExecutionsQuery,
  PluginExecutionListResponse,
  AISettings,
  AITitleTranslationPreviewResponse,
  AITitleDisplayPreference,
  AIStatus,
  AITestConnectionResponse,
  AITitleTranslationBackfillResponse,
  AITitleTranslationRetryResponse,
  AITaggingBackfillResponse,
  AITaggingBackfillRequest,
  AITagLocalizationBackfillResponse,
  PendingAITagSuggestionPage,
  ReviewAITagSuggestionRequest,
  ReviewAITagSuggestionResponse,
  UndoAITaggingRunResponse,
  OcrOperationResponse,
  OcrSettings,
  DirectoryListResponse,
  CollectionSummary,
  CollectionDetail,
  CollectionReviewItem,
  CollectionRebuildResponse,
  CollectionRebuildPreview,
  VersionCleanupResponse,
  VersionGroup,
  RandomRecommendationSession,
  RandomRecommendationMetrics,
  PreferenceRule,
} from "@/types/api";
import type {
  CacheStatusResponse,
  ConfigureCacheRequest,
  ConfigureCacheResponse,
  ScanSettingsResponse,
  TriggerScanResponse,
} from "@/types/settings";

const api = axios.create({
  baseURL: "/api/v1",
  timeout: 10000,
  paramsSerializer: {
    serialize: (params) => {
      const parts: string[] = [];
      for (const [key, value] of Object.entries(params)) {
        if (value == null) continue;
        if (Array.isArray(value)) {
          // 数组用逗号拼接为单个值，后端用 deserialize_comma_separated 解析
          if (value.length > 0) {
            parts.push(
              `${encodeURIComponent(key)}=${encodeURIComponent(value.join(","))}`,
            );
          }
        } else {
          parts.push(`${encodeURIComponent(key)}=${encodeURIComponent(value)}`);
        }
      }
      return parts.join("&");
    },
  },
});

// 请求拦截器 - 添加认证头
api.interceptors.request.use((config) => {
  const apiKey = localStorage.getItem("apiKey");
  if (apiKey) {
    config.headers.Authorization = `Bearer ${apiKey}`;
  }
  return config;
});

// 响应拦截器 - 处理认证错误
api.interceptors.response.use(
  (response) => response,
  (error) => {
    if (error.response?.status === 401) {
      localStorage.removeItem("apiKey");

      // 认证过期时保留当前页面，完整刷新后由登录页恢复目标路由。
      // 登录接口本身失败时不要再次重定向，避免产生跳转循环。
      if (window.location.pathname !== "/login") {
        const currentRoute =
          window.location.pathname + window.location.search + window.location.hash;
        window.location.href = `/login?redirect=${encodeURIComponent(currentRoute)}`;
      }
    }
    return Promise.reject(error);
  },
);

// 健康检查
export const getHealth = async (): Promise<HealthResponse> => {
  const response = await api.get("/health");
  return response.data;
};

// 认证相关
export const login = async (
  credentials: LoginRequest,
): Promise<AuthResponse> => {
  const response = await api.post("/auth/login", credentials);
  return response.data;
};

export const register = async (
  userData: CreateUserRequest,
): Promise<AuthResponse> => {
  const response = await api.post("/auth/register", userData);
  return response.data;
};

export const logout = async (): Promise<void> => {
  await api.post("/auth/logout");
};

// 获取单个漫画详情
export const getArchive = async (id: string): Promise<Archive> => {
  const response = await api.get(`/archives/${id}`);
  return response.data;
};

export const getCollections = async (
  params?: SearchParams & { categoryId?: string | null },
): Promise<CollectionSummary[]> => {
  const response = await api.get<CollectionSummary[]>("/collections", {
    params,
  });
  return response.data;
};

export const getCollection = async (
  id: string,
  params?: SearchParams & { categoryId?: string | null },
): Promise<CollectionDetail> => {
  const response = await api.get<CollectionDetail>(`/collections/${id}`, {
    params,
  });
  return response.data;
};

export const getCollectionReviews = async (): Promise<
  CollectionReviewItem[]
> => {
  const response = await api.get<CollectionReviewItem[]>(
    "/collections/reviews",
  );
  return response.data;
};

export const applyCollectionReview = async (
  id: string,
  action: "approve" | "reject",
): Promise<void> => {
  await api.post(`/collections/reviews/${id}`, { action });
};

export const rebuildCollections =
  async (): Promise<CollectionRebuildResponse> => {
    const response = await api.post<CollectionRebuildResponse>(
      "/collections/rebuild",
    );
    return response.data;
  };

export const previewCollectionRebuild =
  async (): Promise<CollectionRebuildPreview> => {
    const response = await api.get<CollectionRebuildPreview>(
      "/collections/rebuild/preview",
    );
    return response.data;
  };

export const deleteCollectionWithMembers = async (
  id: string,
): Promise<{ collectionId: string; deletedArchives: number }> => {
  const response = await api.delete<{
    collectionId: string;
    deletedArchives: number;
  }>(`/collections/${id}/with-members`);
  return response.data;
};

export const removeCollectionMember = async (
  archiveId: string,
): Promise<void> => {
  await api.delete(`/collections/members/${archiveId}`);
};

export const updateCollection = async (
  id: string,
  payload: {
    displayTitle?: string;
    subtitle?: string;
    isManualLocked?: boolean;
  },
): Promise<void> => {
  await api.put(`/collections/${id}`, payload);
};

export const getVersionGroups = async (
  params?: SearchParams & { categoryId?: string | null; status?: string },
): Promise<VersionGroup[]> => {
  const response = await api.get<VersionGroup[]>("/version-groups", {
    params,
  });
  return response.data;
};

export const keepAllVersions = async (id: string): Promise<void> => {
  await api.post(`/version-groups/${id}/keep-all`);
};

export const restoreVersionGroup = async (id: string): Promise<void> => {
  await api.delete(`/version-groups/${id}/keep-all`);
};

export const cleanupVersions = async (
  id: string,
  keepArchiveId: string,
  deleteArchiveIds: string[],
): Promise<VersionCleanupResponse> => {
  const response = await api.post<VersionCleanupResponse>(
    `/version-groups/${id}/cleanup`,
    {
      keepArchiveId,
      deleteArchiveIds,
    },
  );
  return response.data;
};

// 获取漫画页面图片
export const getArchivePage = async (
  id: string,
  page: number,
): Promise<string> => {
  const response = await api.get(`/archives/${id}/pages/${page}`, {
    responseType: "blob",
  });
  return URL.createObjectURL(response.data);
};

// 获取漫画缩略图
export const getArchiveThumbnail = async (id: string): Promise<string> => {
  const response = await api.get(`/archives/${id}/thumbnail`, {
    responseType: "blob",
  });
  return URL.createObjectURL(response.data);
};

// 阅读进度
export const getProgress = async (
  archiveId: string,
): Promise<ReadingProgress> => {
  const response = await api.get(`/archives/${archiveId}/progress`);
  return response.data;
};

export const updateProgress = async (
  archiveId: string,
  progress: UpdateProgressRequest,
): Promise<void> => {
  await api.post(`/archives/${archiveId}/progress`, progress);
};

export const recordBehaviorEvent = async (
  request: BehaviorEventRequest,
): Promise<BehaviorEventResponse> => {
  const response = await api.post("/behavior-events", request);
  return response.data;
};

export const listTrashEntries = async (params?: {
  status?: string;
  limit?: number;
}): Promise<TrashEntry[]> => {
  const response = await api.get<TrashEntry[]>("/trash", { params });
  return response.data;
};

export const restoreTrashEntry = async (entryId: string): Promise<TrashEntry> => {
  const response = await api.post<TrashEntry>(`/trash/${entryId}/restore`);
  return response.data;
};

export const restoreTrashOperation = async (operationId: string): Promise<TrashEntry[]> => {
  const response = await api.post<TrashEntry[]>(`/trash/operations/${operationId}/restore`);
  return response.data;
};

export const purgeTrashEntry = async (entryId: string): Promise<void> => {
  await api.post(`/trash/${entryId}/purge`);
};

export const purgeTrashOperation = async (operationId: string): Promise<void> => {
  await api.post(`/trash/operations/${operationId}/purge`);
};

export const getTrashPage = async (entryId: string, page: number): Promise<string> => {
  const response = await api.get(`/trash/${entryId}/pages/${page}`, {
    responseType: "blob",
  });
  return URL.createObjectURL(response.data);
};

// 批量获取多个漫画的阅读进度（返回数组格式）
export const getBatchProgress = async (
  archiveIds: string[],
): Promise<ReadingProgress[]> => {
  if (archiveIds.length === 0) return [];

  try {
    const request: BatchProgressRequest = { archiveIds };
    const response = await api.post<BatchProgressResponse>(
      "/progress/batch",
      request,
    );

    // 将对象格式的响应转换为数组格式
    const progressArray: ReadingProgress[] = Object.values(
      response.data.progress,
    );

    return progressArray;
  } catch (error) {
    console.error("Failed to get batch progress:", error);
    return [];
  }
};

// 批量获取多个漫画的阅读进度（返回映射格式）
export const getBatchProgressMap = async (
  archiveIds: string[],
): Promise<Record<string, ReadingProgress>> => {
  if (archiveIds.length === 0) return {};

  try {
    const request: BatchProgressRequest = { archiveIds };
    const response = await api.post<BatchProgressResponse>(
      "/progress/batch",
      request,
    );

    return response.data.progress;
  } catch (error) {
    console.error("Failed to get batch progress map:", error);
    return {};
  }
};

// 搜索漫画
export const searchArchives = async (
  params: SearchParams,
): Promise<PaginatedResponse<Archive>> => {
  const response = await api.get("/search", { params });
  return response.data;
};

// 获取标签
export const getTags = async (): Promise<Tag[]> => {
  const response = await api.get("/tags");
  return response.data;
};

// 系统设置
export const getSettings = async (): Promise<SystemSettings> => {
  const response = await api.get("/settings");
  return response.data;
};

export const updateSettings = async (
  settings: SystemSettings,
): Promise<void> => {
  await api.put("/settings", settings);
};

// 扫描设置
export const getScanSettings = async (): Promise<ScanSettingsResponse> => {
  const response = await api.get("/settings/scan-settings");
  return response.data;
};

export const updateScanSettings = async (
  scanSettings: ScanSettings,
): Promise<ScanSettingsResponse> => {
  const response = await api.post("/settings/scan-settings", { scanSettings });
  return response.data;
};

// 手动触发扫描
export const triggerScan = async (): Promise<TriggerScanResponse> => {
  const response = await api.post("/settings/scan");
  return response.data;
};

// 系统初始化
export const getSystemStatus = async (): Promise<SystemStatus> => {
  const response = await api.get("/system/status");
  return response.data;
};

export const initializeSystem = async (
  data: InitializeSystemRequest,
): Promise<AuthResponse> => {
  const response = await api.post("/system/initialize", data);
  return response.data;
};

// 分类管理
export const getCategories = async (): Promise<Category[]> => {
  const response = await api.get("/categories");
  return response.data;
};

export const createCategory = async (
  data: CreateCategoryRequest,
): Promise<Category> => {
  const response = await api.post("/categories", data);
  return response.data;
};

export const createDynamicCategory = async (
  data: CreateDynamicCategoryRequest,
): Promise<DynamicCategory> => {
  const response = await api.post("/categories/dynamic", data);
  return response.data;
};

export const updateCategory = async (
  id: string,
  data: UpdateCategoryRequest,
): Promise<void> => {
  await api.put(`/categories/${id}`, data);
};

export const deleteCategory = async (id: string): Promise<void> => {
  await api.delete(`/categories/${id}`);
};

export const getCategoryArchives = async (
  categoryId: string,
  params?: SearchParams,
): Promise<PaginatedResponse<Archive>> => {
  const response = await api.get(`/categories/${categoryId}/archives`, {
    params: {
      pageNumb: params?.pageNumb || 1,
      pageSize: params?.pageSize || 20,
      ...params,
    },
  });
  return response.data;
};

export const addArchivesToCategory = async (
  categoryId: string,
  data: AddArchivesToCategoryRequest,
): Promise<void> => {
  await api.post(`/categories/${categoryId}/archives`, data);
};

export const removeArchivesFromCategory = async (
  categoryId: string,
  data: AddArchivesToCategoryRequest,
): Promise<void> => {
  await api.delete(`/categories/${categoryId}/archives`, { data });
};

// 获取档案所属的分类
export const getArchiveCategories = async (
  archiveId: string,
): Promise<string[]> => {
  const response = await api.get(`/archives/${archiveId}/categories`);
  return response.data;
};

// 用户管理
export const getUsers = async (): Promise<User[]> => {
  const response = await api.get("/users");
  return response.data;
};

export const createUser = async (data: CreateUserRequest): Promise<User> => {
  const response = await api.post("/users", data);
  return response.data;
};

export const getUser = async (id: string): Promise<User> => {
  const response = await api.get(`/users/${id}`);
  return response.data;
};

export const updateUser = async (
  id: string,
  data: Partial<CreateUserRequest>,
): Promise<void> => {
  await api.put(`/users/${id}`, data);
};

export const deleteUser = async (id: string): Promise<void> => {
  await api.delete(`/users/${id}`);
};

// 批量操作
export const batchDeleteArchives = async (
  archiveIds: string[],
): Promise<void> => {
  await api.delete("/archives/batch-delete", { data: { archiveIds } });
};

export const batchDeleteTagArchives = async (tagId: string): Promise<void> => {
  await api.delete(`/tags/${tagId}/archives/batch-delete`);
};

export const batchDeleteCategoryArchives = async (
  categoryId: string,
): Promise<CategoryBatchDeleteResult> => {
  const response = await api.delete(
    `/categories/${categoryId}/archives/batch-delete`,
    {
      timeout: 0,
    },
  );
  return response.data;
};

export const getCategoryDeletePreview = async (
  categoryId: string,
): Promise<CategoryDeletePreview> => {
  const response = await api.get(
    `/categories/${categoryId}/archives/delete-preview`,
  );
  return response.data;
};

export const pruneTags = async (): Promise<void> => {
  await api.delete("/tags/prune");
};

export const pruneCategories = async (): Promise<void> => {
  await api.delete("/categories/prune");
};

// 缓存管理
export const getCacheStatus = async (): Promise<CacheStatusResponse> => {
  const response = await api.get("/cache/status");
  return response.data;
};

export const configureCache = async (
  config: ConfigureCacheRequest,
): Promise<ConfigureCacheResponse> => {
  const response = await api.post("/cache/configure", config);
  return response.data;
};

export type CacheClearScope = "all" | "pages" | "covers";

export interface ClearCacheResponse {
  message?: string;
  success: boolean;
  scope: CacheClearScope;
}

export const clearCache = async (
  scope: CacheClearScope = "all",
): Promise<ClearCacheResponse> => {
  const response = await api.delete("/cache/clear", {
    params: { scope },
  });
  return response.data;
};

// 随机漫画
export const getRandomArchives = async (
  params: {
    count?: number;
    categoryId?: string;
    query?: string;
    tags?: string[];
    minPages?: number;
    maxPages?: number;
    minFileSize?: number;
    maxFileSize?: number;
    createdAfter?: string;
    createdBefore?: string;
  } = {},
): Promise<Archive[]> => {
  const {
    count = 20,
    categoryId,
    query,
    tags,
    minPages,
    maxPages,
    minFileSize,
    maxFileSize,
    createdAfter,
    createdBefore,
  } = params;
  const requestParams: Record<string, any> = { count };
  if (categoryId) requestParams.category_id = categoryId;
  if (query) requestParams.query = query;
  if (tags && tags.length > 0) requestParams.tags = tags;
  if (minPages != null) requestParams.minPages = minPages;
  if (maxPages != null) requestParams.maxPages = maxPages;
  if (minFileSize != null) requestParams.minFileSize = minFileSize;
  if (maxFileSize != null) requestParams.maxFileSize = maxFileSize;
  if (createdAfter) requestParams.createdAfter = createdAfter;
  if (createdBefore) requestParams.createdBefore = createdBefore;
  const response = await api.get("/archives/random", { params: requestParams });
  return response.data;
};

export const getRandomArchiveSession = async (
  params: Parameters<typeof getRandomArchives>[0] = {},
): Promise<RandomRecommendationSession> => {
  const requestParams: Record<string, any> = { count: params.count ?? 20 };
  const filters: Record<string, unknown> = {
    category_id: params.categoryId,
    query: params.query,
    tags: params.tags,
    minPages: params.minPages,
    maxPages: params.maxPages,
    minFileSize: params.minFileSize,
    maxFileSize: params.maxFileSize,
    createdAfter: params.createdAfter,
    createdBefore: params.createdBefore,
  };
  Object.entries(filters).forEach(([key, value]) => {
    if (value != null && value !== "" && (!Array.isArray(value) || value.length > 0)) requestParams[key] = value;
  });
  const response = await api.get<RandomRecommendationSession>("/archives/random/session", { params: requestParams });
  return response.data;
};

export const getRandomRecommendationMetrics = async (
  days: 7 | 30 | 90 = 30,
): Promise<RandomRecommendationMetrics> => {
  const response = await api.get<RandomRecommendationMetrics>(
    "/curation/random-metrics",
    { params: { days } },
  );
  return response.data;
};

export const getPreferenceRules = async (): Promise<PreferenceRule[]> => {
  const response = await api.get<PreferenceRule[]>("/preference-rules");
  return response.data;
};

export const setPreferenceRuleEnabled = async (
  ruleId: string,
  enabled: boolean,
): Promise<void> => {
  await api.post(`/preference-rules/${encodeURIComponent(ruleId)}/${enabled ? "enable" : "disable"}`);
};

// 插件管理
export const getPlugins = async (): Promise<Plugin[]> => {
  const response = await api.get<Plugin[]>("/plugins");
  return response.data;
};

export const installPlugin = async (pluginData: FormData): Promise<Plugin> => {
  const response = await api.post<Plugin>(
    "/plugins/install",
    pluginData,
    {
      headers: { "Content-Type": "multipart/form-data" },
    },
  );
  return response.data;
};

export const togglePlugin = async (id: string): Promise<void> => {
  await api.put(`/plugins/${id}/toggle`);
};

export const configurePlugin = async (
  id: string,
  config: any,
): Promise<void> => {
  await api.put(`/plugins/${id}/config`, { config });
};

export const uninstallPlugin = async (id: string): Promise<void> => {
  await api.delete(`/plugins/${id}`);
};

export const getPlugin = async (id: string): Promise<PluginDetail> => {
  const response = await api.get<PluginDetail>(`/plugins/${id}`);
  return response.data;
};

export const getPluginConfigSchema = async (
  id: string,
): Promise<PluginConfigSchemaResponse> => {
  const response = await api.get<PluginConfigSchemaResponse>(
    `/plugins/${id}/config/schema`,
  );
  return response.data;
};

export const executePlugin = async (
  id: string,
  payload: PluginExecuteRequest = {},
): Promise<PluginExecuteResponse> => {
  const response = await api.post<PluginExecuteResponse>(
    `/plugins/${id}/execute`,
    payload,
  );
  return response.data;
};

export const executePluginForArchive = async (
  id: string,
  archiveId: string,
  payload: PluginExecuteRequest = {},
): Promise<PluginExecuteResponse> => {
  const response = await api.post<PluginExecuteResponse>(
    `/plugins/${id}/execute/${archiveId}`,
    payload,
  );
  return response.data;
};

export const searchEhentaiCandidates = async (
  archiveId: string,
): Promise<EhentaiCandidateSearchResponse> => {
  const response = await api.get<EhentaiCandidateSearchResponse>(
    `/plugins/ehentai-metadata/candidates/${archiveId}`,
  );
  return response.data;
};

export const searchNhentaiCandidates = async (
  archiveId: string,
): Promise<NhentaiCandidateSearchResponse> => {
  const response = await api.get<NhentaiCandidateSearchResponse>(
    `/plugins/nhentai-metadata/candidates/${archiveId}`,
  );
  return response.data;
};

export const getPluginExecutions = async (
  id: string,
  query: PluginExecutionsQuery = {},
): Promise<PluginExecutionListResponse> => {
  const response = await api.get<PluginExecutionListResponse>(
    `/plugins/${id}/executions`,
    {
      params: query,
    },
  );
  return response.data;
};

export const getAllPluginExecutions = async (
  query: PluginExecutionsQuery = {},
): Promise<PluginExecutionListResponse> => {
  const response = await api.get<PluginExecutionListResponse>(
    "/plugin-executions",
    {
      params: query,
    },
  );
  return response.data;
};

const serializeAISettings = (settings: AISettings): AISettings => {
  const profiles = settings.profiles.map((profile) => {
    const { apiKey: rawApiKey, ...connection } = profile.connection;
    const apiKey = rawApiKey?.trim();
    return {
      ...profile,
      name: profile.name.trim(),
      connection: {
        ...connection,
        baseUrl: connection.baseUrl.trim(),
        model: connection.model.trim(),
        ...(apiKey ? { apiKey } : {}),
      },
    };
  });
  const activeProfile =
    profiles.find((profile) => profile.id === settings.activeProfileId) ??
    profiles[0];

  return {
    ...settings,
    profiles,
    activeProfileId: activeProfile?.id ?? settings.activeProfileId,
    connection: activeProfile?.connection ?? settings.connection,
  };
};

// AI 配置与任务
export const getAISettings = async (): Promise<AISettings> => {
  const response = await api.get<AISettings>("/settings/ai");
  return response.data;
};

export const getTitleDisplayPreference = async (): Promise<AITitleDisplayPreference> => {
  const response = await api.get<AITitleDisplayPreference>(
    "/settings/title-display",
  );
  return response.data;
};

export const updateAISettings = async (settings: AISettings): Promise<void> => {
  await api.put("/settings/ai", serializeAISettings(settings));
};

export const testAIConnection = async (
  settings: AISettings,
): Promise<AITestConnectionResponse> => {
  const response = await api.post<AITestConnectionResponse>(
    "/settings/ai/test-connection",
    serializeAISettings(settings),
  );
  return response.data;
};

export const previewAITitleTranslation = async (
  title: string,
  targetLanguage: string,
  settings: AISettings,
): Promise<AITitleTranslationPreviewResponse> => {
  const response = await api.post<AITitleTranslationPreviewResponse>(
    "/settings/ai/title-translation/preview",
    {
      title,
      targetLanguage,
      settings: serializeAISettings(settings),
    },
  );
  return response.data;
};

export const getOcrSettings = async (): Promise<OcrSettings> => {
  const response = await api.get<OcrSettings>("/settings/ocr");
  return response.data;
};

export const updateOcrSettings = async (
  settings: Pick<OcrSettings, "enabled" | "image" | "failurePolicy">,
): Promise<void> => {
  await api.put("/settings/ocr", settings);
};

export const downloadOcrModel = async (
  modelId: string,
): Promise<OcrOperationResponse> => {
  const response = await api.post<OcrOperationResponse>(
    `/settings/ocr/models/${encodeURIComponent(modelId)}/download`,
  );
  return response.data;
};

export const activateOcrModel = async (
  modelId: string,
): Promise<OcrOperationResponse> => {
  const response = await api.post<OcrOperationResponse>(
    `/settings/ocr/models/${encodeURIComponent(modelId)}/activate`,
  );
  return response.data;
};

export const backfillAITitleTranslations = async (
  force = false,
  repair = false,
): Promise<AITitleTranslationBackfillResponse> => {
  const response = await api.post<AITitleTranslationBackfillResponse>(
    "/ai/title-translations/backfill",
    undefined,
    { params: { force, repair } },
  );
  return response.data;
};

export const retryArchiveTitleTranslation = async (
  archiveId: string,
): Promise<AITitleTranslationRetryResponse> => {
  const response = await api.post<AITitleTranslationRetryResponse>(
    `/archives/${archiveId}`,
  );
  return response.data;
};

export const getPendingAITagSuggestions = async (params?: {
  archiveId?: string;
  limit?: number;
  page?: number;
  includeAutoApplied?: boolean;
}): Promise<PendingAITagSuggestionPage> => {
  const response = await api.get<PendingAITagSuggestionPage>(
    "/ai/tags/suggestions",
    {
      params: {
        archiveId: params?.archiveId,
        limit: params?.limit,
        page: params?.page,
        includeAutoApplied: params?.includeAutoApplied,
      },
    },
  );
  return response.data;
};

export const reviewAITagSuggestion = async (
  suggestionId: string,
  payload: ReviewAITagSuggestionRequest,
): Promise<ReviewAITagSuggestionResponse> => {
  const response = await api.post<ReviewAITagSuggestionResponse>(
    `/ai/tags/suggestions/${encodeURIComponent(suggestionId)}/review`,
    payload,
  );
  return response.data;
};

export const undoAITaggingRun = async (
  runId: string,
): Promise<UndoAITaggingRunResponse> => {
  const response = await api.post<UndoAITaggingRunResponse>(
    `/ai/tags/runs/${encodeURIComponent(runId)}/undo`,
  );
  return response.data;
};

export const backfillAITagging = async (
  payload: AITaggingBackfillRequest = {},
): Promise<AITaggingBackfillResponse> => {
  const response = await api.post<AITaggingBackfillResponse>(
    "/ai/tags/backfill",
    payload,
  );
  return response.data;
};

export const backfillAITagLocalizations = async (): Promise<AITagLocalizationBackfillResponse> => {
  const response = await api.post<AITagLocalizationBackfillResponse>(
    "/ai/tags/localizations/backfill",
  );
  return response.data;
};

export const getAIStatus = async (): Promise<AIStatus> => {
  const response = await api.get("/ai/status");
  return response.data;
};

export const getAITasks = async (params?: {
  status?: string;
  jobType?: string;
  failureCode?: string;
  cursor?: string;
  limit?: number;
  includePayload?: boolean;
}): Promise<import("@/types/api").AITaskDiagnosticPage> => {
  const response = await api.get("/ai/tasks", { params });
  return response.data;
};

export const getAITask = async (
  taskId: string,
): Promise<import("@/types/api").AITaskDiagnostic> => {
  const response = await api.get(`/ai/tasks/${encodeURIComponent(taskId)}`);
  return response.data;
};

export const getAIFailureSummary = async (): Promise<
  import("@/types/api").AIFailureSummary
> => {
  const response = await api.get("/ai/failures/summary");
  return response.data;
};

export const controlAI = async (action: "pause" | "resume"): Promise<void> => {
  await api.put("/ai/control", { action });
};

export const controlAITaskQueue = async (
  jobType: string,
  action: "pause" | "resume" | "forceContinue",
): Promise<void> => {
  await api.put(`/ai/queues/${encodeURIComponent(jobType)}/control`, { action });
};

export const forceContinueAIModel = async (profileId: string): Promise<void> => {
  await api.put(`/ai/models/${encodeURIComponent(profileId)}/force-continue`);
};

// 标签管理
export const createTag = async (
  name: string,
  namespace: string,
): Promise<Tag> => {
  const response = await api.post("/tags", { name, namespace });
  return response.data;
};

export const deleteTag = async (id: string): Promise<void> => {
  await api.delete(`/tags/${id}`);
};

export const addTagToArchive = async (
  archiveId: string,
  tagId: string,
): Promise<void> => {
  await api.post(`/archives/${archiveId}/tags`, { tag_id: tagId });
};

export const removeTagFromArchive = async (
  archiveId: string,
  tagId: string,
): Promise<void> => {
  await api.delete(`/archives/${archiveId}/tags/${tagId}`);
};

export interface ArchiveDeleteContext {
  recommendationSessionId?: string;
  recommendationPosition?: number;
}

export const deleteArchive = async (
  archiveId: string,
  context: ArchiveDeleteContext = {},
): Promise<void> => {
  await api.delete(`/archives/${archiveId}`, { params: context });
};

// 文件系统浏览
export const getDirectories = async (
  path?: string,
): Promise<DirectoryListResponse> => {
  const params = path ? { path } : {};
  const response = await api.get("/filesystem/directories", { params });
  return response.data;
};
