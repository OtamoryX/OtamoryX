import axios from 'axios'
import type { 
  Archive, 
  PaginatedResponse, 
  SearchParams, 
  HealthResponse,
  SystemSettings,
  Tag,
  ReadingProgress,
  UpdateProgressRequest,
  BatchProgressRequest,
  BatchProgressResponse,
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
  User,
  Plugin,
  AISettings,
  AIStatus,
  DirectoryListResponse
} from '@/types/api'

const api = axios.create({
  baseURL: '/api/v1',
  timeout: 10000,
})

// 请求拦截器 - 添加认证头
api.interceptors.request.use((config) => {
  const apiKey = localStorage.getItem('apiKey')
  if (apiKey) {
    config.headers.Authorization = `Bearer ${apiKey}`
  }
  return config
})

// 响应拦截器 - 处理认证错误
api.interceptors.response.use(
  (response) => response,
  (error) => {
    if (error.response?.status === 401) {
      localStorage.removeItem('apiKey')
      window.location.href = '/login'
    }
    return Promise.reject(error)
  }
)

// 健康检查
export const getHealth = async (): Promise<HealthResponse> => {
  const response = await api.get('/health')
  return response.data
}

// 认证相关
export const login = async (credentials: LoginRequest): Promise<AuthResponse> => {
  const response = await api.post('/auth/login', credentials)
  return response.data
}

export const register = async (userData: CreateUserRequest): Promise<AuthResponse> => {
  const response = await api.post('/auth/register', userData)
  return response.data
}

export const logout = async (): Promise<void> => {
  await api.post('/auth/logout')
}

// 获取单个漫画详情
export const getArchive = async (id: string): Promise<Archive> => {
  const response = await api.get(`/archives/${id}`)
  return response.data
}

// 获取漫画页面图片
export const getArchivePage = async (id: string, page: number): Promise<string> => {
  const response = await api.get(`/archives/${id}/pages/${page}`, {
    responseType: 'blob'
  })
  return URL.createObjectURL(response.data)
}

// 获取漫画缩略图
export const getArchiveThumbnail = async (id: string): Promise<string> => {
  const response = await api.get(`/archives/${id}/thumbnail`, {
    responseType: 'blob'
  })
  return URL.createObjectURL(response.data)
}

// 阅读进度
export const getProgress = async (archiveId: string): Promise<ReadingProgress> => {
  const response = await api.get(`/archives/${archiveId}/progress`)
  return response.data
}

export const updateProgress = async (archiveId: string, progress: UpdateProgressRequest): Promise<void> => {
  await api.post(`/archives/${archiveId}/progress`, progress)
}

// 批量获取多个漫画的阅读进度（返回数组格式）
export const getBatchProgress = async (archiveIds: string[]): Promise<ReadingProgress[]> => {
  if (archiveIds.length === 0) return []
  
  try {
    const request: BatchProgressRequest = { archiveIds }
    const response = await api.post<BatchProgressResponse>('/progress/batch', request)
    
    // 将对象格式的响应转换为数组格式
    const progressArray: ReadingProgress[] = Object.values(response.data.progress)
    
    return progressArray
  } catch (error) {
    console.error('Failed to get batch progress:', error)
    return []
  }
}

// 批量获取多个漫画的阅读进度（返回映射格式）
export const getBatchProgressMap = async (archiveIds: string[]): Promise<Record<string, ReadingProgress>> => {
  if (archiveIds.length === 0) return {}
  
  try {
    const request: BatchProgressRequest = { archiveIds }
    const response = await api.post<BatchProgressResponse>('/progress/batch', request)
    
    return response.data.progress
  } catch (error) {
    console.error('Failed to get batch progress map:', error)
    return {}
  }
}

// 搜索漫画
export const searchArchives = async (params: SearchParams): Promise<PaginatedResponse<Archive>> => {
  const response = await api.get('/search', { params })
  return response.data
}

// 获取标签
export const getTags = async (): Promise<Tag[]> => {
  const response = await api.get('/tags')
  return response.data
}

// 系统设置
export const getSettings = async (): Promise<SystemSettings> => {
  const response = await api.get('/settings')
  return response.data
}

export const updateSettings = async (settings: SystemSettings): Promise<void> => {
  await api.put('/settings', settings)
}

// 扫描设置
export const getScanSettings = async (): Promise<any> => {
  const response = await api.get('/settings/scan-settings')
  return response.data
}

export const updateScanSettings = async (scanSettings: any): Promise<any> => {
  const response = await api.post('/settings/scan-settings', { scanSettings })
  return response.data
}

// 手动触发扫描
export const triggerScan = async (): Promise<any> => {
  const response = await api.post('/settings/scan')
  return response.data
}

// 系统初始化
export const getSystemStatus = async (): Promise<SystemStatus> => {
  const response = await api.get('/system/status')
  return response.data
}

export const initializeSystem = async (data: InitializeSystemRequest): Promise<AuthResponse> => {
  const response = await api.post('/system/initialize', data)
  return response.data
}

// 分类管理
export const getCategories = async (): Promise<Category[]> => {
  const response = await api.get('/categories')
  return response.data
}

export const createCategory = async (data: CreateCategoryRequest): Promise<Category> => {
  const response = await api.post('/categories', data)
  return response.data
}

export const createDynamicCategory = async (data: CreateDynamicCategoryRequest): Promise<DynamicCategory> => {
  const response = await api.post('/categories/dynamic', data)
  return response.data
}

export const updateCategory = async (id: string, data: UpdateCategoryRequest): Promise<void> => {
  await api.put(`/categories/${id}`, data)
}

export const deleteCategory = async (id: string): Promise<void> => {
  await api.delete(`/categories/${id}`)
}

export const getCategoryArchives = async (categoryId: string, params?: SearchParams): Promise<PaginatedResponse<Archive>> => {
  const response = await api.get(`/categories/${categoryId}/archives`, { 
    params: {
      pageNumb: params?.pageNumb || 1,
      pageSize: params?.pageSize || 20,
      ...params
    }
  })
  return response.data
}

export const addArchivesToCategory = async (categoryId: string, data: AddArchivesToCategoryRequest): Promise<void> => {
  await api.post(`/categories/${categoryId}/archives`, data)
}

export const removeArchivesFromCategory = async (categoryId: string, data: AddArchivesToCategoryRequest): Promise<void> => {
  await api.delete(`/categories/${categoryId}/archives`, { data })
}

// 用户管理
export const getUsers = async (): Promise<User[]> => {
  const response = await api.get('/users')
  return response.data
}

export const createUser = async (data: CreateUserRequest): Promise<User> => {
  const response = await api.post('/users', data)
  return response.data
}

export const getUser = async (id: string): Promise<User> => {
  const response = await api.get(`/users/${id}`)
  return response.data
}

export const updateUser = async (id: string, data: Partial<CreateUserRequest>): Promise<void> => {
  await api.put(`/users/${id}`, data)
}

export const deleteUser = async (id: string): Promise<void> => {
  await api.delete(`/users/${id}`)
}

// 批量操作
export const batchDeleteArchives = async (archiveIds: string[]): Promise<void> => {
  await api.delete('/archives/batch-delete', { data: { archiveIds } })
}

export const batchDeleteTagArchives = async (tagId: string): Promise<void> => {
  await api.delete(`/tags/${tagId}/archives/batch-delete`)
}

export const batchDeleteCategoryArchives = async (categoryId: string): Promise<void> => {
  await api.delete(`/categories/${categoryId}/archives/batch-delete`)
}

export const pruneTags = async (): Promise<void> => {
  await api.delete('/tags/prune')
}

export const pruneCategories = async (): Promise<void> => {
  await api.delete('/categories/prune')
}

// 缓存管理
export const getCacheStatus = async (): Promise<any> => {
  const response = await api.get('/cache/status')
  return response.data
}

export const configureCache = async (config: { strategy?: string; custom_config?: any }): Promise<void> => {
  await api.post('/cache/configure', config)
}

export const clearCache = async (): Promise<void> => {
  await api.delete('/cache/clear')
}

// 随机漫画
export const getRandomArchives = async (count = 20): Promise<Archive[]> => {
  const response = await api.get('/archives/random', { params: { count } })
  return response.data
}

// 插件管理
export const getPlugins = async (): Promise<Plugin[]> => {
  const response = await api.get('/plugins')
  return response.data
}

export const installPlugin = async (pluginData: FormData): Promise<Plugin> => {
  const response = await api.post('/plugins/install', pluginData, {
    headers: { 'Content-Type': 'multipart/form-data' }
  })
  return response.data
}

export const togglePlugin = async (id: string): Promise<void> => {
  await api.put(`/plugins/${id}/toggle`)
}

export const configurePlugin = async (id: string, config: any): Promise<void> => {
  await api.put(`/plugins/${id}/config`, config)
}

export const uninstallPlugin = async (id: string): Promise<void> => {
  await api.delete(`/plugins/${id}`)
}

// AI自动标签
export const getAISettings = async (): Promise<AISettings> => {
  const response = await api.get('/settings/ai')
  return response.data
}

export const updateAISettings = async (settings: AISettings): Promise<void> => {
  await api.put('/settings/ai', settings)
}

export const getAIStatus = async (): Promise<AIStatus> => {
  const response = await api.get('/ai/status')
  return response.data
}

export const controlAI = async (action: 'pause' | 'resume'): Promise<void> => {
  await api.put('/ai/control', { action })
}

// 标签管理
export const createTag = async (name: string, namespace: string): Promise<Tag> => {
  const response = await api.post('/tags', { name, namespace })
  return response.data
}

export const deleteTag = async (id: string): Promise<void> => {
  await api.delete(`/tags/${id}`)
}

export const addTagToArchive = async (archiveId: string, tagId: string): Promise<void> => {
  await api.post(`/archives/${archiveId}/tags`, { tagId })
}

export const removeTagFromArchive = async (archiveId: string, tagId: string): Promise<void> => {
  await api.delete(`/archives/${archiveId}/tags/${tagId}`)
}

export const deleteArchive = async (archiveId: string): Promise<void> => {
  await api.delete(`/archives/${archiveId}`)
}

// 文件系统浏览
export const getDirectories = async (path?: string): Promise<DirectoryListResponse> => {
  const params = path ? { path } : {}
  const response = await api.get('/filesystem/directories', { params })
  return response.data
}

