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
  AddArchivesToCategoryRequest
} from '@/types/api'

const api = axios.create({
  baseURL: '/api/v1',
  timeout: 10000,
})

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

// 获取漫画列表
export const getArchives = async (params?: SearchParams): Promise<PaginatedResponse<Archive>> => {
  const response = await api.get('/archives', { params })
  return response.data
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

// 阅读进度
export const getProgress = async (archiveId: string): Promise<ReadingProgress> => {
  const response = await api.get(`/archives/${archiveId}/progress`)
  return response.data
}

export const updateProgress = async (archiveId: string, progress: UpdateProgressRequest): Promise<void> => {
  await api.post(`/archives/${archiveId}/progress`, progress)
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
  const response = await api.get(`/categories/${categoryId}/archives`, { params })
  return response.data
}

export const addArchivesToCategory = async (categoryId: string, data: AddArchivesToCategoryRequest): Promise<void> => {
  await api.post(`/categories/${categoryId}/archives`, data)
}

export const removeArchivesFromCategory = async (categoryId: string, data: AddArchivesToCategoryRequest): Promise<void> => {
  await api.delete(`/categories/${categoryId}/archives`, { data })
}