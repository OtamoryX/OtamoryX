import { useQuery, useMutation, useQueryClient } from '@tanstack/vue-query'
import * as api from '@/utils/api'
import type { SearchParams, CreateCategoryRequest } from '@/types/api'

// 漫画相关查询
export const useArchives = (params?: SearchParams) => {
  return useQuery({
    queryKey: ['archives', params],
    queryFn: () => api.searchArchives(params || {}),
    retry: 2,
    staleTime: 5 * 60 * 1000, // 5分钟
  })
}

export const useArchive = (id: string) => {
  return useQuery({
    queryKey: ['archive', id],
    queryFn: () => api.getArchive(id),
    enabled: !!id,
    retry: 2,
  })
}

export const useSearchArchives = (params: SearchParams) => {
  return useQuery({
    queryKey: ['search', params],
    queryFn: () => api.searchArchives(params),
    retry: 1,
  })
}

// 分类相关查询
export const useCategories = () => {
  return useQuery({
    queryKey: ['categories'],
    queryFn: api.getCategories,
    retry: 2,
    staleTime: 10 * 60 * 1000, // 10分钟
  })
}

export const useCategoryArchives = (categoryId: string, params?: SearchParams) => {
  return useQuery({
    queryKey: ['category-archives', categoryId, params],
    queryFn: () => api.getCategoryArchives(categoryId, params),
    enabled: !!categoryId,
    retry: 2,
  })
}

// 变更操作
export const useCreateCategory = () => {
  const queryClient = useQueryClient()
  
  return useMutation({
    mutationFn: (data: CreateCategoryRequest) => api.createCategory(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['categories'] })
    },
  })
}

export const useDeleteCategory = () => {
  const queryClient = useQueryClient()
  
  return useMutation({
    mutationFn: (id: string) => api.deleteCategory(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['categories'] })
    },
  })
}

// 标签相关
export const useTags = () => {
  return useQuery({
    queryKey: ['tags'],
    queryFn: api.getTags,
    retry: 2,
    staleTime: 15 * 60 * 1000, // 15分钟
  })
}

// 系统状态
export const useSystemStatus = () => {
  return useQuery({
    queryKey: ['system-status'],
    queryFn: api.getSystemStatus,
    retry: 1,
    refetchInterval: 30 * 1000, // 每30秒检查一次
  })
}

// 健康检查
export const useHealth = () => {
  return useQuery({
    queryKey: ['health'],
    queryFn: api.getHealth,
    retry: 1,
    refetchInterval: 60 * 1000, // 每分钟检查一次
  })
}