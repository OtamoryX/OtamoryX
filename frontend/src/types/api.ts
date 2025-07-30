export interface Archive {
  readonly id: string
  readonly title: string
  readonly path: string
  readonly pageCount: number
  readonly fileSize: number
  readonly hash: string
  readonly createdAt: string
  readonly updatedAt: string
  readonly tags: Tag[]
}

export interface Tag {
  readonly id: number
  readonly name: string
  readonly namespace: string
}

export interface PaginatedResponse<T> {
  readonly data: T[]
  readonly page: number
  readonly limit: number
  readonly total: number
  readonly hasNext: boolean
}

export interface SearchParams {
  query?: string
  tags?: string[]
  minPages?: number
  maxPages?: number
  minFileSize?: number
  maxFileSize?: number
  createdAfter?: string   // 添加时间过滤 - 创建时间之后
  createdBefore?: string  // 添加时间过滤 - 创建时间之前
  lastReadAfter?: string  // 上次阅读时间过滤 - 阅读时间之后
  lastReadBefore?: string // 上次阅读时间过滤 - 阅读时间之前
  sortBy?: string
  sortOrder?: string
  page?: number
  limit?: number
}

export interface HealthResponse {
  status: string
  version: string
  timestamp: string
}

export interface User {
  readonly id: string
  readonly username: string
  readonly email?: string
  readonly createdAt: string
  readonly updatedAt: string
}

export interface AuthResponse {
  readonly token: string
  readonly user: User
}

export interface LoginRequest {
  username: string
  password: string
}

export interface CreateUserRequest {
  username: string
  email?: string
  password: string
}

export interface ReadingProgress {
  readonly id: number
  readonly archiveId: string
  readonly userId: string
  readonly currentPage: number
  readonly totalPages: number
  readonly progressPercentage: number
  readonly lastReadAt: string
}

export interface UpdateProgressRequest {
  currentPage: number
}

export interface SystemSettings {
  comicsPath: string
  supportedFormats: string[]
  maxFileSize: number
  imageCacheSize: number
  scanOnStartup: boolean
}

// 系统初始化相关
export interface SystemStatus {
  initialized: boolean
  hasAdmin: boolean
  version: string
}

export interface InitializeSystemRequest {
  username: string
  password: string
  email?: string
}

// 分类管理相关
export interface Category {
  readonly id: string
  readonly name: string
  readonly description?: string
  readonly isStatic: boolean
  readonly archiveCount: number
  readonly createdAt: string
  readonly updatedAt: string
}

export interface DynamicCategory {
  readonly id: string
  readonly name: string
  readonly description?: string
  readonly searchParams: string
  readonly createdAt: string
  readonly updatedAt: string
}

export interface CreateCategoryRequest {
  name: string
  description?: string
}

export interface CreateDynamicCategoryRequest {
  name: string
  description?: string
  searchParams: SearchParams
}

export interface UpdateCategoryRequest {
  name?: string
  description?: string
}

export interface AddArchivesToCategoryRequest {
  archiveIds: string[]
}