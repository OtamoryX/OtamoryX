export interface Archive {
  readonly id: string;
  readonly title: string;
  readonly subtitle?: string;
  readonly subtitleLanguage?: string;
  readonly path: string;
  readonly pageCount: number;
  readonly fileSize: number;
  readonly hash: string;
  readonly createdAt: string;
  readonly updatedAt: string;
  readonly tags: Tag[];
}

export interface CollectionSummary {
  readonly id: string;
  readonly displayTitle: string;
  readonly subtitle?: string;
  readonly coverArchiveId?: string;
  readonly status: "auto" | "needs_review" | "manual" | string;
  readonly isManualLocked: boolean;
  readonly memberCount: number;
  readonly contentCount: number;
  readonly variantGroupCount: number;
  readonly variantCount: number;
  readonly reviewCount: number;
  readonly matchedMemberCount: number;
  readonly progressPercentage?: number;
}

export interface CollectionMember {
  readonly archive: Archive;
  readonly matchesFilter: boolean;
  readonly unitType: string;
  readonly volumeNumber?: string;
  readonly chapterNumber?: string;
  readonly issueNumber?: string;
  readonly rawNumber?: string;
  readonly sortKey: number;
  readonly variantGroupKey?: string;
  readonly confidence: number;
  readonly membershipSource: string;
  readonly isManualLocked: boolean;
  readonly review?: {
    readonly id: string;
    readonly reason: string;
    readonly evidence: Record<string, unknown>;
  } | null;
}

export interface CollectionDetail {
  readonly collection: CollectionSummary;
  readonly members: CollectionMember[];
}

export interface VersionCandidate {
  readonly archive: Archive;
  readonly matchesFilter: boolean;
  readonly confidence: number;
  readonly isRecommended: boolean;
  readonly recommendationReasons: string[];
}

export interface VersionGroup {
  readonly id: string;
  readonly groupKey: string;
  readonly displayTitle: string;
  readonly subtitle?: string;
  readonly collectionId?: string;
  readonly collectionTitle?: string;
  readonly unitLabel: string;
  readonly confidence: number;
  readonly status: "active" | "keep_all" | string;
  readonly recommendedArchiveId?: string;
  readonly reclaimableSize: number;
  readonly matchedMemberCount: number;
  readonly members: VersionCandidate[];
}

export interface VersionCleanupResponse {
  readonly keptArchiveId: string;
  readonly deleted: number;
  readonly failedArchiveIds: string[];
}

export interface CollectionReviewItem {
  readonly id: string;
  readonly archive: Archive;
  readonly collection: CollectionSummary;
  readonly reason: string;
  readonly evidence: Record<string, unknown>;
  readonly status: string;
}

export interface CollectionRebuildResponse {
  readonly parsedArchives: number;
  readonly createdCollections: number;
  readonly groupedArchives: number;
  readonly pendingReviews: number;
}

export interface CollectionRebuildPreviewItem {
  readonly displayTitle: string;
  readonly memberCount: number;
  readonly status: "auto" | "needs_review" | "versions" | string;
  readonly reason: string;
}

export interface CollectionRebuildPreview {
  readonly parsedArchives: number;
  readonly collectionCandidates: CollectionRebuildPreviewItem[];
  readonly versionCandidates: CollectionRebuildPreviewItem[];
  readonly pendingReviewCount: number;
}

export interface Tag {
  readonly id: string;
  readonly name: string;
  readonly namespace: string;
}

export interface PaginatedResponse<T> {
  readonly data: T[];
  readonly pageNumb: number;
  readonly pageSize: number;
  readonly total: number;
  readonly hasNext: boolean;
}

export interface SearchParams {
  query?: string;
  tags?: string[];
  minPages?: number;
  maxPages?: number;
  minFileSize?: number;
  maxFileSize?: number;
  createdAfter?: string; // 添加时间过滤 - 创建时间之后
  createdBefore?: string; // 添加时间过滤 - 创建时间之前
  lastReadAfter?: string; // 上次阅读时间过滤 - 阅读时间之后
  lastReadBefore?: string; // 上次阅读时间过滤 - 阅读时间之前
  sortBy?: string;
  sortOrder?: string;
  pageNumb?: number;
  pageSize?: number;
}

export interface HealthResponse {
  status: string;
  version: string;
  timestamp: string;
}

export interface User {
  readonly id: string;
  readonly username: string;
  readonly email?: string;
  readonly role: "admin" | "user";
  readonly createdAt: string;
  readonly updatedAt: string;
}

export interface AuthResponse {
  readonly token: string;
  readonly user: User;
}

export interface LoginRequest {
  username: string;
  password: string;
}

export interface CreateUserRequest {
  username: string;
  email?: string;
  password: string;
  role?: "admin" | "user";
}

export interface UpdateUserRequest {
  username?: string;
  email?: string;
  password?: string;
  role?: "admin" | "user";
}

export interface UserPathsRequest {
  paths: string[];
}

export interface BatchDeleteUsersRequest {
  user_ids: string[];
}

export interface ReadingProgress {
  readonly id: number;
  readonly archiveId: string;
  readonly userId: string;
  readonly currentPage: number;
  readonly totalPages: number;
  readonly progressPercentage: number;
  readonly lastReadAt: string;
}

export interface BehaviorEventRequest {
  archiveId?: string;
  eventType: string;
  eventKey?: string;
  page?: number;
  metadata?: Record<string, unknown>;
  occurredAt?: string;
}

export interface BehaviorEvent {
  id: string;
  userId: string;
  archiveId?: string | null;
  eventType: string;
  eventKey?: string | null;
  page?: number | null;
  metadataJson: string;
  occurredAt: string;
  createdAt: string;
}

export interface BehaviorEventResponse {
  event: BehaviorEvent;
  duplicate: boolean;
}

export interface RandomRecommendationSession {
  sessionId: string;
  archives: Archive[];
}

export interface RandomRecommendationMetric {
  exposed: number;
  opened: number;
  effectiveReads: number;
  quickExits: number;
  manualDeletes: number;
  effectiveReadRate: number;
  manualDeletesPer100Opens: number;
}

export interface RandomRecommendationTopicCoverage {
  candidateTopicCount: number;
  exposedTopicCount: number;
  explorationTopicCount: number;
  exposureCoverage: number;
  explorationCoverage: number;
}

export interface RandomRecommendationAlgorithmMetrics {
  algorithmVariant: string;
  overall: RandomRecommendationMetric;
  preferred: RandomRecommendationMetric;
  exploration: RandomRecommendationMetric;
  topics: RandomRecommendationTopicCoverage;
}

export interface RandomRecommendationMetrics {
  days: number;
  overall: RandomRecommendationMetric;
  preferred: RandomRecommendationMetric;
  exploration: RandomRecommendationMetric;
  topics: RandomRecommendationTopicCoverage;
  byAlgorithm: RandomRecommendationAlgorithmMetrics[];
}

export interface PreferenceRule {
  id: string;
  userId: string;
  name: string;
  ruleVersion: string;
  conditions: Record<string, unknown>;
  exceptions: Record<string, unknown>;
  action: "keep" | "downrank" | "auto_delete" | string;
  confidenceThreshold: number;
  enabled: boolean;
  ownerRole: string;
  falsePositiveCount: number;
  autoPaused: boolean;
}

export interface TrashEntry {
  readonly id: string;
  readonly userId: string;
  readonly archiveId: string;
  readonly originalPath: string;
  readonly trashPath?: string;
  readonly reason?: string;
  readonly ruleVersion?: string;
  readonly ruleId?: string;
  readonly evaluationId?: string;
  readonly modelConfidence?: number;
  readonly metadataJson: string;
  readonly operationId?: string;
  readonly operationType?: string;
  readonly status: "active" | "restored" | "expired" | string;
  readonly deletedAt: string;
  readonly expiresAt?: string;
  readonly restoredAt?: string;
  readonly cleanupAttempts: number;
  readonly lastCleanupAttemptAt?: string;
  readonly lastCleanupError?: string;
  readonly expiredAt?: string;
}

export interface UpdateProgressRequest {
  currentPage: number;
  readerSessionId?: string;
  recommendationSessionId?: string;
}

export interface BatchProgressRequest {
  archiveIds: string[];
}

export interface BatchProgressResponse {
  progress: Record<string, ReadingProgress>;
}

export interface SystemSettings {
  comicsPath: string;
  supportedFormats: string[];
  maxFileSize: number;
  imageCacheSize: number;
  imageCachePath: string;
  scanOnStartup: boolean;
  scanSettings: ScanSettings;
  imageCacheQuality?: number;
  imageCacheFormat?: string;
}

export interface ScanSettings {
  enabled: boolean;
  recursive: boolean;
  ignoreHidden: boolean;
  realtimeMonitoring: boolean;
}

// 系统初始化相关
export interface SystemStatus {
  initialized: boolean;
  hasAdmin: boolean;
  version: string;
}

export interface InitializeSystemRequest {
  username: string;
  password: string;
  email?: string;
}

// 分类管理相关
export interface Category {
  readonly id: string;
  readonly name: string;
  readonly description?: string;
  readonly isStatic: boolean;
  readonly archiveCount: number;
  readonly createdAt: string;
  readonly updatedAt: string;
}

export interface DynamicCategory {
  readonly id: string;
  readonly name: string;
  readonly description?: string;
  readonly searchParams: string;
  readonly createdAt: string;
  readonly updatedAt: string;
}

export interface CreateCategoryRequest {
  name: string;
  description?: string;
}

export interface CreateDynamicCategoryRequest {
  name: string;
  description?: string;
  searchParams: SearchParams;
}

export interface UpdateCategoryRequest {
  name?: string;
  description?: string;
}

export interface AddArchivesToCategoryRequest {
  archiveIds: string[];
}

export interface CategoryDeletePreview {
  categoryType: "static" | "dynamic";
  matched: number;
}

export interface CategoryBatchDeleteResult {
  categoryType: "static" | "dynamic" | "unknown";
  matched: number;
  deleted: number;
  failed: number;
}

// 插件相关类型
export type JsonValue =
  | string
  | number
  | boolean
  | null
  | { [key: string]: JsonValue }
  | JsonValue[];

export interface Plugin {
  readonly id: string;
  readonly plugin_id?: string;
  readonly name: string;
  readonly version: string;
  readonly plugin_type?: string;
  readonly enabled: boolean;
  readonly description?: string | null;
  readonly author?: string | null;
  readonly config?: JsonValue | null;
  readonly execution_count?: number;
  readonly last_executed_at?: string | null;
  readonly installedAt: string;
  readonly updatedAt: string;
}

export interface PluginDetail extends Plugin {
  readonly plugin_id: string;
  readonly manifest_version: number;
  readonly plugin_api_version: number;
  readonly plugin_type: string;
  readonly icon?: string | null;
  readonly permissions?: JsonValue | null;
  readonly manifest?: JsonValue | null;
  readonly execution_count: number;
}

export interface PluginConfigSchemaResponse {
  readonly plugin_id: string;
  readonly config_schema: JsonValue;
  readonly cooldown?: number | null;
}

export interface PluginExecuteRequest {
  archive_id?: string;
  archive_ids?: string[];
  oneshot_param?: string;
  input?: JsonValue;
  // 兼容已有调用的 camelCase 写法
  archiveId?: string;
  archiveIds?: string[];
  oneshotParam?: string;
}

export interface PluginExecutionDispatchResult {
  readonly plugin_id: string;
  readonly archive_id?: string | null;
  readonly execution_id?: string | null;
  readonly status: string;
  readonly error?: string | null;
}

export interface PluginExecuteResponse {
  readonly plugin_id: string;
  readonly total: number;
  readonly accepted: number;
  readonly failed: number;
  readonly results: PluginExecutionDispatchResult[];
}

export interface EhentaiCandidate {
  readonly galleryId: string;
  readonly token: string;
  readonly sourceUrl: string;
  readonly title: string;
}

export interface EhentaiCandidateSearchResponse {
  readonly archiveId: string;
  readonly candidates: EhentaiCandidate[];
}

export interface NhentaiCandidate {
  readonly galleryId: string;
  readonly sourceUrl: string;
  readonly title: string;
}

export interface NhentaiCandidateSearchResponse {
  readonly archiveId: string;
  readonly candidates: NhentaiCandidate[];
}

export interface PluginExecutionRecord {
  readonly execution_id: string;
  readonly plugin_id: string;
  readonly archive_id?: string | null;
  readonly execution_type: string;
  readonly status: string;
  readonly input_summary?: string | null;
  readonly output_summary?: string | null;
  readonly error_message?: string | null;
  readonly duration_ms?: number | null;
  readonly started_at: string;
  readonly completed_at?: string | null;
}

export interface PluginExecutionsQuery {
  limit?: number;
  offset?: number;
  status?: string;
  archive_id?: string;
  plugin_id?: string;
  // 兼容调用方 camelCase 写法
  archiveId?: string;
  pluginId?: string;
}

export interface PluginExecutionListResponse {
  readonly total: number;
  readonly limit: number;
  readonly offset: number;
  readonly items: PluginExecutionRecord[];
}

// AI 配置。敏感的 apiKey 仅可写入；读取设置时服务端只返回 apiKeyConfigured。
export interface AIConnectionSettings {
  provider: "openaiCompatible";
  baseUrl: string;
  model: string;
  visionCapable: boolean;
  authMode: "bearer" | "none";
  apiKey?: string;
  apiKeyConfigured: boolean;
}

export interface AIConnectionProfile {
  id: string;
  name: string;
  enabled: boolean;
  connection: AIConnectionSettings;
}

export interface AIExecutionSettings {
  maxConcurrentTasks: number;
  timeoutSeconds: number;
  maxRetries: number;
}

export interface AITitleTranslationSettings {
  enabled: boolean;
  targetLanguage: string;
  skipIfTargetLanguage: boolean;
  retranslateOnTitleChange: boolean;
  displayTranslatedTitle: boolean;
}

export interface AIAutoTaggingSettings {
  enabled: boolean;
  autoApplyThreshold: number;
  /** Retain suggestions for review, or automatically apply candidates at the threshold. */
  mode: "suggestions" | "autoApplyReliable";
  /** Enqueue the content workflow for each newly discovered archive. */
  autoProcessNewArchives: boolean;
}

export interface AISettings {
  connection: AIConnectionSettings;
  profiles: AIConnectionProfile[];
  activeProfileId: string;
  execution: AIExecutionSettings;
  features: {
    titleTranslation: AITitleTranslationSettings;
    autoTagging: AIAutoTaggingSettings;
  };
}

export interface AITestConnectionResponse {
  readonly success: boolean;
  readonly message?: string;
}

export interface AITitleDisplayPreference {
  displayTranslatedTitle: boolean;
}

export interface AITitleTranslationBackfillResponse {
  readonly started: boolean;
}

export interface AITitleTranslationRetryResponse {
  readonly queued: boolean;
}

export interface AITagSuggestion {
  readonly id: string;
  readonly runId: string;
  readonly archiveId: string;
  readonly name: string;
  readonly namespace: string;
  readonly confidence: number;
  readonly evidence: unknown;
  readonly provenance: unknown;
  readonly status: string;
  readonly reviewedAt: string | null;
  readonly reviewedBy: string | null;
  readonly editedTagId: string | null;
  readonly createdAt: string;
  readonly updatedAt: string;
}

export interface PendingAITagSuggestion extends AITagSuggestion {
  readonly archiveTitle: string;
}

export type AITagSuggestionReviewAction = "approve" | "reject" | "edit";

export interface ReviewAITagSuggestionRequest {
  action: AITagSuggestionReviewAction;
  editedName?: string;
  editedNamespace?: string;
}

export interface ReviewAITagSuggestionResponse {
  readonly suggestion: AITagSuggestion;
  readonly application: {
    readonly applicationId: string;
    readonly tagId: string;
    readonly name: string;
    readonly namespace: string;
    readonly createdArchiveTag: boolean;
  } | null;
}

export interface AITaggingBackfillResponse {
  readonly queued: number;
  readonly skipped: number;
  readonly attempted: number;
  readonly failed: number;
  readonly hasMore: boolean;
  readonly nextCursor: string | null;
}

export interface AITaggingBackfillRequest {
  limit?: number;
  cursor?: string;
  archiveIds?: string[];
}

export interface UndoAITaggingRunResponse {
  readonly runId: string;
  readonly applicationsUndone: number;
  readonly archiveTagsRemoved: number;
  readonly archiveTagsPreserved: number;
}

export interface AIStatus {
  queueSize: number;
  processingCount: number;
  completedToday: number;
  failedToday: number;
  languageDetectionPending: number;
  retryScheduled: number;
  unresolvedFailureCount: number;
  providerBlockedUntil: string | null;
  averageProcessingTime: number;
  activeModels: string[];
  /** Pending and processing work grouped by the shared queue executor lane. */
  queueByLane: Record<string, number>;
}

export interface OcrModelStatus {
  id: string;
  name: string;
  language: string;
  version: string;
  downloaded: boolean;
  active: boolean;
  loading: boolean;
  error: string | null;
}

export interface OcrSettings {
  enabled: boolean;
  activeModelId: string;
  cachePath: string;
  models: OcrModelStatus[];
}

export interface OcrOperationResponse {
  accepted: boolean;
  message: string;
}

export interface AIGeneratedTag {
  readonly id: string;
  readonly archiveId: string;
  readonly tagId: string;
  readonly confidence: number;
  readonly approved: boolean | null;
  readonly createdAt: string;
  readonly reviewedAt?: string;
  readonly reviewedBy?: string;
}

// 搜索建议类型
export interface SearchSuggestion {
  type: "tag" | "title" | "author" | "series";
  value: string;
  count: number;
}

// 统计相关类型
export interface LibraryStats {
  totalArchives: number;
  totalCategories: number;
  totalTags: number;
  totalFileSize: number;
  averagePageCount: number;
  mostUsedTags: Tag[];
  recentlyAdded: Archive[];
}

// 阅读历史
export interface ReadingHistory {
  readonly id: string;
  readonly archiveId: string;
  readonly archive: Archive;
  readonly lastReadAt: string;
  readonly progressPercentage: number;
}

// 收藏夹
export interface Favorite {
  readonly id: string;
  readonly archiveId: string;
  readonly archive: Archive;
  readonly createdAt: string;
}

// 通知类型
export interface Notification {
  readonly id: string;
  readonly type: "info" | "success" | "warning" | "error";
  readonly title: string;
  readonly message: string;
  readonly read: boolean;
  readonly createdAt: string;
}

// 文件系统浏览相关类型
export interface DirectoryInfo {
  readonly name: string;
  readonly path: string;
  readonly is_accessible: boolean;
}

export interface DirectoryListResponse {
  readonly current_path: string;
  readonly parent_path: string | null;
  readonly directories: DirectoryInfo[];
}
