import type { SearchParams } from '@/types/api'

export interface ParsedSearch {
  query: string
  tags: string[]
}

/**
 * 解析搜索字符串
 * 支持格式：
 * - 纯文本：`some title`
 * - 标签：`artist:john`
 * - 逗号分隔：`artist:john, genre:action`
 * - 混合：`some title, artist:john`
 */
export function parseSearchQuery(input: string): ParsedSearch {
  const trimmed = input.trim()
  if (!trimmed) {
    return { query: '', tags: [] }
  }

  const parts = trimmed.split(',').map(s => s.trim()).filter(Boolean)
  const tags: string[] = []
  const queryParts: string[] = []

  for (const part of parts) {
    // 包含冒号且冒号前后都有内容的视为标签
    const colonIndex = part.indexOf(':')
    if (colonIndex > 0 && colonIndex < part.length - 1) {
      tags.push(part)
    } else {
      queryParts.push(part)
    }
  }

  return {
    query: queryParts.join(', '),
    tags,
  }
}

/**
 * 将 ParsedSearch 转换为 API SearchParams
 */
export function toSearchParams(parsed: ParsedSearch, page: number, pageSize: number): SearchParams {
  const params: SearchParams = {
    pageNumb: page,
    pageSize,
  }

  if (parsed.query) {
    params.query = parsed.query
  }

  if (parsed.tags.length > 0) {
    params.tags = parsed.tags
  }

  return params
}
