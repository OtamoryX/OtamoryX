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

export interface UpdateProgressRequest {
  currentPage: number;
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
  apiKey?: string;
  apiKeyConfigured: boolean;
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
}

export interface AIAutoTaggingSettings {
  enabled: boolean;
  autoApplyThreshold: number;
}

export interface AISettings {
  connection: AIConnectionSettings;
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

export interface AITitleTranslationBackfillResponse {
  readonly queued: number;
  readonly skipped?: number;
}

export interface AIStatus {
  queueSize: number;
  processingCount: number;
  completedToday: number;
  failedToday: number;
  averageProcessingTime: number;
  activeModels: string[];
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
